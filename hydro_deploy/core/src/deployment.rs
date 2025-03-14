#![allow(
    unexpected_cfgs,
    reason = "https://github.com/BrynCooke/buildstructor/issues/192"
)]

use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Weak};

use anyhow::Result;
use futures::{FutureExt, StreamExt, TryStreamExt};
use tokio::sync::RwLock;

use super::gcp::GcpNetwork;
use super::{
    CustomService, GcpComputeEngineHost, Host, LocalhostHost, ResourcePool, ResourceResult,
    Service, progress,
};
use crate::{AzureHost, ServiceBuilder};

pub struct Deployment {
    pub hosts: Vec<Weak<dyn Host>>,
    pub services: Vec<Weak<RwLock<dyn Service>>>,
    pub resource_pool: ResourcePool,
    localhost_host: Option<Arc<LocalhostHost>>,
    last_resource_result: Option<Arc<ResourceResult>>,
    next_host_id: usize,
    next_service_id: usize,
}

impl Default for Deployment {
    fn default() -> Self {
        Self::new()
    }
}

impl Deployment {
    pub fn new() -> Self {
        let mut ret = Self {
            hosts: Vec::new(),
            services: Vec::new(),
            resource_pool: ResourcePool::default(),
            localhost_host: None,
            last_resource_result: None,
            next_host_id: 0,
            next_service_id: 0,
        };

        ret.localhost_host = Some(ret.add_host(LocalhostHost::new));
        ret
    }

    #[expect(non_snake_case, reason = "constructor-esque")]
    pub fn Localhost(&self) -> Arc<LocalhostHost> {
        self.localhost_host.clone().unwrap()
    }

    #[expect(non_snake_case, reason = "constructor-esque")]
    pub fn CustomService(
        &mut self,
        on: Arc<dyn Host>,
        external_ports: Vec<u16>,
    ) -> Arc<RwLock<CustomService>> {
        self.add_service(|id| CustomService::new(id, on, external_ports))
    }

    /// Runs `deploy()`, and `start()`, waits for the trigger future, then runs `stop()`.
    pub async fn run_until(&mut self, trigger: impl Future<Output = ()>) -> Result<()> {
        // TODO(mingwei): should `trigger` interrupt `deploy()` and `start()`? If so make sure shutdown works as expected.
        self.deploy().await?;
        self.start().await?;
        trigger.await;
        self.stop().await?;
        Ok(())
    }

    /// Runs `start()`, waits for the trigger future, then runs `stop()`.
    /// This is useful if you need to initiate external network connections between
    /// `deploy()` and `start()`.
    pub async fn start_until(&mut self, trigger: impl Future<Output = ()>) -> Result<()> {
        // TODO(mingwei): should `trigger` interrupt `deploy()` and `start()`? If so make sure shutdown works as expected.
        self.start().await?;
        trigger.await;
        self.stop().await?;
        Ok(())
    }

    /// Runs `deploy()`, and `start()`, waits for CTRL+C, then runs `stop()`.
    pub async fn run_ctrl_c(&mut self) -> Result<()> {
        self.run_until(tokio::signal::ctrl_c().map(|_| ())).await
    }

    /// Runs `start()`, waits for CTRL+C, then runs `stop()`.
    /// This is useful if you need to initiate external network connections between
    /// `deploy()` and `start()`.
    pub async fn start_ctrl_c(&mut self) -> Result<()> {
        self.start_until(tokio::signal::ctrl_c().map(|_| ())).await
    }

