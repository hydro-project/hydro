use std::{
    collections::HashMap,
    process::{Child, Command},
    sync::{Arc, RwLock},
};

#[cfg(unix)]
use std::os::unix::process::CommandExt;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;
use tokio::signal;

pub static TERRAFORM_ALPHABET: [char; 16] = [
    '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', 'a', 'b', 'c', 'd', 'e', 'f',
];

/// Keeps track of resources which may need to be cleaned up.
#[derive(Default)]
pub struct TerraformPool {
    counter: u32,
    active_applies: HashMap<u32, Arc<tokio::sync::RwLock<TerraformApply>>>,
}

impl TerraformPool {
    fn create_apply(
        &mut self,
        deployment_folder: TempDir,
    ) -> Result<(u32, Arc<tokio::sync::RwLock<TerraformApply>>)> {
        let next_counter = self.counter;
        self.counter += 1;

        let mut apply_command = Command::new("terraform");

        apply_command
            .current_dir(deployment_folder.path())
            .arg("apply")
            .arg("-auto-approve");

        #[cfg(unix)]
        {
            apply_command.process_group(0);
        }

        let spawned_child = apply_command
            .spawn()
            .context("Failed to spawn `terraform`. Is it installed?")?;

        let spawned_id = spawned_child.id();

        let deployment = Arc::new(tokio::sync::RwLock::new(TerraformApply {
            child: Some((spawned_id, Arc::new(RwLock::new(spawned_child)))),
            deployment_folder: Some(deployment_folder),
        }));

        self.active_applies.insert(next_counter, deployment.clone());

        Ok((next_counter, deployment))
    }

    fn drop_apply(&mut self, counter: u32) {
        self.active_applies.remove(&counter);
    }
}

impl Drop for TerraformPool {
    fn drop(&mut self) {
        for (_, apply) in self.active_applies.drain() {
            debug_assert_eq!(Arc::strong_count(&apply), 1);
            // make sure we can write, because that means we own this apply
            let _ = apply.blocking_write();
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TerraformBatch {
    pub terraform: TerraformConfig,
    pub resource: HashMap<String, HashMap<String, serde_json::Value>>,
    pub output: HashMap<String, TerraformOutput>,
}

impl Default for TerraformBatch {
    fn default() -> TerraformBatch {
        TerraformBatch {
            terraform: TerraformConfig {
                required_providers: HashMap::new(),
            },
            resource: HashMap::new(),
            output: HashMap::new(),
        }
    }
}

impl TerraformBatch {
    pub async fn provision(self, pool: &mut TerraformPool) -> Result<TerraformResult> {
        let dothydro_folder = std::env::current_dir().unwrap().join(".hydro");
        std::fs::create_dir_all(&dothydro_folder).unwrap();
        let deployment_folder = tempfile::tempdir_in(dothydro_folder).unwrap();

        if self.terraform.required_providers.is_empty()
            && self.resource.is_empty()
            && self.output.is_empty()
        {
            return Ok(TerraformResult {
                outputs: HashMap::new(),
                deployment_folder: None,
            });
        }

        std::fs::write(
            deployment_folder.path().join("main.tf.json"),
            serde_json::to_string(&self).unwrap(),
        )
        .unwrap();

        if !Command::new("terraform")
            .current_dir(deployment_folder.path())
            .arg("init")
            .spawn()
            .context("Failed to spawn `terraform`. Is it installed?")?
            .wait()
            .context("Failed to launch terraform init command")?
            .success()
        {
            bail!("Failed to initialize terraform");
        }

        let (apply_id, apply) = pool.create_apply(deployment_folder)?;

        let output = apply.write().await.output().await;
        pool.drop_apply(apply_id);
        output
    }
}

struct TerraformApply {
    child: Option<(u32, Arc<RwLock<Child>>)>,
    deployment_folder: Option<tempfile::TempDir>,
}

impl TerraformApply {
    async fn output(&mut self) -> Result<TerraformResult> {
        let (_, child) = self.child.as_ref().unwrap().clone();

        let wait_channel = tokio::task::spawn_blocking(move || {
            // it is okay for this thread to keep running even if the future is cancelled
            child.write().unwrap().wait().unwrap()
        });

        tokio::select! {
            _ = signal::ctrl_c() => {
                bail!("Received Ctrl-C")
            },
            status = wait_channel => {
                self.child = None;

                if !status.unwrap().success() {
                    bail!("Terraform deployment failed");
                }

                let mut output_command = Command::new("terraform");
                output_command
                    .current_dir(self.deployment_folder.as_ref().unwrap().path())
                    .arg("output")
                    .arg("-json");

                #[cfg(unix)]
                {
                    output_command.process_group(0);
                }

                let output = output_command.output()
                    .context("Failed to read Terraform outputs")?;

                Ok(TerraformResult {
                    outputs: serde_json::from_slice(&output.stdout).unwrap(),
                    deployment_folder: self.deployment_folder.take(),
                })
            },
        }
    }
}

impl Drop for TerraformApply {
    fn drop(&mut self) {
        if let Some((pid, child)) = self.child.take() {
            #[cfg(unix)]
            nix::sys::signal::kill(
                nix::unistd::Pid::from_raw(pid as i32),
                nix::sys::signal::Signal::SIGTERM,
            )
            .unwrap();

            let mut child_write = child.write().unwrap();
            if child_write.try_wait().unwrap().is_none() {
                println!("Waiting for Terraform apply to finish...");
                child_write.wait().unwrap();
            }
        }

        if let Some(deployment_folder) = &self.deployment_folder {
            println!(
                "Destroying terraform deployment at {}",
                deployment_folder.path().display()
            );

            if !Command::new("terraform")
                .current_dir(deployment_folder)
                .arg("destroy")
                .arg("-auto-approve")
                .spawn()
                .expect("Failed to spawn terraform destroy command")
                .wait()
                .expect("Failed to destroy terraform deployment")
                .success()
            {
                eprintln!("WARNING: failed to destroy terraform deployment");
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TerraformConfig {
    pub required_providers: HashMap<String, TerraformProvider>,
}

#[derive(Serialize, Deserialize)]
pub struct TerraformProvider {
    pub source: String,
    pub version: String,
}

#[derive(Serialize, Deserialize)]
pub struct TerraformOutput {
    pub value: String,
}

pub struct TerraformResult {
    pub outputs: HashMap<String, TerraformOutput>,
    /// `None` if no deployment was performed
    pub deployment_folder: Option<tempfile::TempDir>,
}

impl Drop for TerraformResult {
    fn drop(&mut self) {
        if let Some(folder) = &self.deployment_folder {
            println!(
                "Destroying terraform deployment at {}",
                folder.path().display()
            );
            if !Command::new("terraform")
                .current_dir(folder)
                .arg("destroy")
                .arg("-auto-approve")
                .spawn()
                .expect("Failed to spawn terraform destroy command")
                .wait()
                .expect("Failed to destroy terraform deployment")
                .success()
            {
                eprintln!("WARNING: failed to destroy terraform deployment");
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct TerraformResultOutput {
    value: String,
}
