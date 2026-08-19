use crate::models::{Book, BookFilterQuery, CreateBookDto, PaginatedBooks, QueryAST, UpdateBookPartialDto, Library, CreateLibraryDto, UpdateLibraryDto, LibraryMember, ShareLibraryDto};
use sqlx::{PgPool, Postgres, QueryBuilder, Transaction};
use uuid::Uuid;

pub async fn fetch_books(
    pool: &PgPool,
    query_params: BookFilterQuery,
    user_id: Uuid,
) -> Result<PaginatedBooks, sqlx::Error> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT b.*, ubi.read_status, ubi.rating, ubi.personal_notes, ubi.reading_notes, ubi.date_started, ubi.date_finished \
         FROM books b \
         LEFT JOIN user_book_interactions ubi ON b.id = ubi.book_id AND ubi.user_id = "
    );
    query.push_bind(user_id);
    query.push(" WHERE b.library_id IN (SELECT l.id FROM libraries l LEFT JOIN library_members lm ON l.id = lm.library_id WHERE l.owner_id = ");
    query.push_bind(user_id);
    query.push(" OR lm.user_id = ");
    query.push_bind(user_id);
    query.push(") ");

    let mut count_query: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT COUNT(*) \
         FROM books b \
         LEFT JOIN user_book_interactions ubi ON b.id = ubi.book_id AND ubi.user_id = "
    );
    count_query.push_bind(user_id);
    count_query.push(" WHERE b.library_id IN (SELECT l.id FROM libraries l LEFT JOIN library_members lm ON l.id = lm.library_id WHERE l.owner_id = ");
    count_query.push_bind(user_id);
    count_query.push(" OR lm.user_id = ");
    count_query.push_bind(user_id);
    count_query.push(") ");

    if let Some(lib_id) = query_params.library_id {
        query.push(" AND b.library_id = ");
        query.push_bind(lib_id);
        count_query.push(" AND b.library_id = ");
        count_query.push_bind(lib_id);
    }

    apply_filters(&query_params, &mut query);
    apply_filters(&query_params, &mut count_query);
    apply_sorting(&query_params, &mut query);

    if let Some(limit) = query_params.limit {
        query.push(" LIMIT ").push_bind(limit.clamp(1, 100000));
    }
    
    if let Some(offset) = query_params.offset {
        query.push(" OFFSET ").push_bind(offset.max(0));
    }

    let books = query.build_query_as::<Book>().fetch_all(pool).await?;
    let total_count: (i64,) = count_query.build_query_as().fetch_one(pool).await?;

    Ok(PaginatedBooks {
        data: books,
        total: total_count.0,
    })
}

pub async fn fetch_book_by_id(
    tx: &mut Transaction<'_, Postgres>,
    book_id: Uuid,
    user_id: Uuid,
) -> Result<Book, sqlx::Error> {
    let query = "
        SELECT b.*, ubi.read_status, ubi.rating, ubi.personal_notes, ubi.reading_notes, ubi.date_started, ubi.date_finished
        FROM books b
        LEFT JOIN user_book_interactions ubi ON b.id = ubi.book_id AND ubi.user_id = $1
        WHERE b.id = $2
    ";
    sqlx::query_as::<_, Book>(query)
        .bind(user_id)
        .bind(book_id)
        .fetch_one(&mut **tx)
        .await
}

pub fn apply_filters<'a>(query_params: &'a BookFilterQuery, query: &mut QueryBuilder<'a, Postgres>) {
    if let Some(query_str) = &query_params.query {
        if let Ok(ast) = serde_json::from_str::<QueryAST>(query_str) {
            query.push(" AND (");
            build_query_recursive(&ast, query);
            query.push(")");
        }
    }
}

