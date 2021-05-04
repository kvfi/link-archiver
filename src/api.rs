use std::collections::HashMap;

use log::{error, info};
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;

use crate::Config;

#[derive(Deserialize)]
struct RequestCodeResponse {
    code: String,
}

#[derive(Deserialize)]
struct RequestTokenResponse {
    access_token: String,
}

#[derive(Deserialize, Debug)]
struct LinkItemResponse {
    item_id: String,
    resolved_id: String,
    given_url: String,
    given_title: String,
    favorite: String,
    status: String,
    time_added: String,
    time_updated: String,
    time_read: String,
    time_favorited: String,
    sort_id: u16,
    resolved_title: String,
    resolved_url: String,
    excerpt: String,
    is_article: String,
    is_index: String,
    has_video: String,
    has_image: String,
    word_count: String,
    lang: String,
    listen_duration_estimate: u16,
}

#[derive(Deserialize, Debug)]
pub struct LinkListResponse {
    status: u8,
    list: HashMap<String, LinkItemResponse>,
}

#[tokio::main]
pub(crate) async fn obtain_request_code(config: &mut Config) -> Result<(), reqwest::Error> {
    let payload = json!({
        "consumer_key": config.consumer_key,
        "redirect_uri": config.redirect_url
    });

    let request_url: &str = &format!("{}/oauth/request", config.api_endpoint);
    let response = Client::new()
        .post(request_url)
        .header("X-Accept", "application/json")
        .json(&payload)
        .send().await?;

    let resp: RequestCodeResponse = response.json().await?;
    config.code = Option::from(resp.code);

    Ok(())
}

pub(crate) fn authorize_app(config: &mut Config) {
    let auth_url: String = format!("https://getpocket.com/auth/authorize?request_token={}&redirect_uri={}", config.code.as_ref().unwrap(), config.redirect_url);
    let mut buffer = String::new();

    config.auth_url = Option::from(auth_url.clone());
    info!("Open the following URL in your browser and click on authorize: {}", auth_url);

    match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => println!("{}", buffer),
        Err(e) => println!("{}", e)
    }

    drop(auth_url);
}

#[tokio::main]
pub(crate) async fn obtain_request_token(config: &mut Config) -> Result<(), reqwest::Error> {
    let payload = json!({
        "consumer_key": config.consumer_key,
        "code": config.code
    });

    let request_url = format!("{}/oauth/authorize", config.api_endpoint);
    let response = Client::new()
        .post(request_url)
        .header("X-Accept", "application/json")
        .json(&payload)
        .send().await?;

    let resp: RequestTokenResponse = response.json().await?;
    config.token = Option::from(resp.access_token);

    Ok(())
}

#[tokio::main]
pub(crate) async fn code_is_valid(config: &mut Config) {
    match &config.auth_url {
        Some(url) =>
            match Client::new()
                .get(url)
                .send().await {
                Ok(resp) => {
                    let status: StatusCode = resp.status();
                    config.code_valid = Option::from(status.is_success());
                }
                Err(e) => error!("Error validating auth URL: {}", e)
            }
        None => {
            config.code_valid = Option::from(false);
            error!("Auth URL is not specified")
        }
    }
}

fn parse_links(map: &mut HashMap<String, LinkItemResponse>) {
    for (key, value) in &*map {
        info!("{} / {:?}", key, value.);
    }
    map.clear();
}

#[tokio::main]
pub(crate) async fn obtain_links(config: Config) -> Result<(), reqwest::Error> {
    let payload = json!({
        "consumer_key": config.consumer_key,
        "access_token": config.token,
        "count": 100
    });

    let request_url = format!("{}/get", config.api_endpoint);
    let response = Client::new()
        .post(request_url)
        .header("X-Accept", "application/json")
        .json(&payload)
        .send().await?;

    let mut links: LinkListResponse = response.json().await?;

    parse_links(&mut links.list);

    Ok(())
}
