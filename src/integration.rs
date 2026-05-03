use crate::models::BookMetadataResponse;
use serde_json::Value;

pub async fn fetch_metadata(identifier: &str) -> Result<BookMetadataResponse, reqwest::Error> {
    let clean_id = identifier.replace("-", "").replace(" ", "").to_uppercase();

    let bibkey = if clean_id.starts_with("OL") {
        format!("OLID:{}", clean_id)
    } else if clean_id.starts_with("OCLC") {
        let numeric_oclc = clean_id.replace("OCLC", "");
        format!("OCLC:{}", numeric_oclc)
    } else if clean_id.len() == 10 || clean_id.len() == 13 {
        format!("ISBN:{}", clean_id)
    } else {
        format!("OCLC:{}", clean_id)
    };

    let url = format!(
        "https://openlibrary.org/api/books?bibkeys={}&format=json&jscmd=data",
        bibkey
    );

    let response = reqwest::get(&url).await?;
    let raw_data: Value = response.json().await?;

    let book_data = match raw_data.as_object().and_then(|obj| obj.values().next()) {
        Some(data) => data,
        None => {
            return Ok(BookMetadataResponse {
                isbn: Some(identifier.to_string()),
                title: None,
                authors: None,
                publish_date: None,
                page_count: None,
                cover_url: None,
                subtitle: None,
                publishers: None,
                physical_format: None,
                weight: None,
                dimensions: None,
                subjects: None,
                languages: None,
            });
        }
    };

    let title = book_data
        .get("title")
        .and_then(|t| t.as_str())
        .map(String::from);
    let subtitle = book_data
        .get("subtitle")
        .and_then(|s| s.as_str())
        .map(String::from);
    let publish_date = book_data
        .get("publish_date")
        .and_then(|d| d.as_str())
        .map(String::from);
    let page_count = book_data
        .get("number_of_pages")
        .and_then(|p| p.as_i64())
        .map(|p| p as i32);
    let physical_format = book_data
        .get("physical_format")
        .and_then(|f| f.as_str())
        .map(String::from);
    let weight = book_data
        .get("weight")
        .and_then(|w| w.as_str())
        .map(String::from);
    let dimensions = book_data
        .get("physical_dimensions")
        .and_then(|d| d.as_str())
        .map(String::from);

    let cover_url = book_data
        .get("cover")
        .and_then(|c| c.get("large"))
        .and_then(|url| url.as_str())
        .map(String::from);

    let extract_name_array = |key: &str| -> Option<Vec<String>> {
        let mut list = Vec::new();
        if let Some(arr) = book_data.get(key).and_then(|a| a.as_array()) {
            for item in arr {
                if let Some(name) = item.get("name").and_then(|n| n.as_str()) {
                    list.push(name.to_string());
                }
            }
        }
        if list.is_empty() { None } else { Some(list) }
    };

    let authors = extract_name_array("authors");
    let publishers = extract_name_array("publishers");
    let subjects = extract_name_array("subjects");

    let mut languages_list = Vec::new();
    if let Some(lang_arr) = book_data.get("languages").and_then(|l| l.as_array()) {
        for lang in lang_arr {
            if let Some(key) = lang.get("key").and_then(|k| k.as_str()) {
                let parsed = key.replace("/languages/", "");
                languages_list.push(parsed);
            }
        }
    }
    let languages = if languages_list.is_empty() {
        None
    } else {
        Some(languages_list)
    };

    Ok(BookMetadataResponse {
        isbn: Some(identifier.to_string()),
        title,
        authors,
        publish_date,
        page_count,
        cover_url,
        subtitle,
        publishers,
        physical_format,
        weight,
        dimensions,
        subjects,
        languages,
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
            if let Some(isbns) = doc.get("isbn").and_then(|i| i.as_array()) {
                if let Some(first_isbn) = isbns.first().and_then(|i| i.as_str()) {
                    isbn_str = Some(first_isbn.to_string());
                }
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
                subtitle: None,
                publishers: None,
                physical_format: None,
                weight: None,
                dimensions: None,
                subjects: None,
                languages: None,
            });
        }
    }

    Ok(results)
}
