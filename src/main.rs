mod auth;
mod handlers;
mod integration;
mod models;
mod repository;

use axum::{
    routing::{delete, get, patch, post, put},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    dotenv().ok();

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "home_library_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .acquire_timeout(Duration::from_secs(3))
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    tokio::fs::create_dir_all("uploads").await.unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/api/users/me", put(handlers::update_profile))
        .route("/api/books", get(handlers::get_all_books).post(handlers::create_book))
        .route("/api/books/{id}", delete(handlers::delete_book).put(handlers::update_book).patch(handlers::patch_book))
        .route("/api/books/batch-delete", post(handlers::delete_books_batch))
        .route("/api/lookup/metadata/{identifier}", get(handlers::lookup_metadata))
        .route("/api/lookup/search", get(handlers::search_metadata))
        .route("/api/upload/cover", post(handlers::upload_cover))
        .route("/api/export/csv", post(handlers::export_csv))
        .route("/api/export/xml", post(handlers::export_xml))
        .route("/api/export/pdf", post(handlers::export_pdf))
        .nest_service("/static", ServeDir::new("uploads"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(pool);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}