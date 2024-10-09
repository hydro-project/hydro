use hydroflow::util::demux_enum::DemuxEnum;
use hydroflow::hydroflow_syntax;

fn main() {
    #[derive(DemuxEnum)]
    enum Shape {
        Square(f64),
        Rectangle { w: f64, h: f64 },
    }

    let mut df = hydroflow_syntax! {
        my_demux = source_iter([
            Shape::Rectangle { w: 10.0, h: 8.0 },
            Shape::Square(9.0),
        ]) -> demux_enum::<Shape>();
        my_demux[Rectangle] -> for_each(std::mem::drop);
    };
    df.run_available();
}
