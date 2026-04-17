use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    models::{Book, CreateBookDto},
    repository,
};

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

pub async fn delete_book(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    match repository::delete_book(&pool, id).await {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "Book not found".to_string()))
            }
        }
        Err(error) => {
            let error_message = format!("Failed to delete book: {}", error);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}

pub async fn update_book(
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateBookDto>,
) -> Result<Json<Book>, (StatusCode, String)> {
    match repository::update_book(&pool, id, payload).await {
        Ok(Some(book)) => Ok(Json(book)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Book not found".to_string())),
        Err(error) => {
            let error_message = format!("Failed to update book: {}", error);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}
