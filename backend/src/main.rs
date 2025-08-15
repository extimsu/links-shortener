use actix_web::{web, App, HttpResponse, HttpServer, Responder, Result};
use mongodb::{bson::{doc, oid::ObjectId, DateTime as MongoDateTime}, Client, Collection, options::{IndexOptions, IndexModel}};
use serde::{Deserialize, Serialize};
use dotenvy::dotenv;
use std::env;
use rand::{distributions::Alphanumeric, Rng};

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
    short_code: String,
}

#[derive(Serialize)]
struct AnalyticsResponse {
    short_code: String,
    transition_count: i64,
}

async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("OK")
}

async fn db_health(client: web::Data<Client>) -> impl Responder {
    match client.database("admin").run_command(doc! {"ping": 1}, None).await {
        Ok(_) => HttpResponse::Ok().body("DB OK"),
        Err(e) => HttpResponse::InternalServerError().body(format!("DB Error: {}", e)),
    }
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
) -> Result<HttpResponse> {
    let short_code: String = rand::thread_rng()
        .sample_iter(&Alphanumeric)
        .take(7)
        .map(char::from)
        .collect();
    let url_doc = UrlDoc {
        id: None,
        short_code: short_code.clone(),
        original_url: req.url.clone(),
        created_at: MongoDateTime::now(),
        transition_count: 0,
    };
    let collection: Collection<UrlDoc> = client.database("shortener").collection("urls");
    let insert_result = collection.insert_one(&url_doc, None).await;
    match insert_result {
        Ok(_) => Ok(HttpResponse::Ok().json(ShortenResponse { short_code })),
        Err(e) => {
            if let Some(11000) = e.code() {
                // Duplicate key error
                Ok(HttpResponse::Conflict().body("Short code already exists. Please try again."))
            } else {
                Err(actix_web::error::ErrorInternalServerError(format!("Insert Error: {}", e)))
            }
        }
    }
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
        Ok(HttpResponse::Ok().json(AnalyticsResponse {
            short_code: url_doc.short_code,
            transition_count: url_doc.transition_count,
        }))
    } else {
        Ok(HttpResponse::NotFound().body("Short URL not found"))
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    let mongo_uri = env::var("MONGODB_URI").unwrap_or_else(|_| "mongodb://mongo:27017".to_string());
    let client = Client::with_uri_str(&mongo_uri).await.expect("Failed to connect to MongoDB");
    ensure_indexes(&client).await;
    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(client.clone()))
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
