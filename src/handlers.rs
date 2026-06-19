use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    body::Body,
};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, FromRow};
use std::io::Cursor;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::{
    auth::{self, Claims},
    integration,
    models::{
        AuthRequest, AuthResponse, BatchDeleteRequest, Book, BookFilterQuery,
        BookMetadataResponse, CreateBookDto, ExportRequest, PaginatedBooks, UserDto,
        UpdateUserDto, UpdateBookPartialDto, CreateLibraryDto, UpdateLibraryDto,
        Library, LibraryMember, ShareLibraryDto,
    },
    repository,
};

static PDF_FONT_FAMILY: std::sync::OnceLock<genpdf::fonts::FontFamily<genpdf::fonts::FontData>> = std::sync::OnceLock::new();

fn format_date(date_str: &str, fmt: Option<&String>) -> String {
    if let Some(format) = fmt {
        let chrono_fmt = match format.as_str() {
            "dd/mm/yyyy hh:mm:ss" => "%d/%m/%Y %H:%M:%S",
            "dd/mm/yyyy" => "%d/%m/%Y",
            "mm/dd/yyyy hh:mm:ss" => "%m/%d/%Y %H:%M:%S",
            "mm/dd/yyyy" => "%m/%d/%Y",
            "yyyy-mm-dd hh:mm:ss" => "%Y-%m-%d %H:%M:%S",
            "yyyy-mm-dd" => "%Y-%m-%d",
            "DD/MM/YYYY" => "%d/%m/%Y",
            "MM/DD/YYYY" => "%m/%d/%Y",
            "YYYY/MM/DD" => "%Y/%m/%d",
            _ => "%Y-%m-%d",
        };

        if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(date_str) {
            return dt.format(chrono_fmt).to_string();
        }
        if let Ok(d) = chrono::NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
            let pure_date_fmt = chrono_fmt.replace(" %H:%M:%S", "");
            return d.format(&pure_date_fmt).to_string();
        }
    }
    date_str.to_string()
}

#[derive(Clone)]
struct CleanTableDecorator {
    num_columns: usize,
    num_rows: usize,
}

impl CleanTableDecorator {
    fn new() -> Self {
        Self { num_columns: 0, num_rows: 0 }
    }
}

impl genpdf::elements::CellDecorator for CleanTableDecorator {
    fn set_table_size(&mut self, num_columns: usize, num_rows: usize) {
        self.num_columns = num_columns;
        self.num_rows = num_rows;
    }

    fn decorate_cell(
        &mut self,
        _column: usize,
        row: usize,
        _has_more: bool,
        area: genpdf::render::Area<'_>,
        _style: genpdf::style::Style,
    ) {
        let size = area.size();
        
        // Header row (row 0)
        if row == 0 {
            // Draw a thicker black line at the bottom of the header
            let mut style = genpdf::style::Style::default();
            style.set_color(genpdf::style::Color::Rgb(0, 0, 0));
            
            // Draw border top
            area.draw_line(
                vec![
                    genpdf::Position::new(0, size.height),
                    genpdf::Position::new(size.width, size.height),
                ],
                style,
            );
        } else {
            // Data rows: Draw a very light gray line at the bottom
            let mut style = genpdf::style::Style::default();
            style.set_color(genpdf::style::Color::Rgb(220, 220, 220));
            
            area.draw_line(
                vec![
                    genpdf::Position::new(0, size.height),
                    genpdf::Position::new(size.width, size.height),
                ],
                style,
            );
        }
    }
}

#[derive(FromRow)]
struct UserAuthRecord {
    id: Uuid,
    username: String,
    password_hash: String,
}



