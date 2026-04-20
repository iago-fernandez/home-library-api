use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    integration,
    models::{Book, BookFilterQuery, BookMetadataResponse, CreateBookDto},
    repository,
};

pub async fn get_all_books(
    State(pool): State<PgPool>,
    Query(filters): Query<BookFilterQuery>,
) -> Result<Json<Vec<Book>>, (StatusCode, String)> {
    match repository::fetch_books(&pool, filters).await {
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

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
}

pub async fn lookup_metadata_by_isbn(
    Path(isbn): Path<String>,
) -> Result<Json<BookMetadataResponse>, (StatusCode, String)> {
    match integration::fetch_metadata_by_isbn(&isbn).await {
        Ok(metadata) => Ok(Json(metadata)),
        Err(_) => Err((
            StatusCode::BAD_GATEWAY,
            "Failed to connect to metadata provider".to_string(),
        )),
    }
}

pub async fn search_metadata(
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<BookMetadataResponse>>, (StatusCode, String)> {
    match integration::search_metadata_by_query(&query.q).await {
        Ok(results) => Ok(Json(results)),
        Err(_) => Err((
            StatusCode::BAD_GATEWAY,
            "Failed to connect to metadata provider".to_string(),
        )),
    }
}
