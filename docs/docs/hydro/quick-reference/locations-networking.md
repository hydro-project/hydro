---
sidebar_position: 2
sidebar_label: Locations & Networking API
---

# Locations & Networking API

Use this page to find the core APIs for placing computation, creating inputs, and connecting locations. Follow the Hydro Guide links for explanations and examples, and use the Rust API links for exact definitions and details.

## Choose and Create a Location

| Task | API | Guide |
| --- | --- | --- |
| Run one copy of a computation | [`Process`](rust:hydro_lang::location::process::Process), [`FlowBuilder::process`](rust:hydro_lang::compile::builder::FlowBuilder::process) | [Processes](../reference/locations/processes.md) |
| Run the same computation across a group | [`Cluster`](rust:hydro_lang::location::cluster::Cluster), [`FlowBuilder::cluster`](rust:hydro_lang::compile::builder::FlowBuilder::cluster) | [Clusters](../reference/locations/clusters.md) |
| Represent a client outside the Hydro program | [`External`](rust:hydro_lang::location::external_process::External), [`FlowBuilder::external`](rust:hydro_lang::compile::builder::FlowBuilder::external) | [External I/O](../reference/io/index.md) |
| Create a logical clock domain | [`Tick`](rust:hydro_lang::location::tick::Tick), [`Location::tick`](rust:hydro_lang::location::Location::tick) | [Locations and Networking](../reference/locations/index.md) |

All location types implement the [`Location`](rust:hydro_lang::location::Location) trait.

## Create Inputs

| Task | API | Guide |
| --- | --- | --- |
| Create a bounded stream from an iterator | [`Location::source_iter`](rust:hydro_lang::location::Location::source_iter) | [Streams](../reference/streaming-data/streams.md#creating-a-stream) |
| Create an unbounded stream from an async stream | [`Location::source_stream`](rust:hydro_lang::location::Location::source_stream) | [External I/O](../reference/io/index.md) |
| Continuously drive a computation | [`Location::spin`](rust:hydro_lang::location::Location::spin) | [Locations and Networking](../reference/locations/index.md) |
| Observe cluster membership changes | [`Location::source_cluster_membership_stream`](rust:hydro_lang::location::Location::source_cluster_membership_stream), [`MembershipEvent`](rust:hydro_lang::location::MembershipEvent) | [Broadcasting and Membership Lists](../reference/locations/clusters.md#broadcasting-and-membership-lists) |
| Receive one-way bincode input from an external client | [`Location::source_external_bincode`](rust:hydro_lang::location::Location::source_external_bincode) | [External I/O](../reference/io/index.md) |
| Bind a bidirectional bincode client | [`Location::bind_single_client_bincode`](rust:hydro_lang::location::Location::bind_single_client_bincode) | [External I/O](../reference/io/index.md) |
| Create a simulator-controlled input | [`Location::sim_input`](rust:hydro_lang::location::Location::sim_input) | [Writing Simulation Tests](../reference/simulation/writing.mdx) |

## Integrate Existing Code

| Task | API | Guide |
| --- | --- | --- |
| Connect an async stream/sink sidecar | [`Location::sidecar_bidi`](rust:hydro_lang::location::Location::sidecar_bidi), [`ForwardHandle::complete`](rust:hydro_lang::forward_handle::ForwardHandle::complete) | [Sidecars](../reference/io/sidecar.md#the-sidecar_bidi-api) |
| Add a stream parameter to a generated embedded function | [`Location::embedded_input`](rust:hydro_lang::location::Location::embedded_input) | [Embedded Mode](../reference/deploy/embedded.mdx#how-it-works) |
| Add a plain value parameter to a generated embedded function | [`Location::embedded_singleton_input`](rust:hydro_lang::location::Location::embedded_singleton_input) | [Singleton Inputs](../reference/deploy/embedded.mdx#singleton-inputs) |
| Add an output callback to a generated embedded function | [`Stream::embedded_output`](rust:hydro_lang::live_collections::stream::Stream::embedded_output) | [Embedded Mode](../reference/deploy/embedded.mdx#how-it-works) |

## Configure Network Channels

| Task | API | Guide |
| --- | --- | --- |
| Select a transport | [`TCP`](rust:hydro_lang::networking::TCP), [`UDP`](rust:hydro_lang::networking::UDP) | [Locations and Networking](../reference/locations/index.md) |
| Choose TCP fail-stop behavior | [`NetworkingConfig::fail_stop`](rust:hydro_lang::networking::NetworkingConfig::fail_stop) | [Process Networking](../reference/locations/processes.md#networking) |
| Allow message loss explicitly | [`NetworkingConfig::lossy`](rust:hydro_lang::networking::NetworkingConfig::lossy) | [Non-Determinism and `nondet!`](../reference/correctness/nondet.md) |
| Model dropped messages as indefinitely delayed | [`NetworkingConfig::lossy_delayed_forever`](rust:hydro_lang::networking::NetworkingConfig::lossy_delayed_forever) | [Writing Simulation Tests](../reference/simulation/writing.mdx) |
| Serialize channel values with bincode | [`NetworkingConfig::bincode`](rust:hydro_lang::networking::NetworkingConfig::bincode) | [Process Networking](../reference/locations/processes.md#networking) |
| Leave serialization to embedded host code | [`NetworkingConfig::embedded`](rust:hydro_lang::networking::NetworkingConfig::embedded) | [Embedded Serialization](../reference/deploy/embedded.mdx#embedded-serialization) |
| Name a channel across service versions or for generated embedded fields | [`NetworkingConfig::name`](rust:hydro_lang::networking::NetworkingConfig::name) | [Embedded Networking](../reference/deploy/embedded.mdx#networking) |

## Supporting Types

- [`MemberId`](rust:hydro_lang::location::member_id::MemberId) identifies a cluster member; see [Clusters](../reference/locations/clusters.md).
- [`MembershipEvent`](rust:hydro_lang::location::MembershipEvent) reports joins and departures.
- [`NetworkHint`](rust:hydro_lang::location::NetworkHint) configures raw external bindings.
- Browse the complete [`location`](rust:hydro_lang::location) and [`networking`](rust:hydro_lang::networking) modules for every public item.
