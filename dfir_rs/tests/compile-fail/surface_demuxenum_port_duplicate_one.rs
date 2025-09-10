use dfir_rs::dfir_syntax;
use dfir_rs::util::demux_enum::DemuxEnum;

fn main() {
    #[derive(DemuxEnum)]
    enum Shape {
        Square(f64),
    }

    let mut df = dfir_syntax! {
        my_demux = source_iter([
            Shape::Square(9.0),
        ]) -> demux_enum::<Shape>();
        my_demux[Square] -> for_each(std::mem::drop);
        my_demux[Square] -> for_each(std::mem::drop);
    };
    df.run_available_sync();
}
