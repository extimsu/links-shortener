mod logging;
mod tracing;
use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use mongodb::{bson::{doc, oid::ObjectId, DateTime as MongoDateTime}, Client, Collection, options::IndexOptions, IndexModel};
use serde::{Deserialize, Serialize};
use std::env;
use rand::{distributions::Alphanumeric, Rng};
use mongodb::options::{ClientOptions, ServerApi, ServerApiVersion};
use url_service::{UrlService, UrlServiceError};
use actix_cors::Cors;
use tracing_actix_web::TracingLogger;
use log::error;
use chrono::{Utc, DateTime, NaiveDateTime};
mod url_service;

#[derive(Debug, Serialize, Deserialize, Clone)]
struct UrlDoc {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    short_code: String,
    original_url: String,
    created_at: MongoDateTime,
    transition_count: i64,
}

/// Represents an analytics record for URL access statistics
#[derive(Debug, Serialize, Deserialize, Clone)]
struct AnalyticsDoc {
    /// MongoDB ObjectId, auto-generated if None during insertion
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    id: Option<ObjectId>,
    /// Reference to the associated URL document
    url_id: ObjectId,
    /// Number of times the short URL has been accessed
    #[serde(default)]
    transition_count: i64,
    /// Timestamp of the most recent access
    last_accessed: Option<MongoDateTime>,
}

#[derive(Deserialize)]
struct ShortenRequest {
    url: String,
}

#[derive(Serialize)]
struct ShortenResponse {
    short_url: String,
    original_url: String,
    created_at: String,
}

#[derive(Serialize)]
struct AnalyticsResponse {
    short_code: String,
    original_url: String,
    created_at: String,
    transition_count: i64,
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

async fn db_health(client: web::Data<Client>) -> impl Responder {
    // REMOVE admin database ping
    HttpResponse::Ok().body("DB OK")
}

async fn ensure_indexes(client: &Client) {
    // Unique index on short_code in urls collection (prevents duplicate short codes)
    let collection: Collection<UrlDoc> = client.database("shortener").collection("urls");
    let index_model = IndexModel::builder()
        .keys(doc! {"short_code": 1})
        .options(IndexOptions::builder().unique(true).build())
        .build();
    let _ = collection.create_index(index_model, None).await;

    // Indexes for analytics collection
    let analytics_collection: Collection<AnalyticsDoc> = client.database("shortener").collection("analytics");
    // Index on url_id for fast lookup of analytics by URL
    let url_id_index = IndexModel::builder()
        .keys(doc! {"url_id": 1})
        .options(None)
        .build();
    let _ = analytics_collection.create_index(url_id_index, None).await;

    // Compound index on url_id and last_accessed for efficient queries on analytics by URL and recency
    let compound_index = IndexModel::builder()
        .keys(doc! {"url_id": 1, "last_accessed": -1})
        .options(None)
        .build();
    let _ = analytics_collection.create_index(compound_index, None).await;
}

async fn shorten_url(
    client: web::Data<Client>,
    req: web::Json<ShortenRequest>,
    http_req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    // --- Integrate advanced validation and normalization ---
    let url_service = UrlService::new_dummy();
    if let Err(e) = url_service.validate_url(&req.url) {
        println!("URL validation failed: {:?}", e);
        return Ok(HttpResponse::BadRequest().body(format!("Invalid URL: {}", e)));
    }
    let normalized_url = match url_service.normalize_url(&req.url) {
        Ok(url) => url,
        Err(e) => {
            println!("URL normalization failed: {:?}", e);
            return Ok(HttpResponse::BadRequest().body(format!("URL normalization failed: {}", e)));
        }
    };
    let collection: Collection<UrlDoc> = client.database("shortener").collection("urls");
    // Check if a short link already exists for this normalized URL
    if let Some(existing) = collection.find_one(doc! {"original_url": &normalized_url}, None).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Query Error: {}", e))
    })? {
        let host = http_req.connection_info().host().to_string();
        let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") { "http" } else { "https" };
        let short_url = format!("{}://{}/{}", scheme, host, existing.short_code);
        let created_at_rfc3339 = DateTime::<Utc>::from_timestamp_millis(existing.created_at.timestamp_millis()).unwrap().to_rfc3339();
        return Ok(HttpResponse::Ok().json(ShortenResponse {
            short_url,
            original_url: normalized_url,
            created_at: created_at_rfc3339,
        }));
    }
    // --- End integration ---
    let mut last_err = None;
    for _ in 0..5 {
        let short_code: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let now = MongoDateTime::now();
        let url_doc = UrlDoc {
            id: None,
            short_code: short_code.clone(),
            original_url: normalized_url.clone(),
            created_at: now,
            transition_count: 0,
        };
        let insert_result = collection.insert_one(&url_doc, None).await;
        match insert_result {
            Ok(_) => {
                // Build the full short URL using the host header
                let host = http_req.connection_info().host().to_string();
                let scheme = if host.starts_with("localhost") || host.starts_with("127.0.0.1") { "http" } else { "https" };
                let short_url = format!("{}://{}/{}", scheme, host, short_code);
                let created_at_rfc3339 = DateTime::<Utc>::from_timestamp_millis(now.timestamp_millis()).unwrap().to_rfc3339();
                return Ok(HttpResponse::Ok().json(ShortenResponse {
                    short_url,
                    original_url: normalized_url.clone(),
                    created_at: created_at_rfc3339,
                }));
            }
            Err(e) => {
                let err_str = format!("{}", e);
                if err_str.contains("E11000") || err_str.contains("duplicate key error") {
                    // Collision, retry
                    last_err = Some(e);
                    continue;
                } else {
                    return Err(actix_web::error::ErrorInternalServerError(format!("Insert Error: {}", e)));
                }
            }
        }
    }
    // If we get here, all attempts failed due to collisions
    Err(actix_web::error::ErrorInternalServerError(format!("Failed to generate unique short code after 5 attempts: {:?}", last_err)))
}

