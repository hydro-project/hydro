# Hydro CLI Design
Hydroflow makes it possible to describe single node/thread stream processors, but is unopinionated when it comes to networking and deployment. Working towards the goal of Hydro being and end-to-end stack for distributed programming, the first step is to automate **deploying** standalone Hydroflow programs to cloud machines and **wiring** them together to form a distributed application. This document describes the design of the Hydro CLI, which will be used to achieve these goals.

The Hydro CLI (`hydro`) allows developers to define Python and JavaScript programs that specify how a set of Hydroflow programs should be deployed. Unlike Terraform, where config files generate a static JSON spec, and SkyPilot, which uses YAML config to specify constraints on machines, Hydro config files are **dynamic**. A single config file can be used to generate multiple deployments, each with different parameters. This allows developers to easily experiment with different configurations and conditions. And on top of that, the same config files can interact and monitor with **live** deployments, making it possible to run complex experiments.

We envision the following use cases:
- Deploying services (like chat apps) to the cloud and exposing them to the internet
- Running research experiments (like compartmentalized paxos) that involve several configurations and dynamic conditions (like network partitions)
- Running applications that involve a mix of Hydroflow and non-Hydroflow components (like a web server and a database)

## Simple Example
Consider the classic echo program example in Hydroflow:
```rust
// src/echo_server.hf
use data_types::EchoMsg; // defined in data_types crate in this workspace
use chrono::prelude::*;
use std::net::SocketAddr;

module main(
    input inbound: (EchoMsg, SocketAddr),
    output outbound: (EchoMsg, SocketAddr)
) {
    // Define a shared inbound channel
    inbound_chan = inbound -> tee();

    // Print all messages for debugging purposes
    inbound_chan[0]
        -> for_each(|(msg, addr): (EchoMsg, SocketAddr)| println!("{}: Got {:?} from {:?}", Utc::now(), msg, addr));

    // Echo back the Echo messages with updated timestamp
    inbound_chan[1]
        -> map(|(EchoMsg {payload, ..}, addr)| (EchoMsg { payload, ts: Utc::now() }, addr) ) -> outbound;
}
```

Assume we also have a `src/echo_client.hf` file that periodically sends `EchoMsg`s to an `outbound` channel and logs echos from the `inbound` channel. We can deploy this program to a cloud machine using the following `echo.hydro.py` config file:
```python
from hydro import Deployment
from hydro.gcp import GcpMachine # keys are automatically loaded from somewhere

async def main():
    deployment = Deployment()

    # Specify the GCP instances we want to deploy to
    server_machine = deployment.GcpMachine(
        project="hydro-1234",
        zone="us-west1-a",
        type="e2-micro",
    )

    client_machine = deployment.GcpMachine(
        project="hydro-1234",
        zone="us-west1-a",
        type="e2-micro",
    )

    # Load Hydroflow programs
    server_hf = deployment.Hydroflow(
        src="src/echo_server.hf",
        on=server_machine,
    )

    client_hf = deployment.Hydroflow(
        src="src/echo_client.hf",
        on=client_machine,
    )

    # Wire the programs
    client_hf.ports.outbound.connect(server_hf.ports.inbound)
    server_hf.ports.outbound.connect(client_hf.ports.inbound)

    # Launch the two programs
    await deployment.deploy()

    # Once deployment finishes, start capturing logs from the client
    # (logs are buffered even before we grab `stdout`)
    client_logs = client_hf.stdout()

    i = 0
    async for log in client_logs:
        print(log)
        i += 1
        if i == 10:
            break

    # Stop the programs (optional)
    await client_hf.stop()
    await server_hf.stop()

    # Kill the machines (optional)
    await client_machine.terminate()
    await server_machine.terminate()
```

Then, we can launch this deployment with a simple command:
```bash
$ hydro run echo.hydro.py
```

Which will automatically compile the Hydroflow programs for the target machine, launch the cloud machines, allocate internal ports for each connected input/output, and start the services.

## Standalone Hydroflow
This will eventually be a design document of its own, but for now we briefly lay out a prototype of how to augment Hydroflow programs to be meaningful without a surrounding Rust program.

TODO
