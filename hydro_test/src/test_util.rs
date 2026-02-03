/// Test utilities for hydro_test integration tests
use tokio::sync::mpsc::UnboundedReceiver;

/// Skip tracing logs and return the next application output message.
///
/// This helper function reads from a stdout receiver and skips over any tracing logs
/// (which start with timestamps like "2026-02-02T..." and contain log level keywords).
/// It returns the first non-tracing message it encounters.
///
/// # Arguments
/// * `stdout` - A mutable reference to an UnboundedReceiver<String>
///
/// # Returns
/// The next application output message (non-tracing log)
///
/// # Panics
/// Panics if the receiver is closed or if receiving fails
pub async fn skip_tracing_logs(stdout: &mut UnboundedReceiver<String>) -> String {
    let mut message = stdout.recv().await.unwrap();
    while message.starts_with("20")
        && (message.contains("TRACE")
            || message.contains("INFO")
            || message.contains("DEBUG")
            || message.contains("WARN")
            || message.contains("ERROR"))
    {
        message = stdout.recv().await.unwrap();
    }
    message
}

#[cfg(test)]
mod tests {
    use tokio::sync::mpsc;

    use super::*;

    #[tokio::test]
    async fn test_skip_tracing_logs_skips_trace() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send("2026-02-02T04:16:01.263079+00:00 TRACE run_stratum: new".to_string())
            .unwrap();
        tx.send("application output".to_string()).unwrap();

        let result = skip_tracing_logs(&mut rx).await;
        assert_eq!(result, "application output");
    }

    #[tokio::test]
    async fn test_skip_tracing_logs_skips_info() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send("2026-02-02T04:16:01.263079+00:00 INFO some info log".to_string())
            .unwrap();
        tx.send("application output".to_string()).unwrap();

        let result = skip_tracing_logs(&mut rx).await;
        assert_eq!(result, "application output");
    }

    #[tokio::test]
    async fn test_skip_tracing_logs_skips_multiple() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send("2026-02-02T04:16:01.263079+00:00 TRACE trace log".to_string())
            .unwrap();
        tx.send("2026-02-02T04:16:01.263079+00:00 INFO info log".to_string())
            .unwrap();
        tx.send("2026-02-02T04:16:01.263079+00:00 DEBUG debug log".to_string())
            .unwrap();
        tx.send("application output".to_string()).unwrap();

        let result = skip_tracing_logs(&mut rx).await;
        assert_eq!(result, "application output");
    }

    #[tokio::test]
    async fn test_skip_tracing_logs_returns_first_non_tracing() {
        let (tx, mut rx) = mpsc::unbounded_channel();
        tx.send("application output".to_string()).unwrap();

        let result = skip_tracing_logs(&mut rx).await;
        assert_eq!(result, "application output");
    }
}
