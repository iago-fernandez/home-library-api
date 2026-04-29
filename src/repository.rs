use crate::models::{Book, BookFilterQuery, CreateBookDto, PaginatedBooks, QueryAST};
use sqlx::{PgPool, Postgres, QueryBuilder};
use uuid::Uuid;

pub async fn fetch_books(
    pool: &PgPool,
    query_params: BookFilterQuery,
) -> Result<PaginatedBooks, sqlx::Error> {
    let mut query: QueryBuilder<Postgres> = QueryBuilder::new("SELECT * FROM books WHERE 1=1");

    if let Some(query_str) = &query_params.query {
        if let Ok(ast) = serde_json::from_str::<QueryAST>(query_str) {
            query.push(" AND (");
            build_query_recursive(&ast, &mut query);
            query.push(")");
        }
    }

    let allowed_sort_columns = [
        "title",
        "page_count",
        "rating",
        "publish_date",
        "created_at",
        "updated_at",
        "purchase_price",
        "authors",
        "publisher",
        "isbn_13",
        "location_room",
        "location_bookcase",
    ];

    let sort_col = query_params
        .sort_by
        .unwrap_or_else(|| "created_at".to_string());

    let final_sort_col = if allowed_sort_columns.contains(&sort_col.as_str()) {
        sort_col
    } else {
        "created_at".to_string()
    };

    let order = if query_params.sort_order.as_deref() == Some("asc") {
        "ASC"
    } else {
        "DESC"
    };

    query.push(format!(" ORDER BY {} {} ", final_sort_col, order));

    let limit = query_params.limit.unwrap_or(50).clamp(1, 100);
    let offset = query_params.offset.unwrap_or(0).max(0);

    query.push(" LIMIT ").push_bind(limit);
    query.push(" OFFSET ").push_bind(offset);

    let books = query.build_query_as::<Book>().fetch_all(pool).await?;

    let total_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM books")
        .fetch_one(pool)
        .await?;

    Ok(PaginatedBooks {
        data: books,
        total: total_count.0,
    })
}