pub fn apply_sorting<'a>(query_params: &'a BookFilterQuery, query: &mut QueryBuilder<'a, Postgres>) {
    let allowed_sort_columns = [
        "catalog_number", "title", "page_count", "rating", "publish_date",
        "created_at", "updated_at", "purchase_price", "authors", "publisher",
        "isbn_13", "location_room", "location_bookcase", "subtitle", "original_title",
        "translators", "illustrators", "original_publish_date", "isbn_10", "oclc_number",
        "open_library_id", "edition", "edition_number", "printing_number", "original_edition",
        "is_first_edition", "collection_name", "volume_in_collection", "series_name",
        "volume_in_series", "book_format", "dimensions", "weight", "language",
        "original_language", "subjects", "genres", "target_audience", "purchase_date",
        "store_or_vendor", "acquisition_type", "location_property", "location_shelf",
        "location_position", "condition_state", "read_status", "date_started",
        "date_finished", "is_loaned", "loaned_to", "loan_date", "expected_return_date",
        "description", "table_of_contents", "personal_notes", "reading_notes"
    ];

    let sort_col = query_params.sort_by.as_deref().unwrap_or("created_at");
    let final_sort_col = if allowed_sort_columns.contains(&sort_col) { sort_col } else { "created_at" };
    let prefix = if ["rating"].contains(&final_sort_col) { "ubi" } else { "b" };
    let order = if query_params.sort_order.as_deref() == Some("asc") { "ASC" } else { "DESC" };

    query.push(format!(" ORDER BY {}.{} {} NULLS LAST, b.id ASC ", prefix, final_sort_col, order));
}

pub fn build_query_recursive(ast: &QueryAST, query: &mut QueryBuilder<Postgres>) {
    match ast {
        QueryAST::Condition { field, operator, value } => apply_condition(field, operator, value, query),
        QueryAST::And { nodes } => {
            query.push("(");
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 { query.push(" AND "); }
                build_query_recursive(node, query);
            }
            query.push(")");
        }
        QueryAST::Or { nodes } => {
            query.push("(");
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 { query.push(" OR "); }
                build_query_recursive(node, query);
            }
            query.push(")");
        }
        QueryAST::Not { node } => {
            query.push("NOT (");
            build_query_recursive(node, query);
            query.push(")");
        }
    }
}

pub fn apply_condition(field: &str, operator: &str, value: &str, query: &mut QueryBuilder<Postgres>) {
    if field == "search" {
        let term = format!("%{}%", value);
        query.push(" (b.title ILIKE ").push_bind(term.clone())
            .push(" OR b.original_title ILIKE ").push_bind(term).push(") ");
        return;
    }

    if field == "author" || field == "authors" {
        query.push(" array_to_string(b.authors, ', ') ILIKE ").push_bind(format!("%{}%", value));
        return;
    }

    let ubi_fields = ["read_status", "rating", "personal_notes", "reading_notes", "date_started", "date_finished"];
    let prefix = if ubi_fields.contains(&field) { "ubi" } else { "b" };
    let full_field = format!("{}.{}", prefix, field);

    let text_columns = ["title", "subtitle", "original_title", "publisher", "collection_name", "series_name", "description", "table_of_contents", "personal_notes", "reading_notes", "location_property", "location_room", "location_bookcase", "location_shelf", "loaned_to", "dimensions", "weight"];
    let exact_string_columns = ["read_status", "book_format", "condition_state", "target_audience", "language", "original_language", "store_or_vendor", "acquisition_type", "isbn_13", "isbn_10", "oclc_number", "open_library_id", "edition", "edition_number", "printing_number", "original_edition"];
    let numeric_columns = ["catalog_number", "page_count", "edition_number", "rating", "volume_in_collection", "volume_in_series", "purchase_price", "location_position"];
    let date_columns = ["publish_date", "original_publish_date", "purchase_date", "date_started", "date_finished", "loan_date", "expected_return_date", "created_at", "updated_at"];
    let boolean_columns = ["is_first_edition", "is_loaned"];

    if text_columns.contains(&field) || exact_string_columns.contains(&field) {
        match operator {
            "_contains" => { query.push(format!(" {} ILIKE ", full_field)).push_bind(format!("%{}%", value)); }
            "_contains_case" => { query.push(format!(" {} LIKE ", full_field)).push_bind(format!("%{}%", value)); }
            "_starts" => { query.push(format!(" {} ILIKE ", full_field)).push_bind(format!("{}%", value)); }
            "_starts_case" => { query.push(format!(" {} LIKE ", full_field)).push_bind(format!("{}%", value)); }
            "_ends" => { query.push(format!(" {} ILIKE ", full_field)).push_bind(format!("%{}", value)); }
            "_ends_case" => { query.push(format!(" {} LIKE ", full_field)).push_bind(format!("%{}", value)); }
            "_exact" => { query.push(format!(" {} = ", full_field)).push_bind(value.to_string()); }
            "_empty" => {
                if value == "true" { query.push(format!(" ({} IS NULL OR {} = '') ", full_field, full_field)); }
                else { query.push(format!(" ({} IS NOT NULL AND {} != '') ", full_field, full_field)); }
            }
            _ => { query.push(format!(" {} ILIKE ", full_field)).push_bind(format!("%{}%", value)); }
        }
    } else if numeric_columns.contains(&field) {
        if let Ok(num_val) = value.parse::<i32>() {
            match operator {
                "_gt" => { query.push(format!(" {} > ", full_field)).push_bind(num_val); }
                "_gte" => { query.push(format!(" {} >= ", full_field)).push_bind(num_val); }
                "_lt" => { query.push(format!(" {} < ", full_field)).push_bind(num_val); }
                "_lte" => { query.push(format!(" {} <= ", full_field)).push_bind(num_val); }
                _ => { query.push(format!(" {} = ", full_field)).push_bind(num_val); }
            }
        } else { query.push(" 1=0 "); }
    } else if date_columns.contains(&field) {
        if let Ok(date_val) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            match operator {
                "_gt" => { query.push(format!(" {} > ", full_field)).push_bind(date_val); }
                "_gte" => { query.push(format!(" {} >= ", full_field)).push_bind(date_val); }
                "_lt" => { query.push(format!(" {} < ", full_field)).push_bind(date_val); }
                "_lte" => { query.push(format!(" {} <= ", full_field)).push_bind(date_val); }
                _ => { query.push(format!(" {} = ", full_field)).push_bind(date_val); }
            }
        } else if let Ok(datetime_val) = chrono::DateTime::parse_from_rfc3339(value) {
            match operator {
                "_gt" => { query.push(format!(" {} > ", full_field)).push_bind(datetime_val.naive_utc()); }
                "_gte" => { query.push(format!(" {} >= ", full_field)).push_bind(datetime_val.naive_utc()); }
                "_lt" => { query.push(format!(" {} < ", full_field)).push_bind(datetime_val.naive_utc()); }
                "_lte" => { query.push(format!(" {} <= ", full_field)).push_bind(datetime_val.naive_utc()); }
                _ => { query.push(format!(" {} = ", full_field)).push_bind(datetime_val.naive_utc()); }
            }
        } else { query.push(" 1=0 "); }
    } else if boolean_columns.contains(&field) {
        if value == "true" {
            query.push(format!(" {} = TRUE ", full_field));
        } else if value == "false" {
            query.push(format!(" {} = FALSE ", full_field));
        } else {
            query.push(" 1=0 ");
        }
    } else {
        query.push(" 1=1 ");
    }
}

