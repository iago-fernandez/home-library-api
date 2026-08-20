mod auth;
mod handlers;
mod integration;
mod models;
mod repository;

use axum::{extract::DefaultBodyLimit,

    routing::{delete, get, post, put},
    Router,
};
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::time::Duration;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tower_http::compression::CompressionLayer;
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
        .max_connections(50)
        .acquire_timeout(Duration::from_secs(30))
        .connect(&database_url)
        .await
        .expect("Failed to create pool");

    // Run migrations automatically on startup
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        if args[1] == "create-user" && args.len() == 4 {
            let username = &args[2];
            let password = &args[3];
            let hash = auth::hash_password(password).expect("Hashing failed");
            let user_id: uuid::Uuid = sqlx::query_scalar("INSERT INTO users (username, password_hash) VALUES ($1, $2) RETURNING id")
                .bind(username)
                .bind(&hash)
                .fetch_one(&pool)
                .await
                .expect("Failed to insert user (maybe username already exists?)");
            
            sqlx::query("INSERT INTO libraries (name, owner_id) VALUES ('Main Library', $1)")
                .bind(user_id)
                .execute(&pool)
                .await
                .expect("Failed to create default library");
                
            println!("User '{}' created successfully with ID: {}", username, user_id);
            return;
        } else if args[1] == "update-password" && args.len() == 4 {
            let username = &args[2];
            let password = &args[3];
            let hash = auth::hash_password(password).expect("Hashing failed");
            let rows = sqlx::query("UPDATE users SET password_hash = $1 WHERE username = $2")
                .bind(&hash)
                .bind(username)
                .execute(&pool)
                .await
                .expect("Database error");
            if rows.rows_affected() > 0 {
                println!("Password updated for user '{}'", username);
            } else {
                println!("User '{}' not found", username);
            }
            return;
        } else {
            println!("Usage:");
            println!("  cargo run -- create-user <username> <password>");
            println!("  cargo run -- update-password <username> <new_password>");
            return;
        }
    }

    tokio::fs::create_dir_all("uploads").await.unwrap();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            axum::http::header::ACCEPT,
        ]);

    let app = Router::new()

        .route("/auth/login", post(handlers::login))
        .route("/api/users/me", put(handlers::update_profile))
        .route("/api/books", get(handlers::get_all_books).post(handlers::create_book))
        .route("/api/books/{id}", delete(handlers::delete_book).put(handlers::update_book).patch(handlers::patch_book))
        .route("/api/books/batch-delete", post(handlers::delete_books_batch))
        .route("/api/libraries", get(handlers::get_libraries).post(handlers::create_library))
        .route("/api/libraries/{id}", put(handlers::update_library).delete(handlers::delete_library))
        .route("/api/libraries/{id}/members", get(handlers::get_library_members).post(handlers::add_library_member))
        .route("/api/libraries/{id}/members/{user_id}", delete(handlers::remove_library_member))
        .route("/api/lookup/metadata/{identifier}", get(handlers::lookup_metadata))
        .route("/api/lookup/search", get(handlers::search_metadata))
        .route("/api/lookup/autocomplete", get(handlers::get_autocomplete))
        .route("/api/upload/cover", post(handlers::upload_cover))
        .route("/api/export/csv", post(handlers::export_csv))
        .route("/api/export/xml", post(handlers::export_xml))
        .route("/api/export/pdf", post(handlers::export_pdf))
        .nest_service("/static", ServeDir::new("uploads"))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .layer(CompressionLayer::new())
        .layer(DefaultBodyLimit::disable())
        .with_state(pool);

    let port = env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{}", port);
    tracing::info!("listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}