async fn redirect_short_url(
    client: web::Data<Client>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let short_code = path.into_inner();
    let collection: Collection<UrlDoc> = client.database("shortener").collection("urls");
    let filter = doc! {"short_code": &short_code};
    if let Some(mut url_doc) = collection.find_one(filter.clone(), None).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Query Error: {}", e))
    })? {
        // Increment transition count
        url_doc.transition_count += 1;
        collection.update_one(
            doc! {"_id": &url_doc.id},
            doc! {"$set": {"transition_count": url_doc.transition_count}},
            None,
        ).await.ok();
        Ok(HttpResponse::Found().append_header(("Location", url_doc.original_url)).finish())
    } else {
        Ok(HttpResponse::NotFound().body("Short URL not found"))
    }
}

async fn analytics(
    client: web::Data<Client>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let short_code = path.into_inner();
    let collection: Collection<UrlDoc> = client.database("shortener").collection("urls");
    let filter = doc! {"short_code": &short_code};
    if let Some(url_doc) = collection.find_one(filter, None).await.map_err(|e| {
        actix_web::error::ErrorInternalServerError(format!("Query Error: {}", e))
    })? {
        let created_at_rfc3339 = DateTime::<Utc>::from_timestamp_millis(url_doc.created_at.timestamp_millis()).unwrap().to_rfc3339();
        Ok(HttpResponse::Ok().json(AnalyticsResponse {
            short_code: url_doc.short_code,
            original_url: url_doc.original_url,
            created_at: created_at_rfc3339,
            transition_count: url_doc.transition_count,
        }))
    } else {
        Ok(HttpResponse::NotFound().body("Short URL not found"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    logging::set_panic_hook();
    logging::init_logging_with_fallback();
    if let Err(e) = tracing::init_tracer() {
        error!("Failed to initialize tracer: {:?}", e);
    }
    dotenvy::dotenv().ok();
    let mongo_uri = env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://mongo:27017".to_string());
    // Pool settings from environment variables (with defaults)
    let max_pool_size = env::var("MONGODB_MAX_POOL_SIZE").ok().and_then(|v| v.parse().ok()).unwrap_or(20);
    let min_pool_size = env::var("MONGODB_MIN_POOL_SIZE").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
    let max_idle_time = env::var("MONGODB_MAX_IDLE_TIME_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(300000); // ms
    let connect_timeout = env::var("MONGODB_CONNECT_TIMEOUT_MS").ok().and_then(|v| v.parse().ok()).unwrap_or(10000); // ms
    // Configure client options with pooling
    let mut client_options = ClientOptions::parse(&mongo_uri).await.expect("Failed to parse MongoDB URI");
    client_options.max_pool_size = Some(max_pool_size);
    client_options.min_pool_size = Some(min_pool_size);
    client_options.max_idle_time = Some(std::time::Duration::from_millis(max_idle_time));
    client_options.connect_timeout = Some(std::time::Duration::from_millis(connect_timeout));
    // Optionally set server API version for compatibility
    client_options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());
    // The MongoDB Client object manages a pool of connections automatically
    let client = Client::with_options(client_options).expect("Failed to connect to MongoDB");
    ensure_indexes(&client).await;
    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin("http://localhost:3000")
            .allowed_origin("http://frontend:80")
            .allow_any_method()
            .allow_any_header()
            .supports_credentials();
        App::new()
            .wrap(cors)
            .wrap(TracingLogger::default())
            .wrap(logging::RequestIdMiddleware)
            .app_data(web::Data::new(client.clone()))
            // REMOVE all /api/admin routes and admin_auth middleware
            .route("/health", web::get().to(health_check))
            .route("/db_health", web::get().to(db_health))
            .route("/api/shorten", web::post().to(shorten_url))
            .route("/api/analytics/{short_code}", web::get().to(analytics))
            .route("/{short_code}", web::get().to(redirect_short_url))
    })
    .bind(("0.0.0.0", 8080))?
    .run()
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use actix_web::{test, web, App};
    use mongodb::{Client, options::ClientOptions};
    use serde_json::json;
    use std::env;

    #[actix_rt::test]
    async fn test_shorten_valid_url() {
        let mongo_uri = env::var("MONGODB_TEST_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
        let mut client_options = ClientOptions::parse(&mongo_uri).await.expect("Failed to parse MongoDB URI");
        let client = Client::with_options(client_options).expect("Failed to connect to MongoDB");
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(client))
                .route("/api/shorten", web::post().to(shorten_url))
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/shorten")
            .set_json(&json!({"url": "https://example.com"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = test::read_body(resp).await;
            println!("test_shorten_valid_url failed: status = {:?}, body = {:?}", status, body);
            panic!("test_shorten_valid_url failed");
        }
        assert!(true);
    }

    #[actix_rt::test]
    async fn test_shorten_invalid_url_format() {
        let mongo_uri = env::var("MONGODB_TEST_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
        let mut client_options = ClientOptions::parse(&mongo_uri).await.expect("Failed to parse MongoDB URI");
        let client = Client::with_options(client_options).expect("Failed to connect to MongoDB");
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(client))
                .route("/api/shorten", web::post().to(shorten_url))
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/shorten")
            .set_json(&json!({"url": "not_a_url"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_rt::test]
    async fn test_shorten_disallowed_domain() {
        let mongo_uri = env::var("MONGODB_TEST_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
        let mut client_options = ClientOptions::parse(&mongo_uri).await.expect("Failed to parse MongoDB URI");
        let client = Client::with_options(client_options).expect("Failed to connect to MongoDB");
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(client))
                .route("/api/shorten", web::post().to(shorten_url))
        ).await;
        let req = test::TestRequest::post()
            .uri("/api/shorten")
            .set_json(&json!({"url": "http://localhost"}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }

    #[actix_rt::test]
    async fn test_shorten_url_too_long() {
        let mongo_uri = env::var("MONGODB_TEST_URI").unwrap_or_else(|_| "mongodb://localhost:27017".to_string());
        let mut client_options = ClientOptions::parse(&mongo_uri).await.expect("Failed to parse MongoDB URI");
        let client = Client::with_options(client_options).expect("Failed to connect to MongoDB");
        let app = test::init_service(
            App::new()
                .app_data(web::Data::new(client))
                .route("/api/shorten", web::post().to(shorten_url))
        ).await;
        let long_url = format!("http://{}", "a".repeat(2050));
        let req = test::TestRequest::post()
            .uri("/api/shorten")
            .set_json(&json!({"url": long_url}))
            .to_request();
        let resp = test::call_service(&app, req).await;
        assert_eq!(resp.status(), 400);
    }
}
