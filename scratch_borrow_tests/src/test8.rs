//! Test 8: Struct-based with async run_tick.
//! Tests borrowing across .await points.
//! RESULT: ✅ PASS

struct Dataflow {
    fold_state: i64,
    stream_handoff: [i64; 8],
    stream_len: usize,
}

impl Dataflow {
    fn new() -> Self {
        Self { fold_state: 0, stream_handoff: [0; 8], stream_len: 0 }
    }

    async fn run_tick(&mut self) {
        // Stratum 0: fold
        self.fold_state += 1;

        // Borrow fold_state
        let fold_ref: &i64 = &self.fold_state;

        // Simulate async subgraph (await point)
        async {}.await;

        // Stratum 1: still using the reference after await
        for i in 0..self.stream_len {
            println!("{} + {} = {}", self.stream_handoff[i], fold_ref,
                     self.stream_handoff[i] + fold_ref);
        }
        self.stream_len = 0;
    }
}

#[tokio::main]
async fn main() {
    let mut df = Dataflow::new();
    df.stream_handoff[0] = 10;
    df.stream_len = 1;
    df.run_tick().await;
    df.run_tick().await;
}
