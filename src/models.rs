use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
}

#[derive(Debug, Serialize, Deserialize, Clone, FromRow)]
pub struct UserDto {
    pub id: Uuid,
    pub username: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserDto {
    pub username: Option<String>,
    pub password: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Library {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub owner_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateLibraryDto {
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateLibraryDto {
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct LibraryMember {
    pub library_id: Uuid,
    pub user_id: Uuid,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub username: Option<String>, // joined field for frontend convenience
}

#[derive(Debug, Deserialize)]
pub struct ShareLibraryDto {
    pub username: String,
    pub role: String, // 'editor', 'viewer'
}

#[derive(Debug, Serialize, Deserialize, FromRow)]
pub struct Book {
    pub id: Uuid,
    pub library_id: Uuid,
    pub catalog_number: i32,
    pub isbn_13: Option<String>,
    pub isbn_10: Option<String>,
    pub open_library_id: Option<String>,
    pub oclc_number: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub original_title: Option<String>,
    pub authors: Vec<String>,
    pub translators: Option<Vec<String>>,
    pub illustrators: Option<Vec<String>>,
    pub publisher: Option<String>,
    pub publish_date: Option<NaiveDate>,
    pub original_publish_date: Option<NaiveDate>,
    pub edition_number: Option<String>,
    pub printing_number: Option<String>,
    pub original_edition: Option<String>,
    pub is_first_edition: Option<bool>,
    pub collection_name: Option<String>,
    pub volume_in_collection: Option<i32>,
    pub series_name: Option<String>,
    pub volume_in_series: Option<i32>,
    pub book_format: Option<String>,
    pub page_count: Option<i32>,
    pub dimensions: Option<String>,
    pub weight: Option<String>,
    pub language: Option<String>,
    pub original_language: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub genres: Option<Vec<String>>,
    pub target_audience: Option<String>,
    pub description: Option<String>,
    pub table_of_contents: Option<String>,
    pub cover_url: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub purchase_price: Option<Decimal>,
    pub store_or_vendor: Option<String>,
    pub acquisition_type: Option<String>,
    pub location_property: Option<String>,
    pub location_room: Option<String>,
    pub location_bookcase: Option<String>,
    pub location_shelf: Option<String>,
    pub location_position: Option<i32>,
    pub condition_state: Option<String>,
    pub is_loaned: Option<bool>,
    pub loaned_to: Option<String>,
    pub loan_date: Option<DateTime<Utc>>,
    pub expected_return_date: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub read_status: Option<String>,
    pub rating: Option<i32>,
    pub personal_notes: Option<String>,
    pub reading_notes: Option<String>,
    pub date_started: Option<NaiveDate>,
    pub date_finished: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBookDto {
    pub library_id: Option<Uuid>,
    pub isbn_13: Option<String>,
    pub isbn_10: Option<String>,
    pub open_library_id: Option<String>,
    pub oclc_number: Option<String>,
    pub title: String,
    pub subtitle: Option<String>,
    pub original_title: Option<String>,
    pub authors: Vec<String>,
    pub translators: Option<Vec<String>>,
    pub illustrators: Option<Vec<String>>,
    pub publisher: Option<String>,
    pub publish_date: Option<NaiveDate>,
    pub original_publish_date: Option<NaiveDate>,
    pub edition_number: Option<String>,
    pub printing_number: Option<String>,
    pub original_edition: Option<String>,
    pub is_first_edition: Option<bool>,
    pub collection_name: Option<String>,
    pub volume_in_collection: Option<i32>,
    pub series_name: Option<String>,
    pub volume_in_series: Option<i32>,
    pub book_format: Option<String>,
    pub page_count: Option<i32>,
    pub dimensions: Option<String>,
    pub weight: Option<String>,
    pub language: Option<String>,
    pub original_language: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub genres: Option<Vec<String>>,
    pub target_audience: Option<String>,
    pub description: Option<String>,
    pub table_of_contents: Option<String>,
    pub cover_url: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub purchase_price: Option<Decimal>,
    pub store_or_vendor: Option<String>,
    pub acquisition_type: Option<String>,
    pub location_property: Option<String>,
    pub location_room: Option<String>,
    pub location_bookcase: Option<String>,
    pub location_shelf: Option<String>,
    pub location_position: Option<i32>,
    pub condition_state: Option<String>,
    pub is_loaned: Option<bool>,
    pub loaned_to: Option<String>,
    pub loan_date: Option<DateTime<Utc>>,
    pub expected_return_date: Option<DateTime<Utc>>,
    pub read_status: Option<String>,
    pub rating: Option<i32>,
    pub personal_notes: Option<String>,
    pub reading_notes: Option<String>,
    pub date_started: Option<NaiveDate>,
    pub date_finished: Option<NaiveDate>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum QueryAST {
    Condition {
        field: String,
        operator: String,
        value: String,
    },
    And {
        nodes: Vec<QueryAST>,
    },
    Or {
        nodes: Vec<QueryAST>,
    },
    Not {
        node: Box<QueryAST>,
    },
}

#[derive(Debug, Deserialize)]
pub struct BookFilterQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub sort_by: Option<String>,
    pub sort_order: Option<String>,
    pub query: Option<String>,
    pub library_id: Option<Uuid>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct BookMetadataResponse {
    pub isbn: Option<String>,
    pub title: Option<String>,
    pub authors: Option<Vec<String>>,
    pub publish_date: Option<String>,
    pub page_count: Option<i32>,
    pub cover_url: Option<String>,
    pub subtitle: Option<String>,
    pub publishers: Option<Vec<String>>,
    pub physical_format: Option<String>,
    pub weight: Option<String>,
    pub dimensions: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub languages: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PaginatedBooks {
    pub data: Vec<Book>,
    pub total: i64,
}

#[derive(Debug, Deserialize)]
pub struct BatchDeleteRequest {
    pub ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub filters: BookFilterQuery,
    pub columns: Vec<String>,
    pub specific_ids: Option<Vec<Uuid>>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBookPartialDto {
    pub library_id: Option<Uuid>,
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub original_title: Option<String>,
    pub authors: Option<Vec<String>>,
    pub translators: Option<Vec<String>>,
    pub illustrators: Option<Vec<String>>,
    pub publisher: Option<String>,
    pub publish_date: Option<NaiveDate>,
    pub original_publish_date: Option<NaiveDate>,
    pub edition_number: Option<String>,
    pub printing_number: Option<String>,
    pub original_edition: Option<String>,
    pub is_first_edition: Option<bool>,
    pub collection_name: Option<String>,
    pub volume_in_collection: Option<i32>,
    pub series_name: Option<String>,
    pub volume_in_series: Option<i32>,
    pub book_format: Option<String>,
    pub page_count: Option<i32>,
    pub dimensions: Option<String>,
    pub weight: Option<String>,
    pub language: Option<String>,
    pub original_language: Option<String>,
    pub subjects: Option<Vec<String>>,
    pub genres: Option<Vec<String>>,
    pub target_audience: Option<String>,
    pub description: Option<String>,
    pub table_of_contents: Option<String>,
    pub cover_url: Option<String>,
    pub purchase_date: Option<NaiveDate>,
    pub purchase_price: Option<Decimal>,
    pub store_or_vendor: Option<String>,
    pub acquisition_type: Option<String>,
    pub location_property: Option<String>,
    pub location_room: Option<String>,
    pub location_bookcase: Option<String>,
    pub location_shelf: Option<String>,
    pub location_position: Option<i32>,
    pub condition_state: Option<String>,
    pub is_loaned: Option<bool>,
    pub loaned_to: Option<String>,
    pub loan_date: Option<DateTime<Utc>>,
    pub expected_return_date: Option<DateTime<Utc>>,
    pub read_status: Option<String>,
    pub rating: Option<i32>,
    pub personal_notes: Option<String>,
    pub reading_notes: Option<String>,
    pub date_started: Option<NaiveDate>,
    pub date_finished: Option<NaiveDate>,
}