use axum::{
    body::Body,
    extract::State,
    http::{header, Response, StatusCode, Uri},
    response::IntoResponse,
};
use std::{
    env,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct FrontendPackage {
    root: PathBuf,
}

impl FrontendPackage {
    pub fn discover() -> Option<Self> {
        let configured_path = env::var_os("TM_DIST_DIR").map(PathBuf::from);
        let adjacent_path = env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|parent| parent.join("tm")));

        configured_path
            .into_iter()
            .chain(adjacent_path)
            .find_map(Self::from_path)
    }

    fn from_path(root: PathBuf) -> Option<Self> {
        root.join("index.html").is_file().then_some(Self { root })
    }

    pub fn description(&self) -> String {
        format!("TM frontend from {}", self.root.display())
    }
}

pub type SharedFrontendPackage = Arc<Option<FrontendPackage>>;

pub async fn serve(State(package): State<SharedFrontendPackage>, uri: Uri) -> Response<Body> {
    let Some(package) = package.as_ref() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let requested_path = uri.path().trim_start_matches('/');
    let asset_path = if requested_path.is_empty() {
        Path::new("index.html")
    } else {
        Path::new(requested_path)
    };

    if !is_safe_relative_path(asset_path) {
        return StatusCode::BAD_REQUEST.into_response();
    }

    if let Some(response) = read_file(&package.root, asset_path).await {
        return response;
    }

    if asset_path.extension().is_none() {
        if let Some(response) = read_file(&package.root, Path::new("index.html")).await {
            return response;
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

fn is_safe_relative_path(path: &Path) -> bool {
    path.components()
        .all(|component| matches!(component, Component::Normal(_)))
}

async fn read_file(root: &Path, path: &Path) -> Option<Response<Body>> {
    let contents = tokio::fs::read(root.join(path)).await.ok()?;
    let content_type = mime_guess::from_path(path).first_or_octet_stream();
    Some(
        Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, content_type.as_ref())
            .header(header::CACHE_CONTROL, "no-cache")
            .body(Body::from(contents))
            .unwrap(),
    )
}
