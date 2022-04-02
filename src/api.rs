use std::collections::HashMap;
use std::time::Instant;

use format_sql_query::{QuotedData, Table};
use log::{debug, error, info};
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LinkItemResponse {
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
    resolved_title: Option<String>,
    resolved_url: Option<String>,
    excerpt: Option<String>,
    is_article: Option<String>,
    is_index: Option<String>,
    has_video: Option<String>,
    has_image: Option<String>,
    word_count: Option<String>,
    lang: Option<String>,
    listen_duration_estimate: u16,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct LinkListResponse {
    status: u8,
    list: HashMap<String, LinkItemResponse>,
}

struct DbStorageReport {
    total: usize,
    inserted: u32,
    time: u128,
}

impl Default for DbStorageReport {
    fn default() -> DbStorageReport {
        DbStorageReport { total: 0, inserted: 0, time: 0 }
    }
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
        .send()
        .await?;

    let resp: RequestCodeResponse = response.json().await?;
    config.code = Option::from(resp.code);

    Ok(())
}

pub(crate) fn authorize_app(config: &mut Config) {
    let auth_url: String = format!(
        "https://getpocket.com/auth/authorize?request_token={}&redirect_uri={}",
        config.code.as_ref().unwrap(),
        config.redirect_url
    );
    let mut buffer = String::new();

    config.auth_url = Option::from(auth_url.clone());
    info!(
        "Open the following URL in your browser and click on authorize: {}",
        auth_url
    );

    match std::io::stdin().read_line(&mut buffer) {
        Ok(_) => println!("{}", buffer),
        Err(e) => println!("{}", e),
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
        .send()
        .await?;

    let resp: RequestTokenResponse = response.json().await?;
    config.token = Option::from(resp.access_token);

    Ok(())
}

#[tokio::main]
pub(crate) async fn code_is_valid(config: &mut Config) {
    match &config.auth_url {
        Some(url) => match Client::new().get(url).send().await {
            Ok(resp) => {
                let status: StatusCode = resp.status();
                config.code_valid = Option::from(status.is_success());
            }
            Err(e) => error!("Error validating auth URL: {}", e),
        },
        None => {
            config.code_valid = Option::from(false);
            error!("Auth URL is not specified")
        }
    }
}

#[tokio::main]
pub(crate) async fn obtain_links(config: &Config) -> Result<(), reqwest::Error> {
    let payload = json!({
        "consumer_key": config.consumer_key,
        "access_token": config.token
    });

    let request_url = format!("{}/get", config.api_endpoint);
    let response = Client::new()
        .post(request_url)
        .header("X-Accept", "application/json")
        .json(&payload)
        .send()
        .await?;

    let links: LinkListResponse = response.json().await?;

    // parse_links(links.list);
    store_db(&links.list);

    Ok(())
}

pub(crate) fn store_db(links: &HashMap<String, LinkItemResponse>) {
    let mut db_storage_report = DbStorageReport { total: links.len(), ..Default::default() };

    let now = Instant::now();

    let connection = sqlite::open("links.db").unwrap();

    connection
        .execute(
            "CREATE TABLE IF NOT EXISTS links (id INTEGER NOT NULL PRIMARY KEY, resolved_title TEXT, item_id INTEGER, time_added TEXT, url TEXT);",
        )
        .unwrap();

    for (_, link) in links.iter() {
        let mut resolved_title = String::new();
        match &link.resolved_title {
            None => error!("Could not resolve title."),
            Some(title) => {
                resolved_title = title.to_string();
            }
        }

        match connection.execute(format!(
            "INSERT INTO {} (resolved_title, item_id, time_added, url) VALUES ({}, {}, {}, {});",
            Table("links".into()), QuotedData(&resolved_title), QuotedData(&link.item_id), QuotedData(&link.time_added), QuotedData(&link.given_url)
        )) {
            Err(e) => {
                error!("{}", e);
                error!("{}", resolved_title);
            }
            Ok(..) => {
                db_storage_report.inserted = db_storage_report.inserted + 1;
                info!("Alright!")
            }
        }
    }

    db_storage_report.time = now.elapsed().as_millis() / 100;

    debug!("Total links: {}", db_storage_report.total);
    debug!("Total inserted: {}", db_storage_report.inserted);
    debug!("Total time: {}", db_storage_report.time);
}
