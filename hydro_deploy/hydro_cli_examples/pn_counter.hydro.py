import asyncio
from codecs import decode
import hydro
import json
from pathlib import Path
from aiostream import stream

# hydro deploy ../hydro_cli_examples/toplotree.hydro.py -- local/gcp DEPTH_OF_TREE
async def main(args):
    num_replicas = int(args[1])
    deployment = hydro.Deployment()

    localhost_machine = deployment.Localhost()

    python_sender = deployment.CustomService(
        external_ports=[],
        on=localhost_machine,
    )

    gcp_vpc = hydro.GcpNetwork(
        project="hydro-chrisdouglas",
    )

    def create_machine():
        if args[0] == "gcp":
            return deployment.GcpComputeEngineHost(
                project="hydro-chrisdouglas",
                machine_type="e2-micro",
                image="debian-cloud/debian-11",
                region="us-west1-a",
                network=gcp_vpc
            )
        else:
            return localhost_machine

    cluster = [
        deployment.HydroflowCrate(
            src=str(Path(__file__).parent.absolute()),
            example="pn_counter",
            args=[json.dumps([i]), json.dumps([num_replicas])],
            on=create_machine()
        )
        for i in range(num_replicas)
    ]

    for i in range(num_replicas):
        cluster[i].ports.to_peer.send_to(hydro.demux(
            {
                j: cluster[j].ports.from_peer.merge()
                for j in range(num_replicas)
                if i != j
            }
        ))

    def create_increment_port(node):
        port = python_sender.client_port()
        port.send_to(node.ports.increment_requests)
        return port

    cluster_increment_ports = [create_increment_port(node) for node in cluster]

    def create_query_response_port(node):
        port = python_sender.client_port()
        node.ports.query_responses.send_to(port)
        return port

    cluster_query_response_ports = [create_query_response_port(node) for node in cluster]

    await deployment.deploy()

    async def get_increment_channel(port):
        return await (await port.server_port()).into_sink()
    cluster_increment_channels = [await get_increment_channel(port) for port in cluster_increment_ports]

    async def get_query_response_channel(port):
        return await (await port.server_port()).into_source()
    cluster_query_response_channels = [await get_query_response_channel(port) for port in cluster_query_response_ports]

    # async def get_stdouts(node):
    #     return await node.stdout()
    # cluster_stdouts = [await get_stdouts(node) for node in cluster]

    def stream_printer(i, v):
        parsed = json.loads(decode(bytes(v), "utf-8"))
        return f"{i}: {parsed}"

    print("deployed!")

    with_path_responses = [
        stream.map(response, lambda x,i=i: stream_printer(i, x))
        for (i, response) in enumerate(cluster_query_response_channels)
    ]

    async def print_queries():
        try:
            async with stream.merge(*with_path_responses).stream() as merged:
                async for log in merged:
                    print(log)
        except asyncio.CancelledError:
            pass
    print_query_task = asyncio.create_task(print_queries())

    try:
        await deployment.start()
        print("started!")

        await asyncio.sleep(1)

        for i in range(1000000):
            if i % 10000 == 0:
                print(f"sending increment {i}")
            await cluster_increment_channels[0].send(bytes("{\"tweet_id\": " + str(i) + ", \"likes\": " + str(i % 2 * 2 - 1) + "}", "utf-8"))
    finally:
        print_query_task.cancel()
        try:
            await print_query_task
        except:
            pass

if __name__ == "__main__":
    import sys
    import hydro.async_wrapper
    hydro.async_wrapper.run(main, sys.argv[1:])
