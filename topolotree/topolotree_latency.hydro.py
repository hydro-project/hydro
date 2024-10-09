import asyncio
from codecs import decode
from typing import List, Optional
import hydro
import json
from pathlib import Path
from aiostream import stream

import pandas as pd
import numpy as np
import uuid


# given a list of IDs for each node in a binary tree,
# organize the nodes into a binary tree and compute the neighbors
# of each node
def get_neighbors_in_binary_tree(flat: List[int]) -> List[List[int]]:
    tree = []
    for i in range(len(flat)):
        tree.append([])
        if i > 0:
            # add the parent
            tree[i].append((i - 1) // 2)
        if 2 * i + 1 < len(flat):
            # add the left child
            tree[i].append(2 * i + 1)
        if 2 * i + 2 < len(flat):
            # add the right child
            tree[i].append(2 * i + 2)
    return tree

def get_leaves_in_binary_tree(flat: List[int]) -> List[int]:
    tree = get_neighbors_in_binary_tree(flat)
    leaves = []
    for i in range(len(tree)):
        if len(tree[i]) == 1:
            leaves.append(i)
    return leaves


async def run_experiment(
    deployment: hydro.Deployment,
    localhost_machine: hydro.LocalhostHost,
    profile,
    machine_pool,
    experiment_id,
    summaries_file,
    tree_arg,
    depth_arg,
    clients_arg,
    partitions_arg,
    is_gcp,
    gcp_vpc,
):
    tree_depth = int(depth_arg)
    is_tree = tree_arg == "topolo"  # or "pn"

    num_replicas = 2**tree_depth - 1

    num_clients = int(clients_arg)

    num_partitions = int(partitions_arg)

    print(f"Launching benchmark with protocol {tree_arg}, {num_replicas} replicas, and {num_clients} clients on {num_partitions} partitions")

    currently_deployed = []

    def create_machine():
        if len(machine_pool) > 0:
            ret = machine_pool.pop()
            currently_deployed.append(ret)
            return ret
        else:
            if is_gcp:
                out = deployment.GcpComputeEngineHost(
                    project="hydro-chrisdouglas",
                    machine_type="n2-standard-4",
                    image="debian-cloud/debian-11",
                    region="us-west1-a",
                    network=gcp_vpc,
                )
            else:
                out = localhost_machine
            currently_deployed.append(out)
            return out

    all_nodes = []
    neighbors = get_neighbors_in_binary_tree(list(range(num_replicas)))
    cluster = [
        deployment.HydroflowCrate(
            src=str(
                Path(__file__).parent.absolute()
            ),
            profile=profile,
            bin="topolotree",
            args=([str(i)] + [str(neighbor) for neighbor in neighbors[i]]),
            on=create_machine(),
        ) if is_tree else deployment.HydroflowCrate(
            src=str(
                Path(__file__).parent.absolute()
            ),
            profile=profile,
            bin="pn" if tree_arg == "pn" else "pn_delta",
            args=[json.dumps([i]), json.dumps([num_replicas])],
            on=create_machine(),
        )
        for i in range(num_replicas)
    ]

    for i in range(num_replicas):
        cluster[i].ports.to_peer.tagged(i).send_to(
            hydro.demux(
                {
                    j: cluster[j].ports.from_peer.merge()
                    for j in range(num_replicas)
                }
            )
        )

    all_nodes = cluster

    if is_tree:
        leaves = get_leaves_in_binary_tree(list(range(num_replicas)))
        sources = list(cluster[i] for i in leaves[: len(leaves) // 2])[:num_partitions]
        dests = list(cluster[i] for i in leaves[len(leaves) // 2 :])[:num_partitions]
    else:
        sources = list(cluster[i] for i in list(range(num_replicas))[: num_replicas // 2])[:num_partitions]
        dests = list(cluster[i] for i in list(range(num_replicas))[num_replicas // 2 :])[:num_partitions]

    for node in all_nodes:
        if node not in sources:
            hydro.null().send_to(node.ports.increment_requests)
        if node not in dests:
            node.ports.query_responses.send_to(hydro.null())

    drivers = [deployment.HydroflowCrate(
        src=str(Path(__file__).parent.absolute()),
        profile=profile,
        bin="latency_measure",
        # 2 ** 14 for memory / latency experiments
        args=[str(num_clients), str(i), str((2 ** 10) // num_partitions)],
        on=create_machine(),
    ) for i in range(num_partitions)]

    for i in range(num_partitions):
        drivers[i].ports.increment_start_node.send_to(
            sources[i].ports.increment_requests
        )
        dests[i].ports.query_responses.send_to(drivers[i].ports.end_node_query)

    await deployment.deploy()

    print("Deployed!")

    latency_per_driver = [[] for _ in range(num_partitions)]
    memory_per_node = [[] for _ in range(num_replicas)]
    throughput_raw_per_driver = [[] for _ in range(num_partitions)]
    throughput_per_driver = [[] for _ in range(num_partitions)]

    latency_streams_with_index = [
        stream.map(await node.stdout(), lambda x, i=i: (i, x))
        for i, node in enumerate(drivers)
    ]

    memories_streams_with_index = [
        stream.map(await node.stdout(), lambda x, i=i: (i, x))
        for i, node in enumerate(all_nodes)
    ]

    async def memory_plotter():
        try:
            async with stream.merge(*memories_streams_with_index).stream() as merged:
                async for node_idx, line in merged:
                    line_split = line.split(",")
                    if line_split[0] == "memory":
                        memory_per_node[node_idx].append(int(line_split[1]))
        except asyncio.CancelledError:
            return

    memory_plotter_task = asyncio.create_task(memory_plotter())

    async def latency_plotter():
        try:
            async with stream.merge(*latency_streams_with_index).stream() as merged:
                async for (driver_idx, line) in merged:
                    line_split = line.split(",")
                    if line_split[0] == "throughput":
                        count = int(line_split[1])
                        period = float(line_split[2])
                        throughput_raw_per_driver[driver_idx].append([count, period])
                        throughput_per_driver[driver_idx].append(count / period)
                    elif line_split[0] == "latency":
                        number = int(line_split[1])  # microseconds
                        latency_per_driver[driver_idx].append(number)
                    elif line_split[0] == "end":
                        break
        except asyncio.CancelledError:
            return

    latency_plotter_task = asyncio.create_task(latency_plotter())

    await deployment.start()
    print("Started! Please wait 60 seconds to collect data.")

    await asyncio.sleep(60)

    print("Stopping all drivers")
    ignore_stderrs = await asyncio.gather(*[node.stderr() for node in all_nodes])

    await asyncio.gather(*[node.stop() for node in drivers])

    print("Collecting latency logs")
    await latency_plotter_task

    memory_plotter_task.cancel()
    await memory_plotter_task

    await asyncio.gather(*[node.stop() for node in all_nodes])

    def summarize(v, kind):
        print("mean = ", np.mean(v))
        print("std = ", np.std(v))
        print("min = ", np.min(v))
        print("max = ", np.max(v))
        print("percentile 99 = ", np.percentile(v, 99))
        print("percentile 75 = ", np.percentile(v, 75))
        print("percentile 50 = ", np.percentile(v, 50))
        print("percentile 25 = ", np.percentile(v, 25))
        print("percentile 1 = ", np.percentile(v, 1))

        summaries_file.write("\n")
        summaries_file.write(tree_arg + ",")
        summaries_file.write(str(tree_depth) + ",")
        summaries_file.write(str(num_clients) + ",")
        summaries_file.write(str(num_partitions) + ",")
        summaries_file.write(kind + ",")
        summaries_file.write(str(np.mean(v)) + ",")
        summaries_file.write(str(np.std(v)) + ",")
        summaries_file.write(str(np.min(v)) + ",")
        summaries_file.write(str(np.max(v)) + ",")
        summaries_file.write(str(np.percentile(v, 99)) + ",")
        summaries_file.write(str(np.percentile(v, 75)) + ",")
        summaries_file.write(str(np.percentile(v, 50)) + ",")
        summaries_file.write(str(np.percentile(v, 25)) + ",")
        summaries_file.write(str(np.percentile(v, 1)))
        summaries_file.flush()

    all_latencies = []
    all_throughputs = []
    all_throughputs_raw = []
    for i in range(num_partitions):
        all_latencies += latency_per_driver[i]
        all_throughputs += throughput_per_driver[i]
        all_throughputs_raw += throughput_raw_per_driver[i]

    print("latency:")
    summarize(all_latencies, "latency")

    print("total throughput:")
    print(throughput_per_driver)
    summarize([v * num_partitions for v in all_throughputs], "total_throughput")

    memory_delta = [memory[-1] - memory[0] for memory in memory_per_node]
    print("memory delta:")
    summarize(memory_delta, "memory_delta")

    pd.DataFrame(all_latencies).to_csv(
        "latency_"
        + tree_arg
        + "_tree_depth_"
        + str(tree_depth)
        + "_num_clients_"
        + str(num_clients)
        + "_num_partitions_"
        + str(num_partitions)
        + "_"
        + experiment_id
        + ".csv",
        index=False,
        header=["latency"],
    )
    pd.DataFrame(all_throughputs_raw).to_csv(
        "throughput_per_driver_"
        + tree_arg
        + "_tree_depth_"
        + str(tree_depth)
        + "_num_clients_"
        + str(num_clients)
        + "_num_partitions_"
        + str(num_partitions)
        + "_"
        + experiment_id
        + ".csv",
        index=False,
        header=["count", "period"],
    )
    pd.DataFrame(memory_delta).to_csv(
        "memory_delta_"
        + tree_arg
        + "_tree_depth_"
        + str(tree_depth)
        + "_num_clients_"
        + str(num_clients)
        + "_num_partitions_"
        + str(num_partitions)
        + "_"
        + experiment_id
        + ".csv",
        index=False,
        header=["memory_delta"],
    )

    for machine in currently_deployed:
        machine_pool.append(machine)


# hydro deploy toplotree_latency.hydro.py -- local/gcp once/pn/pn_counter_delta DEPTH_OF_TREE NUM_CLIENTS
async def main(args):
    # the current timestamp
    import datetime

    experiment_id = str(datetime.datetime.now())

    summaries_file = open(f"summaries_{experiment_id}.csv", "w")
    summaries_file.write(
        "protocol,tree_depth,num_clients,num_partitions,kind,mean,std,min,max,percentile_99,percentile_75,percentile_50,percentile_25,percentile_1"
    )

    deployment = hydro.Deployment()
    localhost = deployment.Localhost()
    pool = []

    network = (
        hydro.GcpNetwork(
            project="hydro-chrisdouglas",
        )
        if args[0] == "gcp"
        else None
    )

    for depth_arg in args[2].split(","):
        for clients_partitions in args[3].split(","):
            for tree_arg in args[1].split(","):
                split = clients_partitions.split("/")
                num_clients_arg = split[0]
                num_partitions_arg = split[1]
                await run_experiment(
                    deployment,
                    localhost,
                    "dev" if args[0] == "local" else None,
                    pool,
                    experiment_id,
                    summaries_file,
                    tree_arg,
                    depth_arg,
                    num_clients_arg,
                    num_partitions_arg,
                    args[0] == "gcp",
                    network,
                )

    summaries_file.close()


if __name__ == "__main__":
    import sys
    import hydro.async_wrapper

    hydro.async_wrapper.run(main, sys.argv[1:])
