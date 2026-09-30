---
sidebar_position: 3
sidebar_label: Program Lifecycle API
---

# Program Lifecycle API

Use this page to find the core APIs for building, optimizing, simulating, compiling, and deploying a Hydro program. Follow the Hydro Guide links for explanations and examples, and use the Rust API links for exact definitions and details.

## Build the Program

| Task | API | Guide |
| --- | --- | --- |
| Create a flow | [`FlowBuilder::new`](rust:hydro_lang::compile::builder::FlowBuilder::new), [`FlowBuilder::with_name`](rust:hydro_lang::compile::builder::FlowBuilder::with_name) | [Locations and Networking](../reference/locations/index.md#creating-locations) |
| Add a process | [`FlowBuilder::process`](rust:hydro_lang::compile::builder::FlowBuilder::process) | [Processes](../reference/locations/processes.md) |
| Add a cluster | [`FlowBuilder::cluster`](rust:hydro_lang::compile::builder::FlowBuilder::cluster) | [Clusters](../reference/locations/clusters.md) |
| Add an external client | [`FlowBuilder::external`](rust:hydro_lang::compile::builder::FlowBuilder::external) | [External I/O](../reference/io/index.md) |
| Finish graph construction | [`FlowBuilder::finalize`](rust:hydro_lang::compile::builder::FlowBuilder::finalize) | [Locations and Networking](../reference/locations/index.md) |

## Optimize, Compile, and Deploy

| Task | API | Guide |
| --- | --- | --- |
| Apply the default optimization pipeline | [`FlowBuilder::with_default_optimize`](rust:hydro_lang::compile::builder::FlowBuilder::with_default_optimize) | [Hydro Deploy](../reference/deploy/index.mdx) |
| Apply a custom optimization | [`FlowBuilder::optimize_with`](rust:hydro_lang::compile::builder::FlowBuilder::optimize_with) | [Hydro Deploy](../reference/deploy/index.mdx) |
| Assign a process to a deployment target | [`FlowBuilder::with_process`](rust:hydro_lang::compile::builder::FlowBuilder::with_process) | [Hydro Deploy](../reference/deploy/index.mdx) |
| Assign a cluster to deployment targets | [`FlowBuilder::with_cluster`](rust:hydro_lang::compile::builder::FlowBuilder::with_cluster) | [Hydro Deploy](../reference/deploy/index.mdx) |
| Assign an external client to a deployment target | [`FlowBuilder::with_external`](rust:hydro_lang::compile::builder::FlowBuilder::with_external) | [Hydro Deploy](../reference/deploy/index.mdx) |
| Compile without launching | [`FlowBuilder::compile`](rust:hydro_lang::compile::builder::FlowBuilder::compile) | [Hydro Deploy](../reference/deploy/index.mdx) |
| Instantiate a deployment | [`FlowBuilder::deploy`](rust:hydro_lang::compile::builder::FlowBuilder::deploy) | [Hydro Deploy](../reference/deploy/index.mdx) |

Hydro Deploy is currently alpha; use the Hydro Guide for current deployment setup and limitations.

## Generate Embedded Code

| Task | API | Guide |
| --- | --- | --- |
| Register process and cluster function names | [`FlowBuilder::with_process`](rust:hydro_lang::compile::builder::FlowBuilder::with_process), [`FlowBuilder::with_cluster`](rust:hydro_lang::compile::builder::FlowBuilder::with_cluster) | [Embedded Mode](../reference/deploy/embedded.mdx#how-it-works) |
| Generate embeddable Rust functions | [`EmbeddedDeploy`](rust:hydro_lang::compile::embedded::EmbeddedDeploy), [`DeployFlow::generate_embedded`](rust:hydro_lang::compile::deploy::DeployFlow::generate_embedded) | [Embedded Mode](../reference/deploy/embedded.mdx) |

`generate_embedded` is available with the `build` feature. Generated wrappers use the `runtime_support` feature.

## Simulate and Test

| Task | API | Guide |
| --- | --- | --- |
| Create a deterministic simulation | [`FlowBuilder::sim`](rust:hydro_lang::compile::builder::FlowBuilder::sim), [`SimFlow`](rust:hydro_lang::sim::flow::SimFlow) | [Simulation Testing](../reference/simulation/index.mdx) |
| Set a simulated cluster's maximum size | [`SimFlow::with_cluster_size`](rust:hydro_lang::sim::flow::SimFlow::with_cluster_size) | [Coverage-Guided Simulation](../reference/simulation/fuzzing.mdx) |
| Explore every execution | [`SimFlow::exhaustive`](rust:hydro_lang::sim::flow::SimFlow::exhaustive) | [Writing Simulation Tests](../reference/simulation/writing.mdx) |
| Run coverage-guided exploration | [`SimFlow::fuzz`](rust:hydro_lang::sim::flow::SimFlow::fuzz) | [Coverage-Guided Simulation](../reference/simulation/fuzzing.mdx) |
| Inspect the compiled simulation | [`SimFlow::compiled`](rust:hydro_lang::sim::flow::SimFlow::compiled) | [Writing Simulation Tests](../reference/simulation/writing.mdx) |
| Test safety without requiring liveness | [`SimFlow::test_safety_only`](rust:hydro_lang::sim::flow::SimFlow::test_safety_only) | [Simulation Testing](../reference/simulation/index.mdx) |
| Skip unsupported consistency assertions | [`SimFlow::skip_consistency_assertions`](rust:hydro_lang::sim::flow::SimFlow::skip_consistency_assertions) | [Simulation Testing](../reference/simulation/index.mdx) |

Browse [`FlowBuilder`](rust:hydro_lang::compile::builder::FlowBuilder) and [`SimFlow`](rust:hydro_lang::sim::flow::SimFlow) for the complete lifecycle API.