pub async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<AuthRequest>,
) -> Result<Json<AuthResponse>, (StatusCode, String)> {
    let record = sqlx::query_as::<_, UserAuthRecord>(
        "SELECT id, username, password_hash FROM users WHERE username = $1"
    )
        .bind(&payload.username)
        .fetch_optional(&pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if let Some(user) = record {
        if auth::verify_password(&user.password_hash, &payload.password) {
            let token = auth::create_jwt(user.id).map_err(|e| {
                (StatusCode::INTERNAL_SERVER_ERROR, format!("Token creation failed: {}", e))
            })?;
            return Ok(Json(AuthResponse {
                token,
                user: UserDto { id: user.id, username: user.username },
            }));
        }
    }

    Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))
}

pub async fn update_profile(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateUserDto>,
) -> Result<StatusCode, (StatusCode, String)> {
    if payload.username.is_none() && payload.password.is_none() {
        return Ok(StatusCode::NO_CONTENT);
    }

    let mut query = sqlx::QueryBuilder::new("UPDATE users SET ");
    let mut has_fields = false;

    if let Some(ref username) = payload.username {
        query.push("username = ").push_bind(username);
        has_fields = true;
    }

    if let Some(ref password) = payload.password {
        let hash = auth::hash_password(password).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, format!("Hashing error: {}", e)))?;
        if has_fields { query.push(", "); }
        query.push("password_hash = ").push_bind(hash);
    }

    query.push(" WHERE id = ").push_bind(claims.sub);

    let result = query.build().execute(&pool).await.map_err(|_| (StatusCode::CONFLICT, "Username likely in use".to_string()))?;

    if result.rows_affected() > 0 {
        Ok(StatusCode::OK)
    } else {
        Err((StatusCode::NOT_FOUND, "User not found".to_string()))
    }
}

pub async fn get_all_books(
    claims: Claims,
    State(pool): State<PgPool>,
    Query(filters): Query<BookFilterQuery>,
) -> Result<Json<PaginatedBooks>, (StatusCode, String)> {
    match repository::fetch_books(&pool, filters, claims.sub).await {
        Ok(paginated) => Ok(Json(paginated)),
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", error))),
    }
}

pub async fn create_book(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(payload): Json<CreateBookDto>,
) -> Result<(StatusCode, Json<Book>), (StatusCode, String)> {
    match repository::create_book(&pool, payload, claims.sub).await {
        Ok(book) => Ok((StatusCode::CREATED, Json(book))),
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to create book: {}", error))),
    }
}

