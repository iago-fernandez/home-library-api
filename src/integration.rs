use std::env;
use crate::models::BookMetadataResponse;
use serde_json::Value;
use std::sync::OnceLock;


fn http_client() -> reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        let user_agent = format!("{}/{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        reqwest::Client::builder()
            .user_agent(user_agent)
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .unwrap()
    }).clone()
}


async fn fetch_google_books_metadata(identifier: &str) -> Result<Option<BookMetadataResponse>, reqwest::Error> {
    let clean_id = identifier.replace("-", "").replace(" ", "").to_uppercase();
    
    // Google books mostly supports ISBNs and general search well. OCLC/OLID are OpenLibrary specific.
    // If it's an OLID, Google Books probably won't find it via q=isbn: but we can try q= clean_id.
    let query = if clean_id.starts_with("OL") || clean_id.starts_with("OCLC") {
        clean_id.clone()
    } else {
        format!("isbn:{}", clean_id)
    };

    let mut url = format!("https://www.googleapis.com/books/v1/volumes?q={}", query);
    if let Ok(api_key) = env::var("GOOGLE_BOOKS_API_KEY") {
        url = format!("{}&key={}", url, api_key);
    }
    
    let response = http_client().get(&url).send().await?;
    let raw_data: Value = response.json().await?;

    if let Some(items) = raw_data.get("items").and_then(|i| i.as_array()) {
        if let Some(first_item) = items.first() {
            if let Some(vol_info) = first_item.get("volumeInfo") {
                let title = vol_info.get("title").and_then(|t| t.as_str()).map(String::from);
                let subtitle = vol_info.get("subtitle").and_then(|s| s.as_str()).map(String::from);
                let publish_date = vol_info.get("publishedDate").and_then(|d| d.as_str()).map(String::from);
                let page_count = vol_info.get("pageCount").and_then(|p| p.as_i64()).map(|p| p as i32);
                // let description = vol_info.get("description").and_then(|d| d.as_str()).map(String::from);
                
                let cover_url = vol_info.get("imageLinks")
                    .and_then(|imgs| imgs.get("thumbnail").or(imgs.get("smallThumbnail")))
                    .and_then(|url| url.as_str())
                    .map(|url| url.replace("http:", "https:")); // Ensure HTTPS

                let extract_array = |key: &str| -> Option<Vec<String>> {
                    let mut list = Vec::new();
                    if let Some(arr) = vol_info.get(key).and_then(|a| a.as_array()) {
                        for item in arr {
                            if let Some(name) = item.as_str() {
                                list.push(name.to_string());
                            }
                        }
                    }
                    if list.is_empty() { None } else { Some(list) }
                };

                let authors = extract_array("authors");
                let publishers = vol_info.get("publisher").and_then(|p| p.as_str()).map(|p| vec![p.to_string()]);
                let subjects = extract_array("categories");
                let language = vol_info.get("language").and_then(|l| l.as_str()).map(|l| vec![l.to_string()]);

                return Ok(Some(BookMetadataResponse {
                    isbn: Some(identifier.to_string()),
                    title,
                    authors,
                    publish_date,
                    page_count,
                    cover_url,
                    subtitle,
                    publishers,
                    physical_format: None,
                    weight: None,
                    dimensions: None,
                    subjects,
                    languages: language,
                }));
            }
        }
    }
    
    Ok(None)
}

async fn fetch_openlibrary_metadata(identifier: &str) -> Result<BookMetadataResponse, reqwest::Error> {

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

    let response = http_client().get(&url).send().await?;
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


pub async fn fetch_metadata(identifier: &str) -> Result<BookMetadataResponse, reqwest::Error> {
    // 1. Try Google Books First
    if let Ok(Some(metadata)) = fetch_google_books_metadata(identifier).await {
        if metadata.title.is_some() {
            return Ok(metadata);
        }
    }
    
    // 2. Fallback to OpenLibrary
    fetch_openlibrary_metadata(identifier).await
}

async fn search_google_books_metadata(query: &str) -> Result<Vec<BookMetadataResponse>, reqwest::Error> {
    let mut url = format!("https://www.googleapis.com/books/v1/volumes?q={}&maxResults=5", query);
    if let Ok(api_key) = env::var("GOOGLE_BOOKS_API_KEY") {
        url = format!("{}&key={}", url, api_key);
    }
    let response = http_client().get(&url).send().await?;
    let raw_data: Value = response.json().await?;

    let mut results = Vec::new();

    if let Some(items) = raw_data.get("items").and_then(|i| i.as_array()) {
        for item in items {
            if let Some(vol_info) = item.get("volumeInfo") {
                let title = vol_info.get("title").and_then(|t| t.as_str()).map(String::from);
                let publish_date = vol_info.get("publishedDate").and_then(|d| d.as_str()).map(|d| d.to_string());
                let page_count = vol_info.get("pageCount").and_then(|p| p.as_i64()).map(|p| p as i32);
                
                let extract_array = |key: &str| -> Option<Vec<String>> {
                    let mut list = Vec::new();
                    if let Some(arr) = vol_info.get(key).and_then(|a| a.as_array()) {
                        for item in arr {
                            if let Some(name) = item.as_str() {
                                list.push(name.to_string());
                            }
                        }
                    }
                    if list.is_empty() { None } else { Some(list) }
                };

                let authors = extract_array("authors");
                
                let mut isbn_str = None;
                if let Some(identifiers) = vol_info.get("industryIdentifiers").and_then(|i| i.as_array()) {
                    for id_obj in identifiers {
                        if let Some(id_type) = id_obj.get("type").and_then(|t| t.as_str()) {
                            if id_type == "ISBN_13" || id_type == "ISBN_10" {
                                if let Some(id_val) = id_obj.get("identifier").and_then(|i| i.as_str()) {
                                    isbn_str = Some(id_val.to_string());
                                    break;
                                }
                            }
                        }
                    }
                }

                let cover_url = vol_info.get("imageLinks")
                    .and_then(|imgs| imgs.get("thumbnail").or(imgs.get("smallThumbnail")))
                    .and_then(|url| url.as_str())
                    .map(|url| url.replace("http:", "https:"));

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
    }
    Ok(results)
}

async fn search_openlibrary_metadata(query: &str) -> Result<Vec<BookMetadataResponse>, reqwest::Error> {

    let url = format!("https://openlibrary.org/search.json?q={}&limit=5", query);
    let response = http_client().get(&url).send().await?;
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

pub async fn search_metadata_by_query(query: &str) -> Result<Vec<BookMetadataResponse>, reqwest::Error> {
    // 1. Try Google Books First
    if let Ok(results) = search_google_books_metadata(query).await {
        if !results.is_empty() {
            return Ok(results);
        }
    }
    
    // 2. Fallback to OpenLibrary
    search_openlibrary_metadata(query).await
}
