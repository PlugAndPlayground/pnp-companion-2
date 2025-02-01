use axum::{response::IntoResponse, response::Response, Json};
use glob::glob;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;

#[derive(Serialize)]
pub struct FileListResponse {
    files: Vec<String>,
    error: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct FileRequest {
    #[serde(rename = "fileName")]
    file_name: String,
}

#[derive(Serialize)]
struct FileResponse {
    content: Option<String>,
    error: Option<String>,
}

pub async fn list_files() -> Response {
    let mut response = FileListResponse {
        files: Vec::new(),
        error: None,
    };

    match glob("*") {
        Ok(paths) => {
            response.files = paths
                .filter_map(|entry| {
                    entry
                        .ok()
                        .and_then(|path| path.to_str().map(|s| s.to_string()))
                })
                .collect();
        }
        Err(e) => {
            response.error = Some(format!("Failed to read directory: {}", e));
        }
    }

    println!("Files found: {:?}", response.files);

    Json(response).into_response()
}

pub async fn get_file(file_request: Json<FileRequest>) -> Response {
    println!("Getting file");
    let mut response = FileResponse {
        content: None,
        error: None,
    };

    // Get the current working directory
    let current_dir = match env::current_dir() {
        Ok(dir) => dir,
        Err(e) => {
            response.error = Some(format!("Failed to get current directory: {}", e));
            return Json(response).into_response();
        }
    };

    // Create a path from the requested filename and canonicalize it
    let requested_path = current_dir.join(&file_request.file_name);
    let canonical_path = match requested_path.canonicalize() {
        Ok(path) => path,
        Err(e) => {
            response.error = Some(format!("Invalid file path: {}", e));
            return Json(response).into_response();
        }
    };

    // Check if the canonical path is within the current directory
    if !canonical_path.starts_with(&current_dir) {
        response.error =
            Some("Access denied: Cannot access files outside current directory".to_string());
        return Json(response).into_response();
    }

    // Check if path is actually a file (not a directory)
    if !canonical_path.is_file() {
        response.error = Some(format!("'{}' is not a file", file_request.file_name));
        return Json(response).into_response();
    }

    // Read the file
    match fs::read_to_string(canonical_path) {
        Ok(content) => {
            response.content = Some(content);
        }
        Err(e) => {
            response.error = Some(format!("Failed to read file: {}", e));
        }
    }

    Json(response).into_response()
}
