use actix_web::{dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform}, Error};
use futures::future::{ready, LocalBoxFuture, Ready};
use tracing::{span, info, Level};
use uuid::Uuid;
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};
use std::fs::File;
use actix_web::HttpMessage;

pub fn init_logging() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| {
            EnvFilter::new("url_shortener=info,actix_web=info,actix_http=info")
        });

    let is_dev = std::env::var("APP_ENV").unwrap_or_else(|_| "development".into()) == "development";
    let registry = tracing_subscriber::registry().with(env_filter);

    if is_dev {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .with_level(true)
            .with_line_number(true)
            .with_ansi(true)
            .pretty();
        registry.with(fmt_layer).init();
    } else {
        let file = File::create("/var/log/url_shortener.log").ok();
        if let Some(file) = file {
            let file_writer = std::sync::Mutex::new(file);
            let json_layer = fmt::layer()
                .with_writer(file_writer)
                .with_target(true)
                .with_level(true)
                .with_line_number(true)
                .json();
            registry.with(json_layer).init();
        } else {
            let json_layer = fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_line_number(true)
                .with_ansi(false)
                .json();
            registry.with(json_layer).init();
        }
    }
    tracing::info!("Logging system initialized in {} mode", if is_dev { "development" } else { "production" });
}

pub fn set_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        let backtrace = std::backtrace::Backtrace::capture();
        if let Some(location) = panic_info.location() {
            tracing::error!(
                message = %panic_info,
                file = %location.file(),
                line = %location.line(),
                column = %location.column(),
                backtrace = %format!("{:?}", backtrace),
                "Application panic"
            );
        } else {
            tracing::error!(
                message = %panic_info,
                backtrace = %format!("{:?}", backtrace),
                "Application panic (unknown location)"
            );
        }
        eprintln!("PANIC: {}", panic_info);
        eprintln!("{:?}", backtrace);
    }));
}

pub fn init_logging_with_fallback() {
    if let Err(e) = try_init_logging() {
        eprintln!("Failed to initialize structured logging: {}", e);
        eprintln!("Falling back to simple stderr logging");
        let stderr_layer = tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(std::io::stderr);
        if let Err(e) = tracing_subscriber::registry()
            .with(stderr_layer)
            .try_init() {
            eprintln!("Failed to initialize fallback logging: {}", e);
        }
    }
}

fn try_init_logging() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();
    Ok(())
}

pub struct RequestIdMiddleware;

impl<S, B> Transform<S, ServiceRequest> for RequestIdMiddleware
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = RequestIdMiddlewareService<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(RequestIdMiddlewareService { service }))
    }
}

pub struct RequestIdMiddlewareService<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for RequestIdMiddlewareService<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        let span = span!(
            Level::INFO,
            "request",
            request_id = %request_id,
            method = %req.method(),
            path = %req.path(),
            remote_addr = %req.connection_info().realip_remote_addr().unwrap_or("unknown")
        );
        req.extensions_mut().insert(request_id.clone());
        let fut = self.service.call(req);
        Box::pin(async move {
            let _guard = span.enter();
            info!("Request started");
            let res = fut.await?;
            info!(status = res.status().as_u16(), "Request completed");
            Ok(res)
        })
    }
}
