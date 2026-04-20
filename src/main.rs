mod handlers;
mod integration;
mod models;
mod repository;

use axum::{
    Router,
    routing::{delete, get},
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to Postgres");

    let app = Router::new()
        .route("/health", get(health_check))
        .route(
            "/books",
            get(handlers::get_all_books).post(handlers::create_book),
        )
        .route(
            "/books/:id",
            delete(handlers::delete_book).put(handlers::update_book),
        )
        .route(
            "/books/lookup/:isbn",
            get(handlers::lookup_metadata_by_isbn),
        )
        .route("/books/search-metadata", get(handlers::search_metadata))
        .with_state(pool);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn health_check() -> &'static str {
    "OK"
}
