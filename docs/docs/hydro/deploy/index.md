<h1 class="crate-title">Hydro Deploy</h1>

<<<<<<<< HEAD:hydro_deploy/core/README.md
**Hydro Deploy** is Hydro's deployment system, allowing you to deploy your app to a variety of platforms. With Hydro Deploy, you can spin up complex services with just a few lines of Rust!
========
# Hydro Deploy

:::caution

Hydro Deploy is currently in alpha, and documentation is still under construction. Please reach out if you run into any issues--it is under active development and we continue to use it for large-scale distributed experiments within the Hydro research group.

:::

Hydro comes equipped with a built-in deployment system, **Hydro Deploy**, which allows you to deploy your Hydro app to a variety of platforms. With Hydro Deploy, you can spin up complex services with just a few lines of Rust! This guide will walk you through the process of deploying your Hydro app in a variety of scenarios.
>>>>>>>> 5fe776fe91e (docs: demote python deploy docs, fix docsrs configs):docs/docs/hydro/deploy/index.md

Hydro Deploy focuses on managing the end-to-end lifecycle of networked services in the cloud. It is not a general-purpose deployment tool, and is not intended to replace systems like Docker Compose or Kubernetes. Instead, Hydro Deploy is designed to be used in conjunction with these tools to manage the lifecycle of your Hydro app.

Currently, Hydro Deploy is focused on _ephemeral applications_, which can be spun up from your laptop and automatically clean up resources on shutdown. Hydro Deploy focuses on automating the core tasks of deploying an app:
- Provisioning virtual machines and network resources on a cloud provider
- Configuring security groups and firewalls
- Building and deploying your Hydroflow services
- Initializing network connections based on a user-defined topology
- Monitoring logs from your services
<<<<<<<< HEAD:hydro_deploy/core/README.md

Hydro Deploy currently supports the following hosts:
- Localhost
- GCP
- Azure
========
>>>>>>>> 5fe776fe91e (docs: demote python deploy docs, fix docsrs configs):docs/docs/hydro/deploy/index.md
