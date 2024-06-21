use reqwest::{Client, Error};
use reqwest::header::{CONTENT_TYPE, HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};

const API_KEY: &str = "397ada08-5792-4093-a85f-3f8da73f4c7c:fx";

#[derive(Serialize, Deserialize, Debug)]
struct ApiResponse {
    translations: Vec<Translations>
}

#[derive(Serialize, Deserialize, Debug)]
struct Translations {
    detected_source_language: String,
    text: String
}

pub async fn translate_word_list(word_list: Vec<String>) -> Result<Vec<String>, Error> {
    let client = Client::new();

    let mut headers = HeaderMap::new();
    headers.insert("Authorization", HeaderValue::from_str(&format!("DeepL-Auth-Key {}", API_KEY)).unwrap());
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let word_string = word_list.join(";");
    let body = serde_json::json!({
        "text": [word_string],
        "target_lang": "EN"
    });

    let response = client.post("https://api-free.deepl.com/v2/translate")
        .headers(headers)
        .json(&body)
        .send()
        .await?;

    if response.status().is_success() {
        let response_body: ApiResponse = response.json().await?;
        let mut translated_word_list = Vec::new();

        for translation in response_body.translations {
            for word in translation.text.split(";") {
                translated_word_list.push(word.to_string());
            };
        }

        return Ok(translated_word_list);
    } else {
        println!("Request failed with status: {}, Error: {}", response.status(), response.text().await?);
    }

    Ok(vec![])
}