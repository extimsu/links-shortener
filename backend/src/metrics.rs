use std::time::{Duration, Instant};
use tracing::{info, warn};

pub struct Timer {
    start: Instant,
    operation: String,
    threshold_warn: Duration,
    threshold_error: Duration,
}

impl Timer {
    pub fn new(operation: impl Into<String>) -> Self {
        Self {
            start: Instant::now(),
            operation: operation.into(),
            threshold_warn: Duration::from_millis(100),
            threshold_error: Duration::from_millis(500),
        }
    }
    pub fn with_thresholds(mut self, warn_ms: u64, error_ms: u64) -> Self {
        self.threshold_warn = Duration::from_millis(warn_ms);
        self.threshold_error = Duration::from_millis(error_ms);
        self
    }
    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }
    pub fn log_elapsed(&self, additional_context: Option<&str>) {
        let elapsed = self.elapsed();
        let elapsed_ms = elapsed.as_millis();
        let context = if let Some(ctx) = additional_context {
            format!("{} ({})", self.operation, ctx)
        } else {
            self.operation.clone()
        };
        if elapsed > self.threshold_error {
            warn!(operation = %context, duration_ms = %elapsed_ms, "Operation exceeded error threshold");
        } else if elapsed > self.threshold_warn {
            warn!(operation = %context, duration_ms = %elapsed_ms, "Operation exceeded warning threshold");
        } else {
            info!(operation = %context, duration_ms = %elapsed_ms, "Operation completed");
        }
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        self.log_elapsed(None);
    }
}

pub async fn time_db_operation<F, T, E>(operation: &str, collection: &str, f: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    let timer = Timer::new(format!("db::{}", operation)).with_thresholds(50, 200);
    let span = crate::tracing::create_db_span(operation, collection);
    let _guard = span.enter();
    let result = f.await;
    timer.log_elapsed(Some(collection));
    result
}

pub async fn time_http_request<F, T, E>(method: &str, path: &str, f: F) -> Result<T, E>
where
    F: std::future::Future<Output = Result<T, E>>,
{
    let timer = Timer::new(format!("http::{}", method)).with_thresholds(200, 1000);
    let result = f.await;
    timer.log_elapsed(Some(path));
    result
}