pub async fn delete_book(
    _claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, (StatusCode, String)> {
    match repository::delete_book(&pool, id).await {
        Ok(rows_affected) => {
            if rows_affected > 0 { Ok(StatusCode::NO_CONTENT) } else { Err((StatusCode::NOT_FOUND, "Book not found".to_string())) }
        }
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete book: {}", error))),
    }
}

pub async fn update_book(
    claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<CreateBookDto>,
) -> Result<Json<Book>, (StatusCode, String)> {
    match repository::update_book(&pool, id, payload, claims.sub).await {
        Ok(Some(book)) => Ok(Json(book)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Book not found".to_string())),
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to update book: {}", error))),
    }
}

#[derive(Deserialize)]
pub struct SearchQuery { pub q: String }

pub async fn lookup_metadata(
    _claims: Claims,
    Path(identifier): Path<String>,
) -> Result<Json<BookMetadataResponse>, (StatusCode, String)> {
    match integration::fetch_metadata(&identifier).await {
        Ok(metadata) => Ok(Json(metadata)),
        Err(_) => Err((StatusCode::BAD_GATEWAY, "Failed to connect to provider".to_string())),
    }
}

pub async fn search_metadata(
    _claims: Claims,
    Query(query): Query<SearchQuery>,
) -> Result<Json<Vec<BookMetadataResponse>>, (StatusCode, String)> {
    match integration::search_metadata_by_query(&query.q).await {
        Ok(results) => Ok(Json(results)),
        Err(_) => Err((StatusCode::BAD_GATEWAY, "Failed to connect to provider".to_string())),
    }
}

#[derive(Deserialize)]
pub struct AutocompleteQuery {
    pub field: String,
    pub q: String,
    pub limit: Option<i64>,
}

pub async fn get_autocomplete(
    claims: Claims,
    State(pool): State<PgPool>,
    Query(query): Query<AutocompleteQuery>,
) -> Result<Json<Vec<String>>, (StatusCode, String)> {
    match repository::fetch_autocomplete_suggestions(&pool, &query.field, &query.q, query.limit, claims.sub).await {
        Ok(results) => Ok(Json(results)),
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Database error: {}", error))),
    }
}

pub async fn delete_books_batch(
    _claims: Claims,
    State(pool): State<PgPool>,
    Json(payload): Json<BatchDeleteRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    match repository::delete_books_batch(&pool, payload.ids).await {
        Ok(rows_affected) => {
            if rows_affected > 0 { Ok(StatusCode::NO_CONTENT) } else { Err((StatusCode::NOT_FOUND, "No books found".to_string())) }
        }
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to delete batch: {}", error))),
    }
}

#[derive(Serialize)]
pub struct UploadResponse { pub url: String }

pub async fn upload_cover(
    _claims: Claims,
    mut multipart: Multipart,
) -> Result<Json<UploadResponse>, (StatusCode, String)> {
    while let Some(field) = multipart.next_field().await.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))? {
        if field.name() == Some("cover") {
            let data = field.bytes().await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            if data.len() > 5 * 1024 * 1024 { return Err((StatusCode::PAYLOAD_TOO_LARGE, "Limit is 5MB".to_string())); }

            let data_vec = data.to_vec();
            let webp_data = tokio::task::spawn_blocking(move || {
                let img = image::load_from_memory(&data_vec).map_err(|_| "Invalid image")?;
                let img = img.resize(600, 900, image::imageops::FilterType::Lanczos3);
                let mut buffer = Cursor::new(Vec::new());
                img.write_to(&mut buffer, image::ImageFormat::WebP).map_err(|_| "Failed encoding")?;
                Ok::<Vec<u8>, &'static str>(buffer.into_inner())
            }).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Panic".to_string()))?.map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

            let file_uuid = Uuid::new_v4().to_string();
            let relative_dir = format!("uploads/{}/{}", &file_uuid[0..2], &file_uuid[2..4]);
            tokio::fs::create_dir_all(&relative_dir).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            let file_path = format!("{}/{}.webp", relative_dir, file_uuid);
            let mut file = File::create(&file_path).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
            file.write_all(&webp_data).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

            return Ok(Json(UploadResponse { url: format!("/static/{}/{}/{}.webp", &file_uuid[0..2], &file_uuid[2..4], file_uuid) }));
        }
    }
    Err((StatusCode::BAD_REQUEST, "No cover field".to_string()))
}

