import asyncio
from typing import Dict, List
import hydro
from pathlib import Path

async def main(args):
    machine_gcp = args[0] == "gcp"
    num_replicas = int(args[1]) if len(args) > 1 else 1

    deployment = hydro.Deployment()
    localhost_machine = deployment.Localhost()

    gcp_vpc = hydro.GcpNetwork(
        project="autocompartmentalization",
    )

    machines: List[hydro.GcpComputeEngineHost] = []
    chat_servers: Dict[int, hydro.HydroflowCrate] = {}
    ports: List[int] = []
    for i in range(num_replicas):
        machine = deployment.GcpComputeEngineHost(
            project="autocompartmentalization",
            machine_type="e2-micro",
            image="debian-cloud/debian-11",
            region="us-west1-a",
            network=gcp_vpc
        ) if machine_gcp else localhost_machine

        machines.append(machine)

        port = 8080 if machine_gcp else (8080 + i)
        ports.append(port)
        chat_servers[i] = deployment.HydroflowCrate(
            src=str(Path(__file__).parent.absolute()),
            example="ws_chat_server",
            profile="release" if machine_gcp else "dev",
            on=machine,
            external_ports=[port],
            args=[str(num_replicas), str(i), str(port)]
        )

    for server in chat_servers.values():
        server.ports.to_peer.send_to(hydro.demux({
            i: to_server.ports.from_peer.merge()
            for (i, to_server) in chat_servers.items()
        }))

    await deployment.deploy()

    await deployment.start()

    if machine_gcp:
        for machine in machines:
            print(f"Will listen on {machine.external_ip}:8080")
    else:
        for port in ports:
            print(f"Will listen on localhost:{port}")

    while True:
        await asyncio.sleep(1)

if __name__ == "__main__":
    import sys
    import hydro.async_wrapper
    hydro.async_wrapper.run(main, sys.argv[1:])
