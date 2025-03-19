use dfir_rs::scheduled::graph::Dfir;
use hydro_lang::deploy::SingleProcessGraph;
use hydro_lang::*;
use stageleft::Quoted;

pub fn first_ten(flow: &FlowBuilder) {
    let process = flow.process::<()>();
    let numbers = process.source_iter(q!(0..10));
    numbers.for_each(q!(|n| println!("{}", n)));
}

#[stageleft::entry]
pub fn first_ten_runtime<'a>(flow: FlowBuilder<'a>) -> impl Quoted<'a, Dfir<'a>> {
    first_ten(&flow);
    flow.compile_no_network::<SingleProcessGraph>()
}

#[cfg(stageleft_runtime)]
#[cfg(test)]
mod tests {
    #[test]
    fn instantiate_first_ten() {
        let _ = super::first_ten_runtime!();
    }
}
