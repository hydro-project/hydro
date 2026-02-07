#[cfg(feature = "test_embedded")]
#[tokio::main]
async fn main() {
    let input = dfir_rs::futures::stream::iter(vec![
        "hello".to_string(),
        "world".to_string(),
        "hydro".to_string(),
    ]);
    let mut flow = hydro_test_embedded::embedded::capitalize(input);
    tokio::task::LocalSet::new()
        .run_until(flow.run_available())
        .await;
}

#[cfg(not(feature = "test_embedded"))]
fn main() {
    eprintln!("This example requires the `test_embedded` feature.");
    std::process::exit(1);
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    #[test]
    fn test_embedded_capitalize() {
        let output = Command::new("cargo")
            .args([
                "run",
                "--frozen",
                "-p",
                "hydro_test_embedded",
                "--example",
                "embedded_capitalize",
                "--features",
                "test_embedded",
            ])
            .stdout(Stdio::piped())
            .output()
            .expect("failed to spawn cargo run");

        let stdout = String::from_utf8_lossy(&output.stdout);

        assert!(
            output.status.success(),
            "example failed with status {}.\nstdout:\n{}",
            output.status,
            stdout,
        );

        let lines: Vec<&str> = stdout.lines().collect();
        let expected = vec!["HELLO", "WORLD", "HYDRO"];
        assert_eq!(
            lines, expected,
            "expected capitalized strings, got:\n{}",
            stdout
        );
    }
}