    pub async fn deploy(&mut self) -> Result<()> {
        self.services.retain(|weak| weak.strong_count() > 0);

        progress::ProgressTracker::with_group("deploy", Some(3), || async {
            let mut resource_batch = super::ResourceBatch::new();

            for service in self.services.iter().filter_map(Weak::upgrade) {
                service.read().await.collect_resources(&mut resource_batch);
            }

            for host in self.hosts.iter().filter_map(Weak::upgrade) {
                host.collect_resources(&mut resource_batch);
            }

            let resource_result = Arc::new(
                progress::ProgressTracker::with_group("provision", Some(1), || async {
                    resource_batch
                        .provision(&mut self.resource_pool, self.last_resource_result.clone())
                        .await
                })
                .await?,
            );
            self.last_resource_result = Some(resource_result.clone());

            for host in self.hosts.iter().filter_map(Weak::upgrade) {
                host.provision(&resource_result);
            }

            let upgraded_services = self
                .services
                .iter()
                .filter_map(Weak::upgrade)
                .collect::<Vec<_>>();

            progress::ProgressTracker::with_group("prepare", Some(upgraded_services.len()), || {
                let services_future = upgraded_services
                    .iter()
                    .map(|service: &Arc<RwLock<dyn Service>>| {
                        let resource_result = &resource_result;
                        async move { service.write().await.deploy(resource_result).await }
                    })
                    .collect::<Vec<_>>();

                futures::stream::iter(services_future)
                    .buffer_unordered(16)
                    .try_fold((), |_, _| async { Ok(()) })
            })
            .await?;

            progress::ProgressTracker::with_group("ready", Some(upgraded_services.len()), || {
                let all_services_ready =
                    upgraded_services
                        .iter()
                        .map(|service: &Arc<RwLock<dyn Service>>| async move {
                            service.write().await.ready().await?;
                            Ok(()) as Result<()>
                        });

                futures::future::try_join_all(all_services_ready)
            })
            .await?;

            Ok(())
        })
        .await
    }

    pub async fn start(&mut self) -> Result<()> {
        self.services.retain(|weak| weak.strong_count() > 0);

        progress::ProgressTracker::with_group("start", None, || {
            let all_services_start = self.services.iter().filter_map(Weak::upgrade).map(
                |service: Arc<RwLock<dyn Service>>| async move {
                    service.write().await.start().await?;
                    Ok(()) as Result<()>
                },
            );

            futures::future::try_join_all(all_services_start)
        })
        .await?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        self.services.retain(|weak| weak.strong_count() > 0);

        progress::ProgressTracker::with_group("stop", None, || {
            let all_services_stop = self.services.iter().filter_map(Weak::upgrade).map(
                |service: Arc<RwLock<dyn Service>>| async move {
                    service.write().await.stop().await?;
                    Ok(()) as Result<()>
                },
            );

            futures::future::try_join_all(all_services_stop)
        })
        .await?;
        Ok(())
    }
}

impl Deployment {
    pub fn add_host<T: Host + 'static, F: FnOnce(usize) -> T>(&mut self, host: F) -> Arc<T> {
        let arc = Arc::new(host(self.next_host_id));
        self.next_host_id += 1;

        self.hosts.push(Arc::downgrade(&arc) as Weak<dyn Host>);
        arc
    }

    pub fn add_service<T: Service + 'static>(
        &mut self,
        service: impl ServiceBuilder<Service = T>,
    ) -> Arc<RwLock<T>> {
        let arc = Arc::new(RwLock::new(service.build(self.next_service_id)));
        self.next_service_id += 1;

        self.services
            .push(Arc::downgrade(&arc) as Weak<RwLock<dyn Service>>);
        arc
    }
}

/// Buildstructor methods.
#[buildstructor::buildstructor]
impl Deployment {
    #[builder(entry = "GcpComputeEngineHost", exit = "add")]
    pub fn add_gcp_compute_engine_host(
        &mut self,
        project: String,
        machine_type: String,
        image: String,
        region: String,
        network: Arc<RwLock<GcpNetwork>>,
        user: Option<String>,
        startup_script: Option<String>,
    ) -> Arc<GcpComputeEngineHost> {
        self.add_host(|id| {
            GcpComputeEngineHost::new(
                id,
                project,
                machine_type,
                image,
                region,
                network,
                user,
                startup_script,
            )
        })
    }

    #[builder(entry = "AzureHost", exit = "add")]
    pub fn add_azure_host(
        &mut self,
        project: String,
        os_type: String, // linux or windows
        machine_size: String,
        image: Option<HashMap<String, String>>,
        region: String,
        user: Option<String>,
    ) -> Arc<AzureHost> {
        self.add_host(|id| AzureHost::new(id, project, os_type, machine_size, image, region, user))
    }
}
