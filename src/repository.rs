use crate::models::Book;
use sqlx::PgPool;

pub async fn fetch_all_books(pool: &PgPool) -> Result<Vec<Book>, sqlx::Error> {
    let books = sqlx::query_as::<_, Book>("SELECT * FROM books")
        .fetch_all(pool)
        .await?;

    Ok(books)
}
