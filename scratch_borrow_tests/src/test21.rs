//! Test 21: Struct approach - can async fn hold references to self fields
//! in a buffer that's ALSO a field of self?
//! RESULT: ✅ PASS (using locals, not self-referential fields)

struct Dataflow {
    fold_state: i64,
    persist_buf: [i64; 16],
    persist_len: usize,
}

impl Dataflow {
    fn new() -> Self {
        Self { fold_state: 0, persist_buf: [0; 16], persist_len: 0 }
    }

    async fn run_tick(&mut self, input: &[i64]) {
        // Stratum 0
        for &v in input {
            self.fold_state += v;
            self.persist_buf[self.persist_len] = v;
            self.persist_len += 1;
        }

        // Local ref buffer (same as closure approach)
        let fold_ref = &self.fold_state;
        let persist_slice = &self.persist_buf[..self.persist_len];

        // Stratum 1
        async {}.await; // await point

        for item in persist_slice {
            println!("item={}, fold={}", item, fold_ref);
        }
    }
}

#[tokio::main]
async fn main() {
    let mut df = Dataflow::new();
    df.run_tick(&[1, 2, 3]).await;
    println!("---");
    df.run_tick(&[4]).await;
}