pub async fn create_book(
    pool: &PgPool,
    payload: CreateBookDto,
    user_id: Uuid,
) -> Result<Book, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let library_id = match payload.library_id {
        Some(id) => id,
        None => {
            sqlx::query_scalar("SELECT id FROM libraries WHERE owner_id = $1 ORDER BY created_at ASC LIMIT 1")
                .bind(user_id)
                .fetch_one(&mut *tx)
                .await?
        }
    };

    let book_id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO books (
            library_id, isbn_13, isbn_10, open_library_id, oclc_number, title, subtitle, original_title,
            authors, translators, illustrators, publisher, publish_date, original_publish_date,
            edition, edition_number, printing_number, original_edition, is_first_edition, collection_name,
            volume_in_collection, series_name, volume_in_series, book_format, page_count,
            dimensions, weight, language, original_language, subjects, genres, target_audience,
            description, table_of_contents, cover_url, purchase_date, purchase_price,
            store_or_vendor, acquisition_type, location_property, location_room, location_bookcase,
            location_shelf, location_position, condition_state, is_loaned, loaned_to,
            loan_date, expected_return_date
        ) VALUES (
            $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19,
            $20, $21, $22, $23, $24, $25, $26, $27, $28, $29, $30, $31, $32, $33, $34, $35, $36,
            $37, $38, $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49
        )
        RETURNING id
        "#,
    )
        .bind(library_id)
        .bind(&payload.isbn_13)
        .bind(&payload.isbn_10)
        .bind(&payload.open_library_id)
        .bind(&payload.oclc_number)
        .bind(&payload.title)
        .bind(&payload.subtitle)
        .bind(&payload.original_title)
        .bind(&payload.authors)
        .bind(&payload.translators)
        .bind(&payload.illustrators)
        .bind(&payload.publisher)
        .bind(&payload.publish_date)
        .bind(&payload.original_publish_date)
        .bind(&payload.edition)
        .bind(&payload.edition_number)
        .bind(&payload.printing_number)
        .bind(&payload.original_edition)
        .bind(&payload.is_first_edition)
        .bind(&payload.collection_name)
        .bind(&payload.volume_in_collection)
        .bind(&payload.series_name)
        .bind(&payload.volume_in_series)
        .bind(&payload.book_format)
        .bind(&payload.page_count)
        .bind(&payload.dimensions)
        .bind(&payload.weight)
        .bind(&payload.language)
        .bind(&payload.original_language)
        .bind(&payload.subjects)
        .bind(&payload.genres)
        .bind(&payload.target_audience)
        .bind(&payload.description)
        .bind(&payload.table_of_contents)
        .bind(&payload.cover_url)
        .bind(&payload.purchase_date)
        .bind(&payload.purchase_price)
        .bind(&payload.store_or_vendor)
        .bind(&payload.acquisition_type)
        .bind(&payload.location_property)
        .bind(&payload.location_room)
        .bind(&payload.location_bookcase)
        .bind(&payload.location_shelf)
        .bind(&payload.location_position)
        .bind(&payload.condition_state)
        .bind(&payload.is_loaned)
        .bind(&payload.loaned_to)
        .bind(&payload.loan_date)
        .bind(&payload.expected_return_date)
        .fetch_one(&mut *tx)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO user_book_interactions (
            user_id, book_id, read_status, rating, personal_notes, reading_notes, date_started, date_finished
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#,
    )
        .bind(user_id)
        .bind(book_id)
        .bind(&payload.read_status.unwrap_or_else(|| "unread".to_string()))
        .bind(&payload.rating)
        .bind(&payload.personal_notes)
        .bind(&payload.reading_notes)
        .bind(&payload.date_started)
        .bind(&payload.date_finished)
        .execute(&mut *tx)
        .await?;

    let book = fetch_book_by_id(&mut tx, book_id, user_id).await?;
    tx.commit().await?;

    Ok(book)
}

