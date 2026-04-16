use dfir_rs::util::demux_enum::DemuxEnum;
use dfir_rs::dfir_syntax;

fn main() {
    #[derive(DemuxEnum)]
    enum Shape {
    }

    let mut df = dfir_syntax! {
        my_demux = source_iter([]) -> demux_enum::<Shape>();
        my_demux[Square] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
