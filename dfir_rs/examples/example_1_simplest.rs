//[use]//
use dfir_rs::dfir_syntax_inline;
//[/use]//

//[macro_call]//
pub fn main() {
    let mut flow = dfir_syntax_inline! {
        source_iter(0..10) -> for_each(|n| println!("Hello {}", n));
    };
    //[/macro_call]//

    //[run]//
    flow.run_available_sync();
    //[/run]//
}
