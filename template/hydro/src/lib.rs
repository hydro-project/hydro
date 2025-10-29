hydro_lang::setup!();

use hydro_lang::prelude::*;

struct EchoServer;
pub fn echo_capitalize<'a>(
    input: Stream<String, Process<'a, EchoServer>, Unbounded>,
) -> Stream<String, Process<'a, EchoServer>, Unbounded> {
    input.map(q!(|s| s.to_uppercase()))
}

#[cfg(test)]
mod tests {
    use hydro_lang::prelude::*;

    fn test_echo_capitalize() {
        let flow = FlowBuilder::new();
        let external = flow.external::<()>();
        let node = flow.process::<()>();

        let (in_port, input) = node.source_external_bincode(&external);
        let out_port = super::echo_capitalize(input).send_bincode_external(&external);

        flow.sim().exhaustive(async |mut compiled| {
            let mut in_port = compiled.connect(&in_port);
            let mut out_port = compiled.connect(&out_port);

            compiled.launch();

            in_port.send("hello".to_string()).await.unwrap();
            in_port.send("world".to_string()).await.unwrap();
            
            out_port.assert_yields_only([
                "HELLO".to_string(),
                "WORLD".to_string(),
            ]).await;
        });
    }
}