pub async fn delete_book(pool: &PgPool, book_id: Uuid) -> Result<u64, sqlx::Error> {
    let book_info = sqlx::query!("SELECT cover_url FROM books WHERE id = $1", book_id)
        .fetch_optional(pool)
        .await?;

    let result = sqlx::query("DELETE FROM books WHERE id = $1")
        .bind(book_id)
        .execute(pool)
        .await?;

    if result.rows_affected() > 0 {
        if let Some(b) = book_info {
            if let Some(cover_url) = b.cover_url {
                if cover_url.starts_with("/static/") {
                    let local_path = cover_url.replace("/static/", "uploads/");
                    let _ = std::fs::remove_file(local_path);
                }
            }
        }
    }

    Ok(result.rows_affected())
}

pub async fn update_book(
    pool: &PgPool,
    book_id: Uuid,
    payload: CreateBookDto,
    user_id: Uuid,
) -> Result<Option<Book>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let library_id = match payload.library_id {
        Some(id) => id,
        None => {
            sqlx::query_scalar("SELECT library_id FROM books WHERE id = $1")
                .bind(book_id)
                .fetch_one(&mut *tx)
                .await?
        }
    };

    let update_result = sqlx::query(
        r#"
        UPDATE books SET
            library_id = $1, isbn_13 = $2, isbn_10 = $3, open_library_id = $4, oclc_number = $5, title = $6,
            subtitle = $7, original_title = $8, authors = $9, translators = $10, illustrators = $11,
            publisher = $12, publish_date = $13, original_publish_date = $14, edition = $15, edition_number = $16,
            printing_number = $17, original_edition = $18, is_first_edition = $19, collection_name = $20,
            volume_in_collection = $21, series_name = $22, volume_in_series = $23, book_format = $24,
            page_count = $25, dimensions = $26, weight = $27, language = $28, original_language = $29,
            subjects = $30, genres = $31, target_audience = $32, description = $33, table_of_contents = $34,
            cover_url = $35, purchase_date = $36, purchase_price = $37, store_or_vendor = $38,
            acquisition_type = $39, location_property = $40, location_room = $41, location_bookcase = $42,
            location_shelf = $43, location_position = $44, condition_state = $45, is_loaned = $46,
            loaned_to = $47, loan_date = $48, expected_return_date = $49
        WHERE id = $50
        "#
    )
        .bind(library_id)
        .bind(&payload.isbn_13)
        .bind(&payload.isbn_10)
        .bind(&payload.open_library_id)
        .bind(&payload.oclc_number)
        .bind(&payload.title)
        .bind(&payload.subtitle)
        .bind(&payload.original_title)
        .bind(&payload.authors)
        .bind(&payload.translators)
        .bind(&payload.illustrators)
        .bind(&payload.publisher)
        .bind(&payload.publish_date)
        .bind(&payload.original_publish_date)
        .bind(&payload.edition)
        .bind(&payload.edition_number)
        .bind(&payload.printing_number)
        .bind(&payload.original_edition)
        .bind(&payload.is_first_edition)
        .bind(&payload.collection_name)
        .bind(&payload.volume_in_collection)
        .bind(&payload.series_name)
        .bind(&payload.volume_in_series)
        .bind(&payload.book_format)
        .bind(&payload.page_count)
        .bind(&payload.dimensions)
        .bind(&payload.weight)
        .bind(&payload.language)
        .bind(&payload.original_language)
        .bind(&payload.subjects)
        .bind(&payload.genres)
        .bind(&payload.target_audience)
        .bind(&payload.description)
        .bind(&payload.table_of_contents)
        .bind(&payload.cover_url)
        .bind(&payload.purchase_date)
        .bind(&payload.purchase_price)
        .bind(&payload.store_or_vendor)
        .bind(&payload.acquisition_type)
        .bind(&payload.location_property)
        .bind(&payload.location_room)
        .bind(&payload.location_bookcase)
        .bind(&payload.location_shelf)
        .bind(&payload.location_position)
        .bind(&payload.condition_state)
        .bind(&payload.is_loaned)
        .bind(&payload.loaned_to)
        .bind(&payload.loan_date)
        .bind(&payload.expected_return_date)
        .bind(book_id)
        .execute(&mut *tx)
        .await?;

    if update_result.rows_affected() == 0 {
        return Ok(None);
    }

    sqlx::query(
        r#"
        INSERT INTO user_book_interactions (user_id, book_id, read_status, rating, personal_notes, reading_notes, date_started, date_finished)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        ON CONFLICT (user_id, book_id) DO UPDATE SET
            read_status = EXCLUDED.read_status,
            rating = EXCLUDED.rating,
            personal_notes = EXCLUDED.personal_notes,
            reading_notes = EXCLUDED.reading_notes,
            date_started = EXCLUDED.date_started,
            date_finished = EXCLUDED.date_finished
        "#
    )
        .bind(user_id)
        .bind(book_id)
        .bind(&payload.read_status.unwrap_or_else(|| "unread".to_string()))
        .bind(&payload.rating)
        .bind(&payload.personal_notes)
        .bind(&payload.reading_notes)
        .bind(&payload.date_started)
        .bind(&payload.date_finished)
        .execute(&mut *tx)
        .await?;

    let book = fetch_book_by_id(&mut tx, book_id, user_id).await?;
    tx.commit().await?;

    Ok(Some(book))
}

