from codecs import decode
import json
import hydro
from pathlib import Path

async def main(args):
    machine_gcp = args[0] == "gcp"

    deployment = hydro.Deployment()
    localhost_machine = deployment.Localhost()

    gcp_vpc = hydro.GcpNetwork(
        project="autocompartmentalization",
    )

    machine2 = deployment.GcpComputeEngineHost(
        project="autocompartmentalization",
        machine_type="e2-micro",
        image="debian-cloud/debian-11",
        region="us-west1-a",
        network=gcp_vpc
    ) if machine_gcp else localhost_machine

    receiver = deployment.CustomService(
        external_ports=[],
        on=localhost_machine,
    )

    sender = deployment.HydroflowCrate(
        src=str(Path(__file__).parent.absolute()),
        example="dedalus_sender",
        args=[json.dumps(([0], 123))],
        on=machine2
    )

    sender_port = receiver.client_port()
    sender.ports.broadcast.send_to(hydro.demux({
        0: sender_port
    }))

    await deployment.deploy()

    print("deployed!")

    await deployment.start()
    print("started!")

    await deployment.deploy()
    await deployment.start()
    print("again!")

    await deployment.deploy()
    await deployment.start()
    print("and again!")

    print((await sender_port.server_port()).json())
    receiver_connection = await (await sender_port.server_port()).into_source()
    async for received in receiver_connection:
        print(decode(bytes(received), "utf-8"))

if __name__ == "__main__":
    import sys
    import hydro.async_wrapper
    hydro.async_wrapper.run(main, sys.argv[1:])
