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

    sender = deployment.CustomService(
        external_ports=[],
        on=localhost_machine,
    )

    receiver = deployment.HydroflowCrate(
        src=str(Path(__file__).parent.absolute()),
        example="stdout_receiver",
        on=machine2,
        display_id="receiver"
    )

    sender_port_1 = sender.client_port()
    sender_port_1.send_to(receiver.ports.echo.merge())

    sender_port_2 = sender.client_port()
    sender_port_2.send_to(receiver.ports.echo.merge())

    await deployment.deploy()

    print("deployed!")

    await deployment.start()
    print("started!")

    sender_1_connection = await (await sender_port_1.server_port()).into_sink()
    sender_2_connection = await (await sender_port_2.server_port()).into_sink()
    
    await sender_1_connection.send(bytes("hi 1!", "utf-8"))
    await sender_2_connection.send(bytes("hi 2!", "utf-8"))

    while True:
        pass

if __name__ == "__main__":
    import sys
    import hydro.async_wrapper
    hydro.async_wrapper.run(main, sys.argv[1:])