pub async fn delete_books_batch(pool: &PgPool, book_ids: Vec<Uuid>) -> Result<u64, sqlx::Error> {
    let books_info = sqlx::query!("SELECT cover_url FROM books WHERE id = ANY($1)", &book_ids)
        .fetch_all(pool)
        .await?;

    let result = sqlx::query("DELETE FROM books WHERE id = ANY($1)")
        .bind(&book_ids)
        .execute(pool)
        .await?;

    if result.rows_affected() > 0 {
        for b in books_info {
            if let Some(cover_url) = b.cover_url {
                if cover_url.starts_with("/static/") {
                    let local_path = cover_url.replace("/static/", "uploads/");
                    let _ = std::fs::remove_file(local_path);
                }
            }
        }
    }

    Ok(result.rows_affected())
}

pub async fn fetch_all_for_export(
    pool: &PgPool,
    query_params: &BookFilterQuery,
    specific_ids: Option<Vec<Uuid>>,
    user_id: Uuid,
) -> Result<Vec<Book>, sqlx::Error> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new(
        "SELECT b.*, ubi.read_status, ubi.rating, ubi.personal_notes, ubi.reading_notes, ubi.date_started, ubi.date_finished \
         FROM books b \
         LEFT JOIN user_book_interactions ubi ON b.id = ubi.book_id AND ubi.user_id = "
    );
    query.push_bind(user_id);
    query.push(" WHERE b.library_id IN (SELECT l.id FROM libraries l LEFT JOIN library_members lm ON l.id = lm.library_id WHERE l.owner_id = ");
    query.push_bind(user_id);
    query.push(" OR lm.user_id = ");
    query.push_bind(user_id);
    query.push(") ");

    if let Some(ids) = specific_ids {
        if !ids.is_empty() {
            query.push(" AND b.id = ANY(");
            query.push_bind(ids);
            query.push(") ");
        } else {
            query.push(" AND 1=0 ");
        }
    } else {
        apply_filters(query_params, &mut query);
    }

    apply_sorting(query_params, &mut query);

    query.build_query_as::<Book>().fetch_all(pool).await
}

