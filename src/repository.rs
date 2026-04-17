use crate::models::Book;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn fetch_all_books(pool: &PgPool) -> Result<Vec<Book>, sqlx::Error> {
    let books = sqlx::query_as::<_, Book>("SELECT * FROM books")
        .fetch_all(pool)
        .await?;

    Ok(books)
}

use crate::models::CreateBookDto;

pub async fn create_book(pool: &PgPool, payload: CreateBookDto) -> Result<Book, sqlx::Error> {
    let book = sqlx::query_as::<_, Book>(
        r#"
        INSERT INTO books (
            isbn_13, isbn_10, open_library_id, oclc_number, title, subtitle, original_title,
            authors, translators, illustrators, publisher, publish_date, original_publish_date,
            edition_number, printing_number, original_edition, is_first_edition, collection_name,
            volume_in_collection, series_name, volume_in_series, book_format, page_count,
            dimensions, weight, language, original_language, subjects, genres, target_audience,
            description, table_of_contents, cover_url, purchase_date, purchase_price,
            store_or_vendor, acquisition_type, location_property, location_room, location_bookcase,
            location_shelf, location_position, condition_state, personal_notes, read_status,
            rating, date_started, date_finished, reading_notes, is_loaned, loaned_to,
            loan_date, expected_return_date
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36,
            $37, $38, $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50, $51, $52, $53
        )
        RETURNING *
        "#,
    )
    .bind(payload.isbn_13)
    .bind(payload.isbn_10)
    .bind(payload.open_library_id)
    .bind(payload.oclc_number)
    .bind(payload.title)
    .bind(payload.subtitle)
    .bind(payload.original_title)
    .bind(payload.authors)
    .bind(payload.translators)
    .bind(payload.illustrators)
    .bind(payload.publisher)
    .bind(payload.publish_date)
    .bind(payload.original_publish_date)
    .bind(payload.edition_number)
    .bind(payload.printing_number)
    .bind(payload.original_edition)
    .bind(payload.is_first_edition)
    .bind(payload.collection_name)
    .bind(payload.volume_in_collection)
    .bind(payload.series_name)
    .bind(payload.volume_in_series)
    .bind(payload.book_format)
    .bind(payload.page_count)
    .bind(payload.dimensions)
    .bind(payload.weight)
    .bind(payload.language)
    .bind(payload.original_language)
    .bind(payload.subjects)
    .bind(payload.genres)
    .bind(payload.target_audience)
    .bind(payload.description)
    .bind(payload.table_of_contents)
    .bind(payload.cover_url)
    .bind(payload.purchase_date)
    .bind(payload.purchase_price)
    .bind(payload.store_or_vendor)
    .bind(payload.acquisition_type)
    .bind(payload.location_property)
    .bind(payload.location_room)
    .bind(payload.location_bookcase)
    .bind(payload.location_shelf)
    .bind(payload.location_position)
    .bind(payload.condition_state)
    .bind(payload.personal_notes)
    .bind(payload.read_status)
    .bind(payload.rating)
    .bind(payload.date_started)
    .bind(payload.date_finished)
    .bind(payload.reading_notes)
    .bind(payload.is_loaned)
    .bind(payload.loaned_to)
    .bind(payload.loan_date)
    .bind(payload.expected_return_date)
    .fetch_one(pool)
    .await?;

    Ok(book)
}

pub async fn delete_book(pool: &PgPool, book_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM books WHERE id = $1")
        .bind(book_id)
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}
