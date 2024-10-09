## Getting Started
This is a template for a Rust project that uses [Hydroflow+](http://github.com/hydro-project/hydroflow) for distributed services. To generate a project, run 

```bash
cargo install cargo-generate
cargo generate gh:hydro-project/hydroflow template/hydroflow_plus
cd <myproject>
```

After `cd`ing into the workspace, ensure the correct nightly version of rust is installed:
```bash
rustup update
```

Then test the project:
```bash
cargo test
```

## Project Structure
The template includes a sample program `first_ten_distributed`.

`first_ten_distributed` demonstrates how to use Hydroflow+ to create dataflow programs for a distributed system, and can be launched by running `cargo run -p flow --example first_ten_distributed`. Note the use of `--example` here because `src/bin/first_ten_distributed.rs` contains the binary that will be launched for each process, whereas `examples/first_ten_distributed.rs` contains a deployment script for connecting the processes together.

This template also comes with an example of deploying the `first_ten_distributed` flow to Google Cloud. To deploy, you will need to install the [Google Cloud SDK](https://cloud.google.com/sdk/docs/install) and [Terraform](https://developer.hashicorp.com/terraform/install). Then, authenticate with Google Cloud and launch the deployment script with your project ID as an argument:

```bash
$ gcloud auth application-default login
$ cargo run --example first_ten_distributed_gcp -- YOUR_PROJECT_ID_HERE
```