fn build_query_recursive(ast: &QueryAST, query: &mut QueryBuilder<Postgres>) {
    match ast {
        QueryAST::Condition { field, operator, value } => {
            apply_condition(field, operator, value, query);
        }
        QueryAST::And { nodes } => {
            query.push("(");
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    query.push(" AND ");
                }
                build_query_recursive(node, query);
            }
            query.push(")");
        }
        QueryAST::Or { nodes } => {
            query.push("(");
            for (i, node) in nodes.iter().enumerate() {
                if i > 0 {
                    query.push(" OR ");
                }
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

fn apply_condition(field: &str, operator: &str, value: &str, query: &mut QueryBuilder<Postgres>) {
    let text_columns = [
        "title", "subtitle", "original_title", "publisher", "collection_name", "series_name",
        "description", "personal_notes", "reading_notes", "location_property", "location_room",
        "location_bookcase", "location_shelf",
    ];

    let exact_string_columns = [
        "read_status", "book_format", "condition_state", "target_audience", "language",
        "original_language", "store_or_vendor", "acquisition_type", "isbn_13", "isbn_10",
    ];

    let numeric_columns = ["page_count", "rating", "volume_in_collection", "volume_in_series"];

    let date_columns = [
        "publish_date", "original_publish_date", "purchase_date", "date_started", "date_finished",
    ];

    if field == "search" {
        let term = format!("%{}%", value);
        query.push(" (title ILIKE ");
        query.push_bind(term.clone());
        query.push(" OR original_title ILIKE ");
        query.push_bind(term);
        query.push(") ");
        return;
    }

    if field == "author" || field == "authors" {
        query.push(" array_to_string(authors, ', ') ILIKE ");
        query.push_bind(format!("%{}%", value));
        return;
    }

    if text_columns.contains(&field) || exact_string_columns.contains(&field) {
        match operator {
            "_contains" => {
                query.push(format!(" {} ILIKE ", field));
                query.push_bind(format!("%{}%", value));
            }
            "_contains_case" => {
                query.push(format!(" {} LIKE ", field));
                query.push_bind(format!("%{}%", value));
            }
            "_starts" => {
                query.push(format!(" {} ILIKE ", field));
                query.push_bind(format!("{}%", value));
            }
            "_starts_case" => {
                query.push(format!(" {} LIKE ", field));
                query.push_bind(format!("{}%", value));
            }
            "_ends" => {
                query.push(format!(" {} ILIKE ", field));
                query.push_bind(format!("%{}", value));
            }
            "_ends_case" => {
                query.push(format!(" {} LIKE ", field));
                query.push_bind(format!("%{}", value));
            }
            "_exact" => {
                query.push(format!(" {} = ", field));
                query.push_bind(value.to_string());
            }
            "_empty" => {
                if value == "true" {
                    query.push(format!(" ({} IS NULL OR {} = '') ", field, field));
                } else {
                    query.push(format!(" ({} IS NOT NULL AND {} != '') ", field, field));
                }
            }
            _ => {
                query.push(format!(" {} ILIKE ", field));
                query.push_bind(format!("%{}%", value));
            }
        }
    } else if numeric_columns.contains(&field) {
        if let Ok(num_val) = value.parse::<i32>() {
            match operator {
                "_gt" => {
                    query.push(format!(" {} > ", field));
                    query.push_bind(num_val);
                }
                "_gte" => {
                    query.push(format!(" {} >= ", field));
                    query.push_bind(num_val);
                }
                "_lt" => {
                    query.push(format!(" {} < ", field));
                    query.push_bind(num_val);
                }
                "_lte" => {
                    query.push(format!(" {} <= ", field));
                    query.push_bind(num_val);
                }
                _ => {
                    query.push(format!(" {} = ", field));
                    query.push_bind(num_val);
                }
            }
        } else {
            query.push(" 1=0 ");
        }
    } else if date_columns.contains(&field) {
        if let Ok(date_val) = chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d") {
            match operator {
                "_gt" => {
                    query.push(format!(" {} > ", field));
                    query.push_bind(date_val);
                }
                "_gte" => {
                    query.push(format!(" {} >= ", field));
                    query.push_bind(date_val);
                }
                "_lt" => {
                    query.push(format!(" {} < ", field));
                    query.push_bind(date_val);
                }
                "_lte" => {
                    query.push(format!(" {} <= ", field));
                    query.push_bind(date_val);
                }
                _ => {
                    query.push(format!(" {} = ", field));
                    query.push_bind(date_val);
                }
            }
        } else {
            query.push(" 1=0 ");
        }
    } else {
        query.push(" 1=1 ");
    }
}

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

pub async fn update_book(
    pool: &PgPool,
    book_id: Uuid,
    payload: CreateBookDto,
) -> Result<Option<Book>, sqlx::Error> {
    let book = sqlx::query_as::<_, Book>(
        r#"
        UPDATE books SET
            isbn_13 = $1, isbn_10 = $2, open_library_id = $3, oclc_number = $4, title = $5,
            subtitle = $6, original_title = $7, authors = $8, translators = $9, illustrators = $10,
            publisher = $11, publish_date = $12, original_publish_date = $13, edition_number = $14,
            printing_number = $15, original_edition = $16, is_first_edition = $17, collection_name = $18,
            volume_in_collection = $19, series_name = $20, volume_in_series = $21, book_format = $22,
            page_count = $23, dimensions = $24, weight = $25, language = $26, original_language = $27,
            subjects = $28, genres = $29, target_audience = $30, description = $31, table_of_contents = $32,
            cover_url = $33, purchase_date = $34, purchase_price = $35, store_or_vendor = $36,
            acquisition_type = $37, location_property = $38, location_room = $39, location_bookcase = $40,
            location_shelf = $41, location_position = $42, condition_state = $43, personal_notes = $44,
            read_status = $45, rating = $46, date_started = $47, date_finished = $48, reading_notes = $49,
            is_loaned = $50, loaned_to = $51, loan_date = $52, expected_return_date = $53,
            updated_at = CURRENT_TIMESTAMP
        WHERE id = $54
        RETURNING *
        "#
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
        .bind(book_id)
        .fetch_optional(pool)
        .await?;

    Ok(book)
}