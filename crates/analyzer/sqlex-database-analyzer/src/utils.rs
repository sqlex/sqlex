use sqlex_analyzer::{AnalyzerError, Result};
use tokio::time::{Duration, sleep};

pub async fn parse_retry_connect<F, Fut, P>(connect_fn: F) -> Result<P>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = std::result::Result<P, sqlx::Error>>,
{
    let mut attempts = 0;
    loop {
        match connect_fn().await {
            Ok(pool) => return Ok(pool),
            Err(e) => {
                if attempts >= 10 {
                    return Err(AnalyzerError::ExecutionError(format!(
                        "Failed to connect after 10 attempts: {}",
                        e
                    )));
                }
                attempts += 1;
                sleep(Duration::from_millis(1000)).await;
            },
        }
    }
}
