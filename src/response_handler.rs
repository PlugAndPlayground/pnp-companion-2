use axum::{response::IntoResponse, response::Response, Json};
use base64::{engine::general_purpose, Engine as _};
use dotenv::dotenv;
use regex::Regex;
use reqwest::{RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use xml2json_rs::JsonBuilder;

fn get_env_var(key: &str) -> String {
    // First try to get from .env file (loaded into env vars by dotenv)
    match env::var(key) {
        Ok(value) => value,
        Err(e) => {
            println!("Couldn't read environment variable {}: {}", key, e);
            String::from("UNKNOWN_ENV_VARIABLE")
        }
    }
}

// Initialize environment at startup
pub fn init_environment() {
    // Create .env file if it doesn't exist
    let env_path = Path::new(".env");
    if !env_path.exists() {
        match fs::File::create(env_path) {
            Ok(_) => println!("Created .env file"),
            Err(e) => println!("Failed to create .env file: {}", e),
        }
    }
    
    // Load .env file
    match dotenv() {
        Ok(_) => println!("Loaded .env file"),
        Err(e) => println!("Error loading .env file: {}", e),
    }
}

fn replace_variables(input: String) -> String {
    // first we replace environmental variables using $TM_KEY{VAR_NAME} syntax
    let env_variable_regex: Regex = Regex::new(r"\$TM_KEY\{(.+?)\}").unwrap();
    let mut out_string = env_variable_regex
        .replace_all(input.as_str(), |caps: &regex::Captures| {
            let key = &caps[1];
            get_env_var(key).to_string()
        })
        .to_string();

    // then we perform all specific functions based on their names
    let available_functions = [("BASE64_ENCODE", |in_string: &str| -> String {
        general_purpose::STANDARD_NO_PAD.encode(in_string)
    })];
    for (name, func) in available_functions {
        let function_reg_string = format!("\\${}\\{{(.+)\\}}", name);
        let function_regex: Regex = Regex::new(&function_reg_string).unwrap();

        out_string = function_regex
            .replace_all(&out_string.as_str(), |caps: &regex::Captures| {
                let key = &caps[1];
                func(key)
            })
            .to_string();
    }

    out_string
}

fn convert_to_json_string(text: String, headers: &HashMap<String, String>) -> String {
    // Check Content-Type header
    if let Some(content_type) = headers.get("Content-Type") {
        if !content_type.to_lowercase().contains("xml") {
            return text;
        }
    }

    let json_builder = JsonBuilder::default();
    match json_builder.build_string_from_xml(text.as_str()) {
        Ok(json) if json.len() > 0 && json != "null" => json,
        _ => text,
    }
}

pub async fn ping() -> Response {
    let response = CompanionResponse {
        status: StatusCode::OK.as_u16(),
        response: String::from("{\"status\":\"ok\"}"),
    };
    Json(response).into_response()
}

pub async fn pnp_request(Json(payload): Json<CompanionInput>) -> Response {
    let received = payload.to_owned();
    let client = reqwest::Client::new();

    let attach_headers = |mut to_send: RequestBuilder| -> RequestBuilder {
        for (key, value) in received.final_headers.clone() {
            to_send = to_send.header(key, replace_variables(value));
        }
        return to_send;
    }; 

    let response = match payload.final_method.as_str() {
        "Get" => {
            let mut to_send = client.get(received.final_url);
            to_send = attach_headers(to_send);
            Some(to_send.send().await)
        }
        "Post" => {
            let mut to_send: RequestBuilder = client.post(received.final_url);
            to_send = attach_headers(to_send);
            Some(to_send.body(received.final_body).send().await)
        }
        _ => None,
    };

    let mut status_code = StatusCode::IM_A_TEAPOT;
    let return_string = match response {
        Some(res) => match res {
            Ok(res) => {
                status_code = res.status();
                let payload = res.text().await;
                match payload {
                    Ok(text) => convert_to_json_string(text, &received.final_headers),
                    Err(e) => String::from(e.to_string()),
                }
            }
            Err(e) => String::from(e.to_string()),
        },
        None => String::from("{\"Response\":\"Invalid method thinks companion\"}"),
    };

    let response = CompanionResponse {
        status: status_code.as_u16(),
        response: return_string,
    };
    Json(response).into_response()
}

#[derive(Deserialize, Clone)]
pub struct CompanionInput {
    #[serde(rename = "finalURL")]
    final_url: String,
    #[serde(rename = "finalHeaders")]
    final_headers: HashMap<String, String>,
    #[serde(rename = "finalBody")]
    final_body: String,
    #[serde(rename = "finalMethod")]
    final_method: String,
}

#[derive(Serialize)]
pub struct CompanionResponse {
    status: u16,
    response: String,
}