pub async fn export_csv(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(payload): Json<ExportRequest>,
) -> impl IntoResponse {
    let books_result = repository::fetch_all_for_export(&pool, &payload.filters, payload.specific_ids, claims.sub).await;
    let requested_columns: Vec<String> = if payload.columns.is_empty() { vec!["id".to_string(), "title".to_string()] } else { payload.columns.into_iter().filter(|c| c != "cover_url").collect() };

    let mut csv_data = String::from("\u{FEFF}");
    
    let header_row: Vec<String> = requested_columns.iter().map(|col| {
        payload.column_labels.as_ref().and_then(|m| m.get(col)).cloned().unwrap_or_else(|| col.replace("_", " ").to_uppercase())
    }).collect();
    csv_data.push_str(&header_row.join(";"));
    csv_data.push('\n');

    if let Ok(books) = books_result {
        for book in books {
            let json_val = serde_json::to_value(&book).unwrap_or(serde_json::json!({}));
            let mut row_values = Vec::new();
            for col in &requested_columns {
                let cell_val = match json_val.get(col) {
                    Some(serde_json::Value::String(s)) => {
                        if col.contains("date") || col.ends_with("_at") {
                            format_date(s, payload.date_format.as_ref())
                        } else {
                            s.to_string()
                        }
                    },
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

    ([(axum::http::header::CONTENT_TYPE, "text/csv; charset=utf-8"), (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"export.csv\"")], Body::from(csv_data))
}

pub async fn export_xml(claims: Claims, State(pool): State<PgPool>, Json(payload): Json<ExportRequest>) -> impl IntoResponse {
    let books_result = repository::fetch_all_for_export(&pool, &payload.filters, payload.specific_ids, claims.sub).await;
    let requested_columns: Vec<String> = if payload.columns.is_empty() { vec!["id".to_string(), "title".to_string()] } else { payload.columns.into_iter().filter(|c| c != "cover_url").collect() };

    let mut xml_data = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<books>\n");
    if let Ok(books) = books_result {
        for book in books {
            let json_val = serde_json::to_value(&book).unwrap_or(serde_json::json!({}));
            xml_data.push_str("  <book>\n");
            for col in &requested_columns {
                let cell_val = match json_val.get(col) {
                    Some(serde_json::Value::String(s)) => {
                        if col.contains("date") || col.ends_with("_at") {
                            format_date(s, payload.date_format.as_ref())
                        } else {
                            s.to_string()
                        }
                    },
                    Some(serde_json::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>().join(", "),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::Bool(b)) => if *b { "Yes".to_string() } else { "No".to_string() },
                    _ => "".to_string(),
                };
                let safe_val = cell_val.replace("<", "&lt;").replace(">", "&gt;").replace("&", "&amp;");
                let label = payload.column_labels.as_ref().and_then(|m| m.get(col)).cloned().unwrap_or_else(|| col.clone());
                // XML tags can't have spaces, so replace spaces with underscores if using labels
                let tag_name = label.replace(" ", "_").to_lowercase();
                xml_data.push_str(&format!("    <{}>{}</{}>\n", tag_name, safe_val, tag_name));
            }
            xml_data.push_str("  </book>\n");
        }
    }
    xml_data.push_str("</books>\n");
    ([(axum::http::header::CONTENT_TYPE, "application/xml; charset=utf-8"), (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"export.xml\"")], Body::from(xml_data))
}

pub async fn export_pdf(claims: Claims, State(pool): State<PgPool>, Json(payload): Json<ExportRequest>) -> impl IntoResponse {
    let books_result = repository::fetch_all_for_export(&pool, &payload.filters, payload.specific_ids, claims.sub).await;
    let requested_columns: Vec<String> = if payload.columns.is_empty() { vec!["title".to_string()] } else { payload.columns.into_iter().filter(|c| c != "cover_url").collect() };

    let font_family = PDF_FONT_FAMILY.get_or_init(|| {
        genpdf::fonts::from_files("fonts", "Roboto", None).expect("Failed to load fonts")
    }).clone();

    let mut doc = genpdf::Document::new(font_family);
    doc.set_paper_size(genpdf::Size { width: genpdf::Mm::from(297.0), height: genpdf::Mm::from(210.0) });

    let mut decorator = genpdf::SimplePageDecorator::new();
    decorator.set_margins(10);
    doc.set_page_decorator(decorator);

    let font_size = match requested_columns.len() {
        0..=5 => 10,
        6..=8 => 9,
        9..=12 => 8,
        _ => 7,
    };
    doc.set_font_size(font_size);

    let mut table = genpdf::elements::TableLayout::new(vec![1; requested_columns.len()]);
    table.set_cell_decorator(CleanTableDecorator::new());

    let mut header_row = table.row();
    for col in &requested_columns { 
        let label = payload.column_labels.as_ref().and_then(|m| m.get(col)).cloned().unwrap_or_else(|| col.replace("_", " ").to_uppercase());
        header_row.push_element(
            genpdf::elements::PaddedElement::new(
                genpdf::elements::Paragraph::new(label),
                genpdf::Margins::trbl(1.5, 1.0, 1.5, 1.0)
            )
        ); 
    }
    header_row.push().unwrap_or_default();

    if let Ok(books) = books_result {
        for book in books {
            let json_val = serde_json::to_value(&book).unwrap_or(serde_json::json!({}));
            let mut row = table.row();
            for col in &requested_columns {
                let cell_val = match json_val.get(col) {
                    Some(serde_json::Value::String(s)) => {
                        if col.contains("date") || col.ends_with("_at") {
                            format_date(s, payload.date_format.as_ref())
                        } else {
                            s.to_string()
                        }
                    },
                    Some(serde_json::Value::Array(a)) => a.iter().filter_map(|v| v.as_str()).collect::<Vec<&str>>().join(", "),
                    Some(serde_json::Value::Number(n)) => n.to_string(),
                    Some(serde_json::Value::Bool(b)) => if *b { "Yes".to_string() } else { "No".to_string() },
                    _ => "".to_string(),
                };
                row.push_element(
                    genpdf::elements::PaddedElement::new(
                        genpdf::elements::Paragraph::new(cell_val),
                        genpdf::Margins::trbl(1.5, 1.0, 1.5, 1.0)
                    )
                );
            }
            row.push().unwrap_or_default();
        }
    }

    doc.push(table);
    let mut buffer = Cursor::new(Vec::new());
    match doc.render(&mut buffer) {
        Ok(_) => ([(axum::http::header::CONTENT_TYPE, "application/pdf"), (axum::http::header::CONTENT_DISPOSITION, "attachment; filename=\"export.pdf\"")], Body::from(buffer.into_inner())).into_response(),
        Err(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Failed to render PDF".to_string()).into_response()
    }
}

pub async fn patch_book(
    claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateBookPartialDto>,
) -> Result<Json<Book>, (StatusCode, String)> {
    match repository::patch_book(&pool, id, payload, claims.sub).await {
        Ok(Some(book)) => Ok(Json(book)),
        Ok(None) => Err((StatusCode::NOT_FOUND, "Book not found".to_string())),
        Err(error) => Err((StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to patch book: {}", error))),
    }
}

pub async fn get_libraries(
    claims: Claims,
    State(pool): State<PgPool>,
) -> Result<Json<Vec<Library>>, StatusCode> {
    match repository::get_libraries(&pool, claims.sub).await {
        Ok(libs) => Ok(Json(libs)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn create_library(
    claims: Claims,
    State(pool): State<PgPool>,
    Json(payload): Json<CreateLibraryDto>,
) -> Result<Json<Library>, StatusCode> {
    match repository::create_library(&pool, payload, claims.sub).await {
        Ok(lib) => Ok(Json(lib)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn update_library(
    claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateLibraryDto>,
) -> Result<Json<Library>, StatusCode> {
    match repository::update_library(&pool, id, payload, claims.sub).await {
        Ok(lib) => Ok(Json(lib)),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn delete_library(
    claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, StatusCode> {
    match repository::delete_library(&pool, id, claims.sub).await {
        Ok(affected) => {
            if affected > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        },
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_library_members(
    claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<LibraryMember>>, StatusCode> {
    match repository::get_library_members(&pool, id, claims.sub).await {
        Ok(members) => Ok(Json(members)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::FORBIDDEN),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn add_library_member(
    claims: Claims,
    State(pool): State<PgPool>,
    Path(id): Path<Uuid>,
    Json(payload): Json<ShareLibraryDto>,
) -> Result<Json<LibraryMember>, StatusCode> {
    match repository::add_library_member(&pool, id, payload, claims.sub).await {
        Ok(member) => Ok(Json(member)),
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn remove_library_member(
    claims: Claims,
    State(pool): State<PgPool>,
    Path((lib_id, user_id)): Path<(Uuid, Uuid)>,
) -> Result<StatusCode, StatusCode> {
    match repository::remove_library_member(&pool, lib_id, user_id, claims.sub).await {
        Ok(affected) => {
            if affected > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(StatusCode::NOT_FOUND)
            }
        },
        Err(sqlx::Error::RowNotFound) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}