pub async fn patch_book(
    pool: &PgPool,
    book_id: Uuid,
    payload: UpdateBookPartialDto,
    user_id: Uuid,
) -> Result<Option<Book>, sqlx::Error> {
    let mut tx = pool.begin().await?;

    let exists: Option<Uuid> = sqlx::query_scalar("SELECT id FROM books WHERE id = $1")
        .bind(book_id)
        .fetch_optional(&mut *tx)
        .await?;

    if exists.is_none() {
        return Ok(None);
    }

    let mut query = QueryBuilder::new("UPDATE books SET ");
    let mut has_book_fields = false;

    macro_rules! bind_book_field {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = payload.$field {
                if has_book_fields { query.push(", "); }
                query.push(concat!($col, " = ")).push_bind(val);
                has_book_fields = true;
            }
        };
    }

    bind_book_field!(library_id, "library_id");
    bind_book_field!(title, "title");
    bind_book_field!(subtitle, "subtitle");
    bind_book_field!(original_title, "original_title");
    bind_book_field!(authors, "authors");
    bind_book_field!(translators, "translators");
    bind_book_field!(illustrators, "illustrators");
    bind_book_field!(publisher, "publisher");
    bind_book_field!(publish_date, "publish_date");
    bind_book_field!(original_publish_date, "original_publish_date");
    bind_book_field!(edition, "edition");
    bind_book_field!(edition_number, "edition_number");
    bind_book_field!(printing_number, "printing_number");
    bind_book_field!(original_edition, "original_edition");
    bind_book_field!(is_first_edition, "is_first_edition");
    bind_book_field!(collection_name, "collection_name");
    bind_book_field!(volume_in_collection, "volume_in_collection");
    bind_book_field!(series_name, "series_name");
    bind_book_field!(volume_in_series, "volume_in_series");
    bind_book_field!(book_format, "book_format");
    bind_book_field!(page_count, "page_count");
    bind_book_field!(dimensions, "dimensions");
    bind_book_field!(weight, "weight");
    bind_book_field!(language, "language");
    bind_book_field!(original_language, "original_language");
    bind_book_field!(subjects, "subjects");
    bind_book_field!(genres, "genres");
    bind_book_field!(target_audience, "target_audience");
    bind_book_field!(description, "description");
    bind_book_field!(table_of_contents, "table_of_contents");
    bind_book_field!(cover_url, "cover_url");
    bind_book_field!(purchase_date, "purchase_date");
    bind_book_field!(purchase_price, "purchase_price");
    bind_book_field!(store_or_vendor, "store_or_vendor");
    bind_book_field!(acquisition_type, "acquisition_type");
    bind_book_field!(location_property, "location_property");
    bind_book_field!(location_room, "location_room");
    bind_book_field!(location_bookcase, "location_bookcase");
    bind_book_field!(location_shelf, "location_shelf");
    bind_book_field!(location_position, "location_position");
    bind_book_field!(condition_state, "condition_state");
    bind_book_field!(is_loaned, "is_loaned");
    bind_book_field!(loaned_to, "loaned_to");
    bind_book_field!(loan_date, "loan_date");
    bind_book_field!(expected_return_date, "expected_return_date");

    if has_book_fields {
        query.push(", updated_at = NOW() WHERE id = ").push_bind(book_id);
        query.build().execute(&mut *tx).await?;
    }

    let mut ubi_query = QueryBuilder::new("UPDATE user_book_interactions SET ");
    let mut has_ubi_fields = false;

    macro_rules! bind_ubi_field {
        ($field:ident, $col:expr) => {
            if let Some(ref val) = payload.$field {
                if has_ubi_fields { ubi_query.push(", "); }
                ubi_query.push(concat!($col, " = ")).push_bind(val);
                has_ubi_fields = true;
            }
        };
    }

    bind_ubi_field!(read_status, "read_status");
    bind_ubi_field!(rating, "rating");
    bind_ubi_field!(personal_notes, "personal_notes");
    bind_ubi_field!(reading_notes, "reading_notes");
    bind_ubi_field!(date_started, "date_started");
    bind_ubi_field!(date_finished, "date_finished");

    if has_ubi_fields {
        ubi_query.push(" WHERE user_id = ").push_bind(user_id);
        ubi_query.push(" AND book_id = ").push_bind(book_id);
        ubi_query.build().execute(&mut *tx).await?;
    }

    let book = fetch_book_by_id(&mut tx, book_id, user_id).await?;
    tx.commit().await?;

    Ok(Some(book))
}

