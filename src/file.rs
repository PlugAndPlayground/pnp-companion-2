#[derive(Serialize)]
struct FileListResponse {
    files: Vec<String>,
    error: Option<String>,
}

fn list_dir_contents(dir: &Path, base: &Path, files: &mut Vec<String>) {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            // Get path relative to base directory
            if let Ok(relative) = path.strip_prefix(base) {
                if let Some(path_str) = relative.to_str() {
                    files.push(path_str.to_string());
                }
                // Recursively list contents if it's a directory
                if path.is_dir() {
                    list_dir_contents(&path, base, files);
                }
            }
        }
    }
}

pub async fn list_files() -> Response {
    let mut response = FileListResponse {
        files: Vec::new(),
        error: None,
    };

    // Get the executable directory
    let exe_dir = match std::env::current_exe().and_then(|p| p.parent().map(|p| p.to_path_buf())) {
        Some(dir) => dir,
        None => {
            response.error = Some("Could not get executable directory".to_string());
            return Json(response).into_response();
        }
    };

    list_dir_contents(&exe_dir, &exe_dir, &mut response.files);

    Json(response).into_response()
}