use dfir_rs::scheduled::context::Context;
use quote::quote;
use stageleft::runtime_support::{FreeVariableWithContext, QuoteTokens};

use crate::Location;

pub static RUNTIME_CONTEXT: RuntimeContext = RuntimeContext { _private: &() };

#[derive(Clone, Copy)]
pub struct RuntimeContext<'a> {
    _private: &'a (),
}

impl<'a, L> FreeVariableWithContext<L> for RuntimeContext<'a>
where
    L: Location<'a>,
{
    type O = &'a Context;

    fn to_tokens(self, _ctx: &L) -> QuoteTokens {
        QuoteTokens {
            prelude: None,
            expr: Some(quote!(&context)),
        }
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;
    use hydro_deploy::Deployment;

    use super::RUNTIME_CONTEXT;
    use crate::*;

    struct P1 {}

    #[tokio::test]
    async fn runtime_context() {
        let mut deployment = Deployment::new();

        let flow = FlowBuilder::new();
        let node = flow.process::<P1>();
        let external = flow.external::<()>();

        let out_port = node
            .source_iter(q!(0..5))
            .map(q!(|v| (v, RUNTIME_CONTEXT.current_tick().0)))
            .send_bincode_external(&external);

        let nodes = flow
            .with_process(&node, deployment.Localhost())
            .with_external(&external, deployment.Localhost())
            .deploy(&mut deployment);

        deployment.deploy().await.unwrap();

        let mut external_out = nodes.connect_source_bincode(out_port).await;

        deployment.start().await.unwrap();

        for i in 0..5 {
            assert_eq!(external_out.next().await.unwrap(), (i, 0));
        }
    }
}
