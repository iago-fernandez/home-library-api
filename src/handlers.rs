use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    body::Body,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::io::Cursor;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    integration,
    models::{Book, BookFilterQuery, BookMetadataResponse, CreateBookDto, PaginatedBooks, ExportRequest},
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
                Err((
                    StatusCode::NOT_FOUND,
                    "No books found to delete".to_string(),
                ))
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
            let data = field.bytes().await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to read file data: {}", e),
                )
            })?;

            if data.len() > 5 * 1024 * 1024 {
                return Err((
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "File too large. Limit is 5MB".to_string(),
                ));
            }

            let data_vec = data.to_vec();

            let webp_data = tokio::task::spawn_blocking(move || {
                let img = image::load_from_memory(&data_vec)
                    .map_err(|_| "Invalid image data or unsupported format")?;

                let img = img.resize(600, 900, image::imageops::FilterType::Lanczos3);

                let mut buffer = Cursor::new(Vec::new());
                img.write_to(&mut buffer, image::ImageFormat::WebP)
                    .map_err(|_| "Failed to encode image to WebP")?;

                Ok::<Vec<u8>, &'static str>(buffer.into_inner())
            })
                .await
                .map_err(|_| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Image processing thread panicked".to_string(),
                    )
                })?
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

            let file_uuid = Uuid::new_v4().to_string();
            let dir1 = &file_uuid[0..2];
            let dir2 = &file_uuid[2..4];
            let file_name = format!("{}.webp", file_uuid);

            let relative_dir = format!("uploads/{}/{}", dir1, dir2);
            tokio::fs::create_dir_all(&relative_dir)
                .await
                .map_err(|e| {
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to create directories: {}", e),
                    )
                })?;

            let file_path = format!("{}/{}", relative_dir, file_name);
            let mut file = File::create(&file_path).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to create file: {}", e),
                )
            })?;

            file.write_all(&webp_data).await.map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to write file: {}", e),
                )
            })?;

            let url = format!("/static/{}/{}/{}", dir1, dir2, file_name);

            return Ok(Json(UploadResponse { url }));
        }
    }

    Err((
        StatusCode::BAD_REQUEST,
        "No cover field found in multipart form".to_string(),
    ))
}

pub async fn export_csv(
    State(pool): State<PgPool>,
    Json(payload): Json<ExportRequest>,
) -> impl IntoResponse {
    let books_result = repository::fetch_all_for_export(&pool, &payload.filters, payload.specific_ids).await;

    let requested_columns: Vec<String> = if payload.columns.is_empty() {
        vec!["id".to_string(), "title".to_string(), "authors".to_string()]
    } else {
        payload.columns.into_iter().filter(|c| c != "cover_url").collect()
    };

    let mut csv_data = String::from("\u{FEFF}");
    csv_data.push_str(&requested_columns.join(";"));
    csv_data.push('\n');

    if let Ok(books) = books_result {
        for book in books {
            let json_val = serde_json::to_value(&book).unwrap_or(serde_json::json!({}));
            let mut row_values = Vec::new();

            for col in &requested_columns {
                let cell_val = match json_val.get(col) {
                    Some(serde_json::Value::String(s)) => s.to_string(),
                    Some(serde_json::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>().join(", "),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::Bool(b)) => if *b { "Yes".to_string() } else { "No".to_string() },
                    _ => "".to_string(),
                }.replace("\"", "\"\"");

                row_values.push(format!("\"{}\"", cell_val));
            }
            csv_data.push_str(&row_values.join(";"));
            csv_data.push('\n');
        }
    }

    (
        [
            (axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"library_export.csv\""),
        ],
        Body::from(csv_data),
    )
}

pub async fn export_xml(
    State(pool): State<PgPool>,
    Json(payload): Json<ExportRequest>,
) -> impl IntoResponse {
    let books_result = repository::fetch_all_for_export(&pool, &payload.filters, payload.specific_ids).await;

    let requested_columns: Vec<String> = if payload.columns.is_empty() {
        vec!["id".to_string(), "title".to_string(), "authors".to_string()]
    } else {
        payload.columns.into_iter().filter(|c| c != "cover_url").collect()
    };

    let mut xml_data = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<books>\n");

    if let Ok(books) = books_result {
        for book in books {
            let json_val = serde_json::to_value(&book).unwrap_or(serde_json::json!({}));
            xml_data.push_str("  <book>\n");

            for col in &requested_columns {
                let cell_val = match json_val.get(col) {
                    Some(serde_json::Value::String(s)) => s.to_string(),
                    Some(serde_json::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>().join(", "),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::Bool(b)) => if *b { "Yes".to_string() } else { "No".to_string() },
                    _ => "".to_string(),
                };

                let safe_val = cell_val.replace("<", "&lt;").replace(">", "&gt;").replace("&", "&amp;");
                xml_data.push_str(&format!("    <{}>{}</{}>\n", col, safe_val, col));
            }
            xml_data.push_str("  </book>\n");
        }
    }

    xml_data.push_str("</books>\n");

    (
        [
            (axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8"),
            (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"library_export.xml\""),
        ],
        Body::from(xml_data),
    )
}

pub async fn export_pdf(
    State(pool): State<PgPool>,
    Json(payload): Json<ExportRequest>,
) -> impl IntoResponse {
    let books_result = repository::fetch_all_for_export(&pool, &payload.filters, payload.specific_ids).await;

    let requested_columns: Vec<String> = if payload.columns.is_empty() {
        vec!["title".to_string(), "authors".to_string(), "publish_date".to_string()]
    } else {
        payload.columns.into_iter().filter(|c| c != "cover_url").collect()
    };

    let font_family = match genpdf::fonts::from_files("fonts", "Roboto", None) {
        Ok(f) => f,
        Err(_) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            "Font directory 'fonts' missing".to_string()
        ).into_response(),
    };

    let mut doc = genpdf::Document::new(font_family);

    let landscape_size = genpdf::Size {
        width: genpdf::Mm::from(297.0),
        height: genpdf::Mm::from(210.0),
    };
    doc.set_paper_size(landscape_size);

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(5);
    doc.set_page_decorator(decorator);
    doc.set_font_size(6);

    let mut table = genpdf::elements::TableLayout::new(vec![1; requested_columns.len()]);
    table.set_cell_decorator(genpdf::elements::FrameCellDecorator::new(true, true, false));

    let mut header_row = table.row();
    for col in &requested_columns {
        let label = col.replace("_", " ").to_uppercase();
        header_row.push_element(genpdf::elements::Paragraph::new(label));
    }
    header_row.push().unwrap_or_default();

    if let Ok(books) = books_result {
        for book in books {
            let json_val = serde_json::to_value(&book).unwrap_or(serde_json::json!({}));
            let mut row = table.row();

            for col in &requested_columns {
                let cell_val = match json_val.get(col) {
                    Some(serde_json::Value::String(s)) => s.to_string(),
                    Some(serde_json::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>().join(", "),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::Bool(b)) => if *b { "Yes".to_string() } else { "No".to_string() },
                    _ => "".to_string(),
                };
                row.push_element(genpdf::elements::Paragraph::new(cell_val));
            }
            row.push().unwrap_or_default();
        }
    }

    doc.push(table);

    let mut buffer = Cursor::new(Vec::new());
    match doc.render(&mut buffer) {
        Ok(_) => {
            (
                [
                    (axum::http::header::CONTENT_TYPE, "application/pdf"),
                    (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"library_export.pdf\""),
                ],
                Body::from(buffer.into_inner()),
            ).into_response()
        },
        Err(_) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to render PDF".to_string()
            ).into_response()
        }
    }
}