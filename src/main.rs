use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::Path;

use error_chain::error_chain;
use log::{error, info};
use serde::{Deserialize, Serialize};
use simple_logger::SimpleLogger;

use crate::api::code_is_valid;

mod api;

static CONFIG_FILE_PATH: &str = "./config.json";

#[derive(Serialize, Deserialize, Default)]
struct Config {
    consumer_key: String,
    redirect_url: String,
    api_endpoint: String,
    code: Option<String>,
    token: Option<String>,
    auth_url: Option<String>,
    code_valid: Option<bool>,
}

error_chain! {
    foreign_links {
        Io(std::io::Error);
        HttpRequest(reqwest::Error);
    }
}

fn main() -> Result<()> {
    SimpleLogger::new().init().unwrap();

    let config_filepath = Path::new(CONFIG_FILE_PATH);

    if config_filepath.exists() {
        let mut config_content = fs::read_to_string(config_filepath)
            .expect("Something went wrong reading the file");
        let mut config: Config = serde_json::from_str(&*config_content).unwrap();

        code_is_valid(&mut config);

        if config.code.is_none() || !config.code_valid.unwrap() {
            match api::obtain_request_code(&mut config) {
                Err(err) => error!("Cannot obtain code: {:?}", err),
                Ok(()) => match config.code {
                    Some(_) => {
                        api::authorize_app(&mut config);
                        match api::obtain_request_token(&mut config) {
                            Ok(_) => println!("{}", "nice"),
                            Err(e) => error!("{}", e)
                        }
                        config_content = serde_json::to_string_pretty(&config).unwrap();
                        let mut file = File::create(config_filepath)?;
                        match file.write_all(&config_content.as_bytes()) {
                            Ok(_) => {
                                info!("Updated config written to disk")
                            }
                            Err(e) => {
                                error!("Cannot write to file: {:?}", e);
                            }
                        }
                    }
                    None => error!("{}", "oh nooo")
                }
            }
        } else {
            info!("{}", "Code is ok");
            match api::obtain_links(config) {
                Ok(_) => info!("Links stored successfully."),
                Err(e) => error!("{}", e)
            }
        }
    } else {
        std::process::exit(1);
    }
    Ok(())
}