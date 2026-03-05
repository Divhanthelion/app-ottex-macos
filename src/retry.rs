use anyhow::Result;
use std::time::Duration;

/// Retry an async operation with exponential backoff.
///
/// - `max_retries`: maximum number of retry attempts (0 means try once, no retries)
/// - `initial_delay`: delay before the first retry
/// - `should_retry`: predicate on the error to decide if we should retry
pub async fn retry_with_backoff<F, Fut, T, R>(
    max_retries: u32,
    initial_delay: Duration,
    should_retry: R,
    mut f: F,
) -> Result<T>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T>>,
    R: Fn(&anyhow::Error) -> bool,
{
    let mut delay = initial_delay;
    let mut last_err = None;

    for attempt in 0..=max_retries {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                if attempt == max_retries || !should_retry(&e) {
                    return Err(e);
                }
                log::warn!(
                    "Attempt {}/{} failed: {}, retrying in {:?}",
                    attempt + 1,
                    max_retries + 1,
                    e,
                    delay
                );
                last_err = Some(e);
                tokio::time::sleep(delay).await;
                delay *= 2;
            }
        }
    }

    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("retry exhausted")))
}

/// Returns true if the error message suggests a retryable HTTP status.
pub fn is_retryable_error(err: &anyhow::Error) -> bool {
    let msg = err.to_string();
    // Retryable HTTP status codes
    for code in &["429", "500", "502", "503", "504"] {
        if msg.contains(code) {
            return true;
        }
    }
    // Connection errors
    if msg.contains("connection") || msg.contains("timed out") || msg.contains("timeout") {
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_success_on_first_try() {
        let result = retry_with_backoff(
            3,
            Duration::from_millis(10),
            |_| true,
            || async { Ok::<_, anyhow::Error>(42) },
        )
        .await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_success_after_failures() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let result = retry_with_backoff(
            3,
            Duration::from_millis(10),
            |_| true,
            move || {
                let c = c.clone();
                async move {
                    let n = c.fetch_add(1, Ordering::SeqCst);
                    if n < 2 {
                        anyhow::bail!("error 500 from server");
                    }
                    Ok(99)
                }
            },
        )
        .await;
        assert_eq!(result.unwrap(), 99);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_exhaustion() {
        let result = retry_with_backoff(
            2,
            Duration::from_millis(10),
            |_| true,
            || async { Err::<i32, _>(anyhow::anyhow!("always fails 500")) },
        )
        .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_non_retryable_fails_immediately() {
        let counter = Arc::new(AtomicU32::new(0));
        let c = counter.clone();
        let result = retry_with_backoff(
            3,
            Duration::from_millis(10),
            |_| false, // never retry
            move || {
                let c = c.clone();
                async move {
                    c.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>(anyhow::anyhow!("non-retryable"))
                }
            },
        )
        .await;
        assert!(result.is_err());
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn test_is_retryable_error() {
        assert!(is_retryable_error(&anyhow::anyhow!(
            "HTTP error 429: rate limited"
        )));
        assert!(is_retryable_error(&anyhow::anyhow!(
            "error 503 service unavailable"
        )));
        assert!(is_retryable_error(&anyhow::anyhow!("connection reset")));
        assert!(!is_retryable_error(&anyhow::anyhow!("API key invalid 401")));
    }
}
