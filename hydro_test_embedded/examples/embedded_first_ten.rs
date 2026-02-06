#[cfg(feature = "test_embedded")]
#[tokio::main]
async fn main() {
    let mut flow = hydro_test_embedded::embedded::first_ten();
    tokio::task::LocalSet::new().run_until(flow.run()).await;
}

#[cfg(not(feature = "test_embedded"))]
fn main() {
    eprintln!("This example requires the `test_embedded` feature.");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use std::io::Read;
    use std::process::{Command, Stdio};

    #[test]
    fn test_embedded_first_ten() {
        let child = Command::new("cargo")
            .args([
                "run",
                "--frozen",
                "-p",
                "hydro_test_embedded",
                "--example",
                "embedded_first_ten",
                "--features",
                "test_embedded",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to spawn cargo run");

        let output = child.wait_with_output().expect("failed to wait on child");

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        assert!(
            output.status.success(),
            "example failed with status {}.\nstdout:\n{}\nstderr:\n{}",
            output.status,
            stdout,
            stderr,
        );

        let lines: Vec<&str> = stdout.lines().collect();
        let expected: Vec<String> = (0..10).map(|i| i.to_string()).collect();
        assert_eq!(
            lines, expected,
            "expected first 10 numbers, got:\n{}",
            stdout
        );
    }
}
