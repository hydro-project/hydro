---
sidebar_position: 1
---

import Tabs from '@theme/Tabs';
import TabItem from '@theme/TabItem';

# Installing Hydro Deploy
Hydro Deploy has two key components: a Python library used to define your Hydro app and deploy it, and an optional CLI that provides a user-friendly entrypoint.

To install Hydro Deploy, you can use pip:

```bash
#shell-command-next-line
pip install hydro-deploy
```

If you intend to deploy to cloud platforms (currently only Google Cloud is supported), you will need to install Terraform and the Google Cloud SDK:

<Tabs groupId="operating-systems">
<TabItem value="mac" label="macOS">

```bash
#shell-command-next-line
brew install terraform

#shell-command-next-line
brew install google-cloud-sdk

#shell-command-next-line
gcloud auth application-default login
```

</TabItem>
<TabItem value="win" label="Windows">

```bash
#shell-command-next-line
choco install terraform

#shell-command-next-line
choco install gcloudsdk

#shell-command-next-line
gcloud auth application-default login
```

</TabItem>
<TabItem value="linux" label="Linux">

Follow the [Terraform instructions](https://developer.hashicorp.com/terraform/tutorials/gcp-get-started/install-cli) to install Terraform. Then follow the [Google Cloud](https://cloud.google.com/sdk/docs/install#linux) instructions to install the Google Cloud SDK. Finally, authenticate with Google Cloud:

```bash
#shell-command-next-line
gcloud auth application-default login
```

</TabItem>
</Tabs>

## Verify Installation
To check that Hydro Deploy is installed correctly, you can run the following command:

```console
#shell-command-next-line
hydro --version
hydro_cli 0.0.0
```
