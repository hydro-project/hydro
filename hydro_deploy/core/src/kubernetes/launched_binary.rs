#[cfg(unix)]
use std::sync::Arc;
use std::sync::Mutex;

use anyhow::Error;
use async_trait::async_trait;
use futures::TryStreamExt;
use k8s_openapi::api::core::v1::Pod;
use kube::api::{Api, AttachedProcess, DeleteParams};
use kube::Client;
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot};

use crate::util::prioritized_broadcast;
use crate::LaunchedBinary;

pub struct LaunchedPodBinary {
    stdin_sender: mpsc::UnboundedSender<String>,
    stdout_deploy_receivers: Arc<Mutex<Option<oneshot::Sender<String>>>>,
    stdout_receivers: Arc<Mutex<Vec<mpsc::UnboundedSender<String>>>>,
    stderr_receivers: Arc<Mutex<Vec<mpsc::UnboundedSender<String>>>>,
    attached_process: Mutex<AttachedProcess>,
    pod_name: String,
}

impl LaunchedPodBinary {
    pub fn new(mut launched_pod_binary: AttachedProcess, id: String, pod_name: String) -> Self {
        // Create streams for stdout and stdin for the running binary in the pod
        let launch_binary_out =
            tokio_util::io::ReaderStream::new(launched_pod_binary.stdout().unwrap());
        let launch_binary_err =
            tokio_util::io::ReaderStream::new(launched_pod_binary.stderr().unwrap());
        let mut stdin = launched_pod_binary.stdin().unwrap();

        let (stdin_sender, mut stdin_receiver) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            while let Some(line) = stdin_receiver.recv().await {
                if stdin.write_all(line.as_bytes()).await.is_err() {
                    break;
                }

                stdin.flush().await.ok();
            }
        });

        let id_clone = id.clone();
        let (stdout_deploy_receivers, stdout_receivers) = prioritized_broadcast(
            launch_binary_out.map_ok(|bytes| String::from_utf8_lossy(&bytes).to_string()),
            move |s| println!("[{id_clone}] {s}"),
        );
        let (_, stderr_receivers) = prioritized_broadcast(
            launch_binary_err.map_ok(|bytes| String::from_utf8_lossy(&bytes).to_string()),
            move |s| eprintln!("[{id}] {s}"),
        );

        Self {
            stdin_sender,
            stdout_deploy_receivers,
            stdout_receivers,
            stderr_receivers,
            attached_process: Mutex::new(launched_pod_binary),
            pod_name,
        }
    }
}

#[async_trait]
impl LaunchedBinary for LaunchedPodBinary {
    fn stdin(&self) -> mpsc::UnboundedSender<String> {
        self.stdin_sender.clone()
    }

    fn deploy_stdout(&self) -> oneshot::Receiver<String> {
        let mut receivers = self.stdout_deploy_receivers.lock().unwrap();

        if receivers.is_some() {
            panic!("Only one deploy stdout receiver is allowed at a time");
        }

        let (sender, receiver) = oneshot::channel::<String>();
        *receivers = Some(sender);
        receiver
    }

    fn stdout(&self) -> mpsc::UnboundedReceiver<String> {
        let mut receivers = self.stdout_receivers.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        receivers.push(sender);
        receiver
    }

    fn stderr(&self) -> mpsc::UnboundedReceiver<String> {
        let mut receivers = self.stderr_receivers.lock().unwrap();
        let (sender, receiver) = mpsc::unbounded_channel::<String>();
        receivers.push(sender);
        receiver
    }

    // returns exit code when the hydroflow program finishes
    fn exit_code(&self) -> Option<i32> {
        Some(1)
    }

    // waits for the hydroflow program to finish
    async fn wait(&mut self) -> Result<i32, Error> {
        let status = self
            .attached_process
            .get_mut()
            .unwrap()
            .take_status()
            .unwrap()
            .await;
        match status {
            Some(_) => return Ok(1),
            None => return Ok(0),
        }
    }

    // waits for the hydroflow program to finish
    async fn stop(&mut self) -> Result<(), Error> {
        // Implement the logic to stop the hydroflow program
        // For now, we will return Ok(()) to indicate success
        let client = Client::try_default().await?;

        // Manage pods
        let pods: Api<Pod> = Api::default_namespaced(client);
        let dp = DeleteParams::default();
        match pods.delete(self.pod_name.as_str(), &dp).await {
            Ok(_) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