pub async fn fetch_autocomplete_suggestions(
    pool: &PgPool,
    field: &str,
    q: &str,
    limit: Option<i64>,
    user_id: Uuid,
) -> Result<Vec<String>, sqlx::Error> {
    let limit_val = limit.unwrap_or(10);

    let allowed_scalar_fields = [
        "publisher", "collection_name", "series_name", "language",
        "original_language", "book_format", "store_or_vendor", "acquisition_type",
        "title", "subtitle", "original_title", "condition_state", "location_property",
        "location_room", "location_bookcase", "location_shelf", "location_position"
    ];
    let allowed_array_fields = ["authors", "translators", "illustrators", "subjects", "genres"];

    let mut query = QueryBuilder::new("");

    if allowed_scalar_fields.contains(&field) {
        query.push("SELECT field_val FROM (SELECT ");
        query.push(field);
        query.push(" as field_val, count(");
        query.push(field);
        query.push(") as c FROM books WHERE library_id IN (SELECT l.id FROM libraries l LEFT JOIN library_members lm ON l.id = lm.library_id WHERE l.owner_id = ");
        query.push_bind(user_id);
        query.push(" OR lm.user_id = ");
        query.push_bind(user_id);
        query.push(") AND ");
        query.push(field);
        query.push(" IS NOT NULL AND ");
        query.push(field);
        query.push(" != '' AND ");
        query.push(field);
        query.push(" ILIKE ");
        query.push_bind(format!("%{}%", q));
        query.push(" GROUP BY ");
        query.push(field);
        query.push(" ORDER BY c DESC LIMIT ");
        query.push_bind(limit_val);
        query.push(") sub ORDER BY field_val ASC");
        
        let result: Vec<String> = query.build_query_scalar().fetch_all(pool).await?;
        return Ok(result);
    } else if allowed_array_fields.contains(&field) {
        query.push("SELECT unnest_val FROM (SELECT unnest_val, count(unnest_val) as c FROM (SELECT unnest(");
        query.push(field);
        query.push(") as unnest_val FROM books WHERE library_id IN (SELECT l.id FROM libraries l LEFT JOIN library_members lm ON l.id = lm.library_id WHERE l.owner_id = ");
        query.push_bind(user_id);
        query.push(" OR lm.user_id = ");
        query.push_bind(user_id);
        query.push(")) as unnested WHERE unnest_val ILIKE ");
        query.push_bind(format!("%{}%", q));
        query.push(" GROUP BY unnest_val ORDER BY c DESC LIMIT ");
        query.push_bind(limit_val);
        query.push(") sub ORDER BY unnest_val ASC");

        let result: Vec<String> = query.build_query_scalar().fetch_all(pool).await?;
        return Ok(result);
    } else {
        return Ok(vec![]);
    }
}

