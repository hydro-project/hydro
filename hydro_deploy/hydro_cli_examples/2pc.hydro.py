import hydro
import json

async def main(args):
    machine_1_gcp = args[0] == "gcp"
    machine_2_gcp = args[1] == "gcp"
    num_participants = int(args[2])

    deployment = hydro.Deployment()
    localhost_machine = deployment.Localhost()

    machine1 = deployment.GcpComputeEngineHost(
        project="autocompartmentalization",
        machine_type="e2-micro",
        image="debian-cloud/debian-11",
        region="us-west1-a"
    ) if machine_1_gcp else localhost_machine

    machine2 = deployment.GcpComputeEngineHost(
        project="autocompartmentalization",
        machine_type="e2-micro",
        image="debian-cloud/debian-11",
        region="us-west1-a"
    ) if machine_2_gcp else localhost_machine

    program = deployment.HydroflowCrate(
        src=".",
        example="dedalus_2pc_coordinator",
        on=machine1
    )

    participant_programs = []
    participant_vote_ports = {}
    participant_instruct_ports = {}
    for i in range(0, num_participants):
        program2 = deployment.HydroflowCrate(
            src=".",
            example="dedalus_2pc_participant",
            args=[json.dumps([i])],
            on=machine2
        )
        program2.ports.vote_from_participant.send_to(hydro.demux({
            0: program.ports.vote_from_participant.merge()
        }))
        program2.ports.ack_from_participant.send_to(hydro.demux({
            0: program.ports.ack_from_participant.merge()
        }))
        participant_programs.append(program2)
        participant_vote_ports[i] = program2.ports.vote_to_participant
        participant_instruct_ports[i] = program2.ports.instruct_to_participant

    program.ports.vote_to_participant.send_to(hydro.demux(participant_vote_ports))
    program.ports.instruct_to_participant.send_to(hydro.demux(participant_instruct_ports))

    await deployment.deploy()

    print("deployed!")

    # create this as separate variable to indicate to Hydro that we want to capture all stdout, even after the loop
    program_out = await program.stdout()

    await deployment.start()
    print("started!")

    counter = 0
    async for log in program_out:
        print(log)
        counter += 1
        if counter == 25:
            break

    print(await program.exit_code())

if __name__ == "__main__":
    import sys
    import hydro.async_wrapper
    hydro.async_wrapper.run(main, sys.argv[1:])
