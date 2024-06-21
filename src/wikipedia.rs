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

pub fn get_pretty_extract(extract: String) -> String {
    let regex = Regex::new(r"\s+").unwrap();
    regex.replace_all(&extract, " ").to_string()
}