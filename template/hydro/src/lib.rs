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
        let node = flow.process();
        let external = flow.external::<()>();

        let (in_port, input) = node.source_external_bincode(&external);
        let out_port = super::echo_capitalize(input).send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let in_port = compiled.connect(&in_port);
            let out_port = compiled.connect(&out_port);

            compiled.launch();

            in_port.send("hello".to_string()).unwrap();
            in_port.send("world".to_string()).unwrap();

            out_port
                .assert_yields_only(["HELLO".to_string(), "WORLD".to_string()])
                .await;
        });
    }
}
