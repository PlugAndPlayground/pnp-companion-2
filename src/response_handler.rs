use axum::{response::IntoResponse, response::Response, Json};
use base64::{engine::general_purpose, Engine as _};
use regex::Regex;
use reqwest::{RequestBuilder, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use xml2json_rs::JsonBuilder;

fn get_env_var(key: &str) -> String {
    match env::var(key) {
        Ok(value) => value,
        Err(e) => {
            println!("Couldn't read environment variable {}: {}", key, e);
            String::from("UNKNOWN_ENV_VARIABLE")
        }
    }
}

fn replace_variables(input: String) -> String {
    // first we replace environmental variables
    //println!("Input str: {}", input);
    let env_variable_regex: Regex = Regex::new(r"\$\{(.+?)\}").unwrap();
    let mut out_string = env_variable_regex
        .replace_all(input.as_str(), |caps: &regex::Captures| {
            let key = &caps[1];
            get_env_var(key).to_string()
        })
        .to_string();

    // then we perform all specific functions based on their names

    //println!("After variable replacements: {}", out_string);
    let available_functions = [("BASE64_ENCODE", |in_string: &str| -> String {
        general_purpose::STANDARD_NO_PAD.encode(in_string)
    })];
    for (name, func) in available_functions {
        let function_reg_string = format!("\\${}\\{{(.+)\\}}", name);
        let function_regex: Regex = Regex::new(&function_reg_string).unwrap();

        out_string = function_regex
            .replace_all(&out_string.as_str(), |caps: &regex::Captures| {
                let key = &caps[1];
                //println!("key to convert: {}", key);
                func(key)
            })
            .to_string();
    }

    //println!("{}", out_string);
    out_string
}

fn convert_to_json_string(text: String) -> String {
    let json_builder = JsonBuilder::default();
    let jsond_xml = json_builder.build_string_from_xml(text.as_str());
    match jsond_xml {
        Ok(a) => {
            // this is a little bit hacky but I don't know why the lib returns an OK that looks like "null"
            if a.len() > 0 && a != "null" {
                return a;
            } else {
                return text;
            }
        }
        Err(_) => text,
    }
}

pub async fn pnp_request(Json(payload): Json<CompanionInput>) -> Response {
    let received = payload.to_owned();
    let client = reqwest::Client::new();

    let attach_headers = |mut to_send: RequestBuilder| -> RequestBuilder {
        for (key, value) in received.final_headers {
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
            Some(
                to_send
                    .body(replace_variables(received.final_body))
                    .send()
                    .await,
            )
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
                    Ok(text) => convert_to_json_string(text),
                    Err(e) => String::from(e.to_string()),
                }
            }
            Err(e) => String::from(e.to_string()),
        },
        None => String::from("{\"Response\":\"Invalid method thinks companion\"}"),
    };
    //println!("res: {}", return_string);

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
