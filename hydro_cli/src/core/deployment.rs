use std::sync::{Arc, Weak};

use anyhow::Result;
use tokio::sync::RwLock;

use super::{progress, Host, ResourcePool, ResourceResult, Service};

#[derive(Default)]
pub struct Deployment {
    pub hosts: Vec<Arc<RwLock<dyn Host>>>,
    pub services: Vec<Weak<RwLock<dyn Service>>>,
    pub resource_pool: ResourcePool,
    last_resource_result: Option<Arc<ResourceResult>>,
    next_host_id: usize,
    next_service_id: usize,
}

impl Deployment {
    pub async fn deploy(&mut self) -> Result<()> {
        progress::ProgressTracker::with_group("deploy", || async {
            let mut resource_batch = super::ResourceBatch::new();
            let active_services = self
                .services
                .iter()
                .filter(|service| service.upgrade().is_some())
                .cloned()
                .collect::<Vec<_>>();
            self.services = active_services;

            for service in self.services.iter_mut() {
                service
                    .upgrade()
                    .unwrap()
                    .write()
                    .await
                    .collect_resources(&mut resource_batch);
            }

            for host in self.hosts.iter_mut() {
                host.write().await.collect_resources(&mut resource_batch);
            }

            let result = Arc::new(
                progress::ProgressTracker::with_group("provision", || async {
                    resource_batch
                        .provision(&mut self.resource_pool, self.last_resource_result.clone())
                        .await
                })
                .await?,
            );
            self.last_resource_result = Some(result.clone());

            progress::ProgressTracker::with_group("provision", || {
                let hosts_provisioned =
                    self.hosts
                        .iter_mut()
                        .map(|host: &mut Arc<RwLock<dyn Host>>| async {
                            host.write().await.provision(&result).await;
                        });
                futures::future::join_all(hosts_provisioned)
            })
            .await;

            progress::ProgressTracker::with_group("deploy", || {
                let services_future =
                    self.services
                        .iter_mut()
                        .map(|service: &mut Weak<RwLock<dyn Service>>| async {
                            service
                                .upgrade()
                                .unwrap()
                                .write()
                                .await
                                .deploy(&result)
                                .await;
                        });

                futures::future::join_all(services_future)
            })
            .await;

            progress::ProgressTracker::with_group("ready", || {
                let all_services_ready =
                    self.services
                        .iter()
                        .map(|service: &Weak<RwLock<dyn Service>>| async {
                            service.upgrade().unwrap().write().await.ready().await?;
                            Ok(()) as Result<()>
                        });

                futures::future::try_join_all(all_services_ready)
            })
            .await?;

            Ok(())
        })
        .await
    }

    pub async fn start(&mut self) {
        let active_services = self
            .services
            .iter()
            .filter(|service| service.upgrade().is_some())
            .cloned()
            .collect::<Vec<_>>();
        self.services = active_services;

        let all_services_start =
            self.services
                .iter()
                .map(|service: &Weak<RwLock<dyn Service>>| async {
                    service.upgrade().unwrap().write().await.start().await;
                });

        futures::future::join_all(all_services_start).await;
    }

    pub fn add_host<T: Host + 'static, F: FnOnce(usize) -> T>(
        &mut self,
        host: F,
    ) -> Arc<RwLock<T>> {
        let arc = Arc::new(RwLock::new(host(self.next_host_id)));
        self.next_host_id += 1;

        self.hosts.push(arc.clone());
        arc
    }

    pub fn add_service<T: Service + 'static>(
        &mut self,
        service: impl FnOnce(usize) -> T,
    ) -> Arc<RwLock<T>> {
        let arc = Arc::new(RwLock::new(service(self.next_service_id)));
        self.next_service_id += 1;

        let dyn_arc: Arc<RwLock<dyn Service>> = arc.clone();
        self.services.push(Arc::downgrade(&dyn_arc));
        arc
    }
}
