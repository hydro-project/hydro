import hydro
import json
from pathlib import Path
from aiostream import stream

async def main(args):
    machine_1_gcp = args[0] == "gcp"
    machine_2_gcp = args[1] == "gcp"

    deployment = hydro.Deployment()
    localhost_machine = deployment.Localhost()

    gcp_vpc = hydro.GcpNetwork(
        project="autocompartmentalization",
    )

    machine1 = deployment.GcpComputeEngineHost(
        project="autocompartmentalization",
        machine_type="e2-micro",
        image="debian-cloud/debian-11",
        region="us-west1-a",
        network=gcp_vpc
    ) if machine_1_gcp else localhost_machine

    machine2 = deployment.GcpComputeEngineHost(
        project="autocompartmentalization",
        machine_type="e2-micro",
        image="debian-cloud/debian-11",
        region="us-west1-a",
        network=gcp_vpc
    ) if machine_2_gcp else localhost_machine

    sender_count = 2
    senders = [deployment.HydroflowCrate(
        src=str(Path(__file__).parent.absolute()),
        example="dedalus_sender",
        args=[json.dumps(([0, 1], i))],
        on=machine1
    ) for i in range(sender_count)]

    receiver1 = deployment.HydroflowCrate(
        src=str(Path(__file__).parent.absolute()),
        example="dedalus_receiver",
        on=machine2
    )

    receiver2 = deployment.HydroflowCrate(
        src=str(Path(__file__).parent.absolute()),
        example="dedalus_receiver",
        on=machine2
    )

    for sender in senders:
        sender.ports.broadcast.send_to(hydro.demux({
            0: receiver1.ports.broadcast.merge(),
            1: receiver2.ports.broadcast.merge()
        }))

    await deployment.deploy()

    print("deployed!")

    # create this as separate variable to indicate to Hydro that we want to capture all stdout, even after the loop
    receiver_1_out = await receiver1.stdout()
    receiver_2_out = await receiver2.stdout()

    await deployment.start()
    print("started!")

    counter = 0
    async with stream.merge(stream.map(receiver_1_out, lambda x: f"RECEIVER 1: {x}"), stream.map(receiver_2_out, lambda x: f"RECEIVER 2: {x}")).stream() as merged:
        async for log in merged:
            print(log)
            counter += 1
            if counter == 10:
                break

    for sender in senders:
        await sender.stop()
        print(await sender.exit_code())

if __name__ == "__main__":
    import sys
    import hydro.async_wrapper
    hydro.async_wrapper.run(main, sys.argv[1:])
