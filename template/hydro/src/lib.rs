#[cfg(stageleft_runtime)]
hydro_lang::setup!();

use hydro_lang::prelude::*;

pub struct EchoServer;
pub fn echo_capitalize<'a>(
    input: Stream<String, Process<'a, EchoServer>>,
) -> Stream<String, Process<'a, EchoServer>> {
    input.map(q!(|s| s.to_uppercase()))
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    #[test]
    fn test_echo_capitalize() {
        let flow = FlowBuilder::new();
        let process = flow.process();
        let external = flow.external::<()>();

        let (in_port, requests) = process.source_external_bincode(&external);
        let responses = super::echo_capitalize(requests);
        let out_port = responses.send_bincode_external(&external);

        flow.sim().exhaustive(async |mut instance| {
            let in_port = instance.connect(&in_port);
            let out_port = instance.connect(&out_port);

            instance.launch();

            in_port.send("hello".to_string());
            in_port.send("world".to_string());

            out_port.assert_yields_only(["HELLO", "WORLD"]).await;
        });
    }
}
