from codecs import decode
import json
from pathlib import Path
import pytest
import hydro

@pytest.mark.asyncio
async def test_connect_to_self():
    deployment = hydro.Deployment()
    localhost_machine = deployment.Localhost()

    program = deployment.HydroflowCrate(
        src=str((Path(__file__).parent.parent.parent / "hydro_cli_examples").absolute()),
        example="empty_program",
        profile="dev",
        on=localhost_machine
    )

    program.ports.out.send_to(program.ports.input)

    await deployment.deploy()
    await deployment.start()

@pytest.mark.asyncio
async def test_python_sender():
    deployment = hydro.Deployment()
    localhost_machine = deployment.Localhost()

    sender = deployment.CustomService(
        external_ports=[],
        on=localhost_machine.client_only(),
    )

    receiver = deployment.HydroflowCrate(
        src=str((Path(__file__).parent.parent.parent / "hydro_cli_examples").absolute()),
        example="stdout_receiver",
        profile="dev",
        on=localhost_machine
    )

    sender_port_1 = sender.client_port()
    sender_port_1.send_to(receiver.ports.echo.merge())

    sender_port_2 = sender.client_port()
    sender_port_2.send_to(receiver.ports.echo.merge())

    await deployment.deploy()

    # create this as separate variable to indicate to Hydro that we want to capture all stdout, even after the loop
    receiver_out = await receiver.stdout()

    await deployment.start()

    sender_1_connection = await (await sender_port_1.server_port()).into_sink()
    sender_2_connection = await (await sender_port_2.server_port()).into_sink()

    await sender_1_connection.send(bytes("hi 1!", "utf-8"))

    async for log in receiver_out:
        assert log == "echo \"hi 1!\""
        break

    await sender_2_connection.send(bytes("hi 2!", "utf-8"))
    async for log in receiver_out:
        assert log == "echo \"hi 2!\""
        break
