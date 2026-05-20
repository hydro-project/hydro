//! Test 7: Generated struct with run_tick. No closure lifetime issues.
//! Also tests: can we hold &fold_state while iterating stream_handoff?
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

    fn run_tick(&mut self) {
        // Stratum 0: fold
        self.fold_state += 1;

        // Stratum 1: borrow fold_state while reading stream_handoff
        let fold_ref: &i64 = &self.fold_state;
        for i in 0..self.stream_len {
            println!("{} + {} = {}", self.stream_handoff[i], fold_ref,
                     self.stream_handoff[i] + fold_ref);
        }
        self.stream_len = 0;
    }
}

fn main() {
    let mut df = Dataflow::new();
    df.stream_handoff[0] = 10;
    df.stream_handoff[1] = 20;
    df.stream_len = 2;
    df.run_tick();
    df.run_tick();
}
