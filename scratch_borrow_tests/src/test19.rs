//! Test 19: Mutable reference to captured state flowing to downstream.
//! Simulates state() operator where downstream can mutate the lattice.
//! RESULT: ✅ PASS

#[tokio::main]
async fn main() {
    let mut lattice_state: Vec<i64> = Vec::new();

    let mut tick = async move |input: &[i64]| -> bool {
        // Stratum 0: merge into lattice
        {
            for &val in input {
                if !lattice_state.contains(&val) {
                    lattice_state.push(val);
                }
            }
        }

        // Downstream gets &mut to the state
        let state_ref: &mut Vec<i64> = &mut lattice_state;

        // Stratum 1: can read AND mutate
        {
            state_ref.sort();
            println!("sorted state: {:?}", state_ref);
        }

        true
    };

    tick(&[3, 1, 2]).await;
    tick(&[5, 1, 4]).await;
}
