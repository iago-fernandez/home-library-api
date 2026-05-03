use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    integration,
    models::{Book, BookFilterQuery, BookMetadataResponse, CreateBookDto, PaginatedBooks},
    repository,
};

pub async fn get_all_books(
    State(pool): State<PgPool>,
    Query(filters): Query<BookFilterQuery>,
) -> Result<Json<PaginatedBooks>, (StatusCode, String)> {
    match repository::fetch_books(&pool, filters).await {
        Ok(paginated) => Ok(Json(paginated)),
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

pub async fn lookup_metadata(
    Path(identifier): Path<String>,
) -> Result<Json<BookMetadataResponse>, (StatusCode, String)> {
    match integration::fetch_metadata(&identifier).await {
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

pub async fn delete_books_batch(
    State(pool): State<PgPool>,
    Json(payload): Json<crate::models::BatchDeleteRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match repository::delete_books_batch(&pool, payload.ids).await {
        Ok(rows_affected) => {
            if rows_affected > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err((StatusCode::NOT_FOUND, "No books found to delete".to_string()))
            }
        }
        Err(error) => {
            let error_message = format!("Failed to delete books in batch: {}", error);
            Err((StatusCode::INTERNAL_SERVER_ERROR, error_message))
        }
    }
}

#[derive(Serialize)]
pub struct UploadResponse {
    pub url: String,
}

pub async fn upload_cover(
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    while let Some(field) = multipart.next_field().await.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            format!("Error processing multipart: {}", e),
        )
    })? {
        if field.name() == Some("cover") {
            let file_uuid = Uuid::new_v4();
            let file_ext = "jpg";
            let file_name = format!("{}.{}", file_uuid, file_ext);
            let file_path = format!("uploads/{}", file_name);

            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read file data: {}", e),
                )
            })?;

            if data.len() > 5 * 1024 * 1024 {
                return Err((StatusCode::PAYLOAD_TOO_LARGE, "File too large. Limit is 5MB".to_string()));
            }

            let mut file = File::create(&file_path).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create file: {}", e),
                )
            })?;

            file.write_all(&data).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to write file: {}", e),
                )
            })?;

            let url = format!("/static/{}", file_name);

            return Ok(Json(UploadResponse { url }));
        }
    }

    Err((StatusCode::BAD_REQUEST, "No cover field found in multipart form".to_string()))
}