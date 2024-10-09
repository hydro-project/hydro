---
sidebar_position: 4
---

# Deployments, Hosts, and Services
Hydro Deploy uses an architecture consisting of three key elements:
- Deployments: a set of active resources and services
- Hosts: targets that can run services, including your laptop and cloud VMs
- Services: networked processes, including Hydroflow projects and external services

## Deployments
Deployments are managed by the `hydro.Deployment` class, which will keep all its tracked resources alive as long as it is being held in memory. This means that you can create a deployment, run some code, and then drop the deployment to clean up all the resources.

To create a deployment, simply call `hydro.Deployment()`:
```python
import hydro

async def main(args):
    deployment = hydro.Deployment()
```

## Hosts
Hosts are targets that can run services. By default, every deployment has `Localhost` available, which is a special host that runs services on the same machine as the deployment. We can access this host by calling `deployment.Localhost()`:
```python
localhost = deployment.Localhost()
```

Hydro Deploy also supports deploying to cloud VMs, currently supporting Google Cloud Platform (with support planned for Azure and AWS).

### Google Cloud Platform
To deploy to Google Cloud Platform, you will need to install Terraform and the Google Cloud SDK (see [install](./install)). You will also need to create a Google Cloud project.

The first step is to create a VPC, which will enable network connections for our services. We can do this by creating a `hydro.GcpNetwork` object:
```python
network = deployment.GcpNetwork(
    project="my-project"
)
```

Then, we can launch a VM on this network using `hydro.GcpComputeEngineHost`:
```python
host = deployment.GcpComputeEngineHost(
    name="my-host",
    network=network,
    machine_type="e2-micro",
    region="us-west1-a",
    image="debian-cloud/debian-11"
)
```

## Services
Services are networked processes, including Hydroflow projects and external services.

### Hydroflow Projects
To create a service based on a Hydroflow project, we can use the `hydro.HydroflowCrate` class:
```python
service = deployment.HydroflowCrate(
    src=".", # path to the Rust project
    on=host
)
```
