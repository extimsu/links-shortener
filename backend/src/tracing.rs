use opentelemetry::global;
use opentelemetry::sdk::propagation::TraceContextPropagator;
use opentelemetry::sdk::trace::{self, Sampler};
use tracing::info;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

pub fn init_tracer() -> Result<(), Box<dyn std::error::Error>> {
    global::set_text_map_propagator(TraceContextPropagator::new());
    let service_name = std::env::var("SERVICE_NAME").unwrap_or_else(|_| "url-shortener".into());
    let jaeger_endpoint = std::env::var("JAEGER_ENDPOINT").unwrap_or_else(|_| "http://localhost:14268/api/traces".into());
    let tracer = opentelemetry_jaeger::new_agent_pipeline()
        .with_service_name(service_name)
        .with_endpoint(jaeger_endpoint)
        .install_batch(opentelemetry::runtime::Tokio)?;
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    tracing_subscriber::registry().with(telemetry_layer).try_init()?;
    info!("OpenTelemetry tracing initialized");
    Ok(())
}

pub fn create_db_span(operation: &str, collection: &str) -> tracing::Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "database",
        operation = operation,
        collection = collection,
        db_type = "mongodb"
    )
}

pub fn create_http_client_span(method: &str, url: &str) -> tracing::Span {
    tracing::span!(
        tracing::Level::DEBUG,
        "http_client",
        http_method = method,
        http_url = url
    )
}
