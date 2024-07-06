use std::collections::HashMap;

use regex::Regex;
use reqwest::Error;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
struct ApiResponse {
    query: Query
}

#[derive(Deserialize, Debug)]
struct Query {
    random: Option<Vec<Random>>,
    pages: Option<HashMap<String, Page>>
}

#[derive(Deserialize, Debug)]
struct Page {
    pageid: u32,
    ns: u32,
    title: String,
    extract: String,
}

#[derive(Deserialize, Debug)]
struct Random {
    id: u32,
    ns: u32,
    title: String
}

pub struct WikipediaResponse {
    pub title: String,
    pub value: String
}

pub async fn get_random_article_extract() -> Result<WikipediaResponse, Error> {
    let random_page_response = reqwest::get("https://en.wikipedia.org/w/api.php?action=query&format=json&list=random&rnnamespace=0&rnlimit=1")
        .await?
        .json::<ApiResponse>()
        .await?;

    let page_name = random_page_response.query.random.unwrap()[0].title.clone();

    let url = "https://en.wikipedia.org/w/api.php?action=query&format=json&prop=extracts&titles=".to_string() + &page_name + "&explaintext=true";

    let extract_response = reqwest::get(url)
        .await?
        .json::<ApiResponse>()
        .await?;

    let extract = extract_response.query.pages.unwrap().values().next().ok_or("No pages found").unwrap().extract.clone();

    Ok(WikipediaResponse {title: page_name, value: extract})
}

pub fn get_pretty_extract(mut extract: String) -> Option<String> {
    let mut equal_sign_count = 0;
    let mut end_headline = 0;

    let mut i = extract.chars().count();
    for char in extract.clone().chars().rev() {
        if !char.is_ascii() {
            return None
        }
        
        i -= 1;

        if char == '=' {
            equal_sign_count += 1;
        } else if char == '\n' {
            if equal_sign_count >= 2 && end_headline != 0 {
                let start = extract
                    .char_indices()
                    .nth(i + 1)
                    .map(|(pos, _)| pos)
                    .unwrap();

                let end = extract
                    .char_indices()
                    .nth(end_headline)
                    .map(|(pos, char)| pos + char.len_utf8())
                    .unwrap();

                extract.replace_range(start..end, "");
            }

            equal_sign_count = 0;
            end_headline = 0;
        } else if equal_sign_count >= 2 {
            end_headline = i + equal_sign_count;
            equal_sign_count = 0;
        }
    }

    let regex = Regex::new(r"\s+").unwrap();
    Some(regex.replace_all(&extract, " ").to_string())
}