pub async fn get_libraries(pool: &PgPool, user_id: Uuid) -> Result<Vec<Library>, sqlx::Error> {
    sqlx::query_as::<_, Library>(
        "SELECT l.* FROM libraries l 
         LEFT JOIN library_members lm ON l.id = lm.library_id 
         WHERE l.owner_id = $1 OR lm.user_id = $1
         GROUP BY l.id ORDER BY l.created_at ASC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

pub async fn create_library(pool: &PgPool, payload: CreateLibraryDto, user_id: Uuid) -> Result<Library, sqlx::Error> {
    sqlx::query_as::<_, Library>(
        "INSERT INTO libraries (name, description, owner_id) 
         VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(payload.name)
    .bind(payload.description)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn update_library(pool: &PgPool, library_id: Uuid, payload: UpdateLibraryDto, user_id: Uuid) -> Result<Library, sqlx::Error> {
    sqlx::query_as::<_, Library>(
        "UPDATE libraries SET 
            name = COALESCE($1, name),
            description = COALESCE($2, description),
            updated_at = NOW()
         WHERE id = $3 AND owner_id = $4 RETURNING *"
    )
    .bind(payload.name)
    .bind(payload.description)
    .bind(library_id)
    .bind(user_id)
    .fetch_one(pool)
    .await
}

pub async fn delete_library(pool: &PgPool, library_id: Uuid, user_id: Uuid) -> Result<u64, sqlx::Error> {
    let result = sqlx::query("DELETE FROM libraries WHERE id = $1 AND owner_id = $2")
        .bind(library_id)
        .bind(user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn get_library_members(pool: &PgPool, library_id: Uuid, requester_id: Uuid) -> Result<Vec<LibraryMember>, sqlx::Error> {
    let is_authorized = check_library_permission(pool, library_id, requester_id, vec!["owner", "editor", "viewer"]).await?;
    if !is_authorized {
        return Err(sqlx::Error::RowNotFound);
    }

    sqlx::query_as::<_, LibraryMember>(
        "SELECT lm.library_id, lm.user_id, lm.role, lm.created_at, u.username 
         FROM library_members lm 
         JOIN users u ON lm.user_id = u.id 
         WHERE lm.library_id = $1"
    )
    .bind(library_id)
    .fetch_all(pool)
    .await
}

pub async fn add_library_member(pool: &PgPool, library_id: Uuid, payload: ShareLibraryDto, owner_id: Uuid) -> Result<LibraryMember, sqlx::Error> {
    let is_owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM libraries WHERE id = $1 AND owner_id = $2)")
        .bind(library_id).bind(owner_id).fetch_one(pool).await?;
    if !is_owner { return Err(sqlx::Error::RowNotFound); }

    let target_user_id: Option<Uuid> = sqlx::query_scalar("SELECT id FROM users WHERE username = $1")
        .bind(&payload.username).fetch_optional(pool).await?;
    
    let target_user_id = match target_user_id {
        Some(id) => id,
        None => return Err(sqlx::Error::RowNotFound),
    };

    sqlx::query_as::<_, LibraryMember>(
        "INSERT INTO library_members (library_id, user_id, role) 
         VALUES ($1, $2, $3) 
         ON CONFLICT (library_id, user_id) DO UPDATE SET role = EXCLUDED.role 
         RETURNING library_id, user_id, role, created_at, $4 as username"
    )
    .bind(library_id)
    .bind(target_user_id)
    .bind(payload.role)
    .bind(payload.username)
    .fetch_one(pool)
    .await
}

pub async fn remove_library_member(pool: &PgPool, library_id: Uuid, target_user_id: Uuid, owner_id: Uuid) -> Result<u64, sqlx::Error> {
    let is_owner: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM libraries WHERE id = $1 AND owner_id = $2)")
        .bind(library_id).bind(owner_id).fetch_one(pool).await?;
    if !is_owner { return Err(sqlx::Error::RowNotFound); }

    let result = sqlx::query("DELETE FROM library_members WHERE library_id = $1 AND user_id = $2")
        .bind(library_id)
        .bind(target_user_id)
        .execute(pool)
        .await?;
    Ok(result.rows_affected())
}

pub async fn check_library_permission(pool: &PgPool, library_id: Uuid, user_id: Uuid, allowed_roles: Vec<&str>) -> Result<bool, sqlx::Error> {
    let owner_id: Option<Uuid> = sqlx::query_scalar("SELECT owner_id FROM libraries WHERE id = $1")
        .bind(library_id)
        .fetch_optional(pool)
        .await?;
    
    if let Some(owner) = owner_id {
        if owner == user_id && allowed_roles.contains(&"owner") {
            return Ok(true);
        }
    } else {
        return Ok(false);
    }

    let role: Option<String> = sqlx::query_scalar("SELECT role FROM library_members WHERE library_id = $1 AND user_id = $2")
        .bind(library_id)
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

    if let Some(r) = role {
        if allowed_roles.contains(&r.as_str()) {
            return Ok(true);
        }
    }
    
    Ok(false)
}