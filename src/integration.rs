use crate::models::BookMetadataResponse;
use serde_json::Value;

pub async fn fetch_metadata_by_isbn(isbn: &str) -> Result<BookMetadataResponse, reqwest::Error> {
    let url = format!(
        "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=data",
        isbn
    );
    let response = reqwest::get(&url).await?;
    let raw_data: Value = response.json().await?;
    let bibkey = format!("ISBN:{}", isbn);

    if raw_data.get(&bibkey).is_none() {
        return Ok(BookMetadataResponse {
            isbn: Some(isbn.to_string()),
            title: None,
            authors: None,
            publish_date: None,
            page_count: None,
            cover_url: None,
        });
    }

    let book_data = &raw_data[&bibkey];
    let title = book_data
        .get("title")
        .and_then(|t| t.as_str())
        .map(String::from);
    let publish_date = book_data
        .get("publish_date")
        .and_then(|d| d.as_str())
        .map(String::from);
    let page_count = book_data
        .get("number_of_pages")
        .and_then(|p| p.as_i64())
        .map(|p| p as i32);
    let cover_url = book_data
        .get("cover")
        .and_then(|c| c.get("large"))
        .and_then(|url| url.as_str())
        .map(String::from);

    let mut authors_list = Vec::new();
    if let Some(authors_array) = book_data.get("authors").and_then(|a| a.as_array()) {
        for author in authors_array {
            if let Some(name) = author.get("name").and_then(|n| n.as_str()) {
                authors_list.push(name.to_string());
            }
        }
    }

    let authors = if authors_list.is_empty() {
        None
    } else {
        Some(authors_list)
    };

    Ok(BookMetadataResponse {
        isbn: Some(isbn.to_string()),
        title,
        authors,
        publish_date,
        page_count,
        cover_url,
    })
}

pub async fn search_metadata_by_query(
    query: &str,
) -> Result<Vec<BookMetadataResponse>, reqwest::Error> {
    let url = format!("https://openlibrary.org/search.json?q={}&limit=5", query);
    let response = reqwest::get(&url).await?;
    let raw_data: Value = response.json().await?;

    let mut results = Vec::new();

    if let Some(docs) = raw_data.get("docs").and_then(|d| d.as_array()) {
        for doc in docs {
            let title = doc.get("title").and_then(|t| t.as_str()).map(String::from);
            let publish_date = doc
                .get("first_publish_year")
                .and_then(|y| y.as_i64())
                .map(|y| y.to_string());
            let page_count = doc
                .get("number_of_pages_median")
                .and_then(|p| p.as_i64())
                .map(|p| p as i32);

            let mut authors_list = Vec::new();
            if let Some(authors_array) = doc.get("author_name").and_then(|a| a.as_array()) {
                for author in authors_array {
                    if let Some(name) = author.as_str() {
                        authors_list.push(name.to_string());
                    }
                }
            }
            let authors = if authors_list.is_empty() {
                None
            } else {
                Some(authors_list)
            };

            let mut isbn_str = None;
            if let Some(isbns) = doc.get("isbn").and_then(|i| i.as_array())
                && let Some(first_isbn) = isbns.first().and_then(|i| i.as_str())
            {
                isbn_str = Some(first_isbn.to_string());
            }

            let mut cover_url = None;
            if let Some(cover_i) = doc.get("cover_i").and_then(|c| c.as_i64()) {
                cover_url = Some(format!(
                    "https://covers.openlibrary.org/b/id/{}-L.jpg",
                    cover_i
                ));
            }

            results.push(BookMetadataResponse {
                isbn: isbn_str,
                title,
                authors,
                publish_date,
                page_count,
                cover_url,
            });
        }
    }

    Ok(results)
}
