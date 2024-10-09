use hydroflow_plus::deploy::SingleProcessGraph;
use hydroflow_plus::*;
use stageleft::*;

pub fn first_ten(flow: &FlowBuilder) {
    let process = flow.process::<()>();
    let numbers = process.source_iter(q!(0..10));
    numbers.for_each(q!(|n| println!("{}", n)));
}

#[stageleft::entry]
pub fn first_ten_runtime<'a>(flow: FlowBuilder<'a>) -> impl Quoted<'a, Hydroflow<'a>> {
    first_ten(&flow);
    flow.with_default_optimize()
        .compile_no_network::<SingleProcessGraph>()
}

#[stageleft::runtime]
#[cfg(test)]
mod tests {
    #[test]
    fn instantiate_first_ten() {
        let _ = super::first_ten_runtime!();
    }
}
