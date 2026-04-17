use crate::{
    models::{Book, CreateBookDto},
    repository,
};
use axum::{extract::State, http::StatusCode, Json};
use sqlx::PgPool;

pub async fn get_all_books(
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Book>>, (StatusCode, String)> {
    match repository::fetch_all_books(&pool).await {
        Ok(books) => Ok(Json(books)),
        Err(error) => {
            let error_message = format!("Database error: {}", error);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}

pub async fn create_book(
    State(pool): State<PgPool>,
    Json(payload): Json<CreateBookDto>,
) -> Result<(StatusCode, Json<Book>), (StatusCode, String)> {
    match repository::create_book(&pool, payload).await {
        Ok(book) => Ok((StatusCode::CREATED, Json(book))),
        Err(error) => {
            let error_message = format!("Failed to create book: {}", error);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}
