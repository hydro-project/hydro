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
        let mut flow = FlowBuilder::new();
        let process = flow.process();

        let (in_port, requests) = process.sim_input();
        let responses = super::echo_capitalize(requests);
        let out_port = responses.sim_output();

        flow.sim().exhaustive(async || {
            in_port.send("hello".to_owned());
            in_port.send("world".to_owned());

            out_port.assert_yields_only(["HELLO", "WORLD"]).await;
        });
    }
}
