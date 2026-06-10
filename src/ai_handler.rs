use axum::{
    body::Body,
    http::{header, Response, StatusCode},
    response::IntoResponse,
    Json,
};
use futures_util::StreamExt;
use reqwest::Client;
use serde_json::{json, Value};
use std::{
    env, io,
    sync::atomic::{AtomicU64, Ordering},
    time::Instant,
};
use tracing::{error, info, warn};

const ANTHROPIC_URL: &str = "https://api.anthropic.com/v1/messages";
const DEEPSEEK_URL: &str = "https://api.deepseek.com/anthropic/v1/messages";
static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Default)]
struct TokenUsage {
    input: u64,
    output: u64,
    cache_creation: u64,
    cache_read: u64,
    reported_total: u64,
}

impl TokenUsage {
    fn total(self) -> u64 {
        if self.reported_total > 0 {
            self.reported_total
        } else {
            self.input + self.output + self.cache_creation + self.cache_read
        }
    }

    fn update_anthropic(&mut self, value: &Value) {
        let usage = value.get("usage").or_else(|| {
            value
                .get("message")
                .and_then(|message| message.get("usage"))
        });
        let Some(usage) = usage else {
            return;
        };

        self.input = self.input.max(json_u64(usage, "input_tokens"));
        self.output = self.output.max(json_u64(usage, "output_tokens"));
        self.cache_creation = self
            .cache_creation
            .max(json_u64(usage, "cache_creation_input_tokens"));
        self.cache_read = self
            .cache_read
            .max(json_u64(usage, "cache_read_input_tokens"));
    }
}

fn next_request_id() -> u64 {
    NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed)
}

fn model(payload: &Value) -> &str {
    payload
        .get("model")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
}

fn json_u64(value: &Value, key: &str) -> u64 {
    value.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn anthropic_usage(body: &Value) -> TokenUsage {
    let mut usage = TokenUsage::default();
    usage.update_anthropic(body);
    usage
}

fn gemini_usage(body: &Value) -> TokenUsage {
    let metadata = body.get("usageMetadata").unwrap_or(&Value::Null);
    TokenUsage {
        input: json_u64(metadata, "promptTokenCount"),
        output: json_u64(metadata, "candidatesTokenCount"),
        cache_read: json_u64(metadata, "cachedContentTokenCount"),
        cache_creation: 0,
        reported_total: json_u64(metadata, "totalTokenCount"),
    }
}

fn log_completed(
    request_id: u64,
    provider: &str,
    model: &str,
    status: StatusCode,
    started: Instant,
    usage: TokenUsage,
) {
    info!(
        request_id,
        provider,
        model,
        status = status.as_u16(),
        duration_ms = started.elapsed().as_millis() as u64,
        input_tokens = usage.input,
        output_tokens = usage.output,
        cache_creation_tokens = usage.cache_creation,
        cache_read_tokens = usage.cache_read,
        total_tokens = usage.total(),
        "AI response received"
    );
}

fn api_key(name: &str, request_id: u64, provider: &str) -> Result<String, Response<Body>> {
    env::var(name)
        .ok()
        .filter(|key| !key.trim().is_empty())
        .ok_or_else(|| {
            warn!(
                request_id,
                provider,
                environment_variable = name,
                "AI request rejected because provider credentials are missing"
            );
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({
                    "error": format!("{name} is not configured in the companion .env file")
                })),
            )
                .into_response()
        })
}

async fn anthropic_request(
    url: &str,
    key_name: &str,
    payload: Value,
    request_id: u64,
    provider: &str,
) -> Result<(StatusCode, Value), Response<Body>> {
    let key = api_key(key_name, request_id, provider)?;
    let response = Client::new()
        .post(url)
        .header("Content-Type", "application/json")
        .header("Anthropic-Version", "2023-06-01")
        .header("x-api-key", key)
        .json(&payload)
        .send()
        .await
        .map_err(|error| upstream_error(error, request_id, provider))?;
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .unwrap_or_else(|error| json!({ "error": error.to_string() }));
    Ok((status, body))
}

fn upstream_error(error: reqwest::Error, request_id: u64, provider: &str) -> Response<Body> {
    error!(
        request_id,
        provider,
        error = %error,
        "AI provider request failed"
    );
    (
        StatusCode::BAD_GATEWAY,
        Json(json!({
            "error": "AI provider request failed",
            "details": error.to_string()
        })),
    )
        .into_response()
}

pub async fn claude(Json(payload): Json<Value>) -> Response<Body> {
    let request_id = next_request_id();
    let started = Instant::now();
    let model = model(&payload).to_owned();
    info!(
        request_id,
        provider = "anthropic",
        model,
        "AI request received"
    );

    match anthropic_request(
        ANTHROPIC_URL,
        "ANTHROPIC_API_KEY",
        payload,
        request_id,
        "anthropic",
    )
    .await
    {
        Ok((status, body)) if status.is_success() => {
            log_completed(
                request_id,
                "anthropic",
                &model,
                status,
                started,
                anthropic_usage(&body),
            );
            (
                status,
                Json(json!({
                    "success": true,
                    "data": body
                })),
            )
                .into_response()
        }
        Ok((status, body)) => {
            warn!(
                request_id,
                provider = "anthropic",
                model,
                status = status.as_u16(),
                duration_ms = started.elapsed().as_millis() as u64,
                "AI provider returned an error response"
            );
            (status, Json(body)).into_response()
        }
        Err(response) => response,
    }
}

pub async fn deepseek(Json(payload): Json<Value>) -> Response<Body> {
    let request_id = next_request_id();
    let started = Instant::now();
    let model = model(&payload).to_owned();
    info!(
        request_id,
        provider = "deepseek",
        model,
        "AI request received"
    );

    match anthropic_request(
        DEEPSEEK_URL,
        "DEEPSEEK_KEY",
        payload,
        request_id,
        "deepseek",
    )
    .await
    {
        Ok((status, body)) => {
            if status.is_success() {
                log_completed(
                    request_id,
                    "deepseek",
                    &model,
                    status,
                    started,
                    anthropic_usage(&body),
                );
            } else {
                warn!(
                    request_id,
                    provider = "deepseek",
                    model,
                    status = status.as_u16(),
                    duration_ms = started.elapsed().as_millis() as u64,
                    "AI provider returned an error response"
                );
            }
            (status, Json(body)).into_response()
        }
        Err(response) => response,
    }
}

pub async fn claude_stream(Json(mut payload): Json<Value>) -> Response<Body> {
    let request_id = next_request_id();
    let started = Instant::now();
    let model = model(&payload).to_owned();
    info!(
        request_id,
        provider = "anthropic",
        model,
        streaming = true,
        "AI request received"
    );

    let key = match api_key("ANTHROPIC_API_KEY", request_id, "anthropic") {
        Ok(key) => key,
        Err(response) => return response,
    };
    payload["stream"] = Value::Bool(true);

    let response = match Client::new()
        .post(ANTHROPIC_URL)
        .header("Content-Type", "application/json")
        .header("Anthropic-Version", "2023-06-01")
        .header("x-api-key", key)
        .json(&payload)
        .send()
        .await
    {
        Ok(response) => response,
        Err(error) => return upstream_error(error, request_id, "anthropic"),
    };

    let status = response.status();
    if !status.is_success() {
        let body = response
            .json::<Value>()
            .await
            .unwrap_or_else(|error| json!({ "error": error.to_string() }));
        warn!(
            request_id,
            provider = "anthropic",
            model,
            status = status.as_u16(),
            duration_ms = started.elapsed().as_millis() as u64,
            "AI provider returned an error response"
        );
        return (status, Json(body)).into_response();
    }

    let stream = futures_util::stream::unfold(
        (
            response.bytes_stream(),
            String::new(),
            TokenUsage::default(),
            false,
        ),
        move |(mut stream, mut buffer, mut usage, mut failed)| {
            let model = model.clone();
            async move {
                match stream.next().await {
                    Some(Ok(bytes)) => {
                        buffer.push_str(&String::from_utf8_lossy(&bytes));
                        collect_stream_usage(&mut buffer, &mut usage);
                        Some((Ok(bytes), (stream, buffer, usage, failed)))
                    }
                    Some(Err(stream_error)) => {
                        failed = true;
                        error!(
                            request_id,
                            provider = "anthropic",
                            model,
                            error = %stream_error,
                            duration_ms = started.elapsed().as_millis() as u64,
                            "AI response stream failed"
                        );
                        Some((
                            Err(io::Error::other(stream_error)),
                            (stream, buffer, usage, failed),
                        ))
                    }
                    None => {
                        collect_stream_usage(&mut buffer, &mut usage);
                        if !failed {
                            log_completed(request_id, "anthropic", &model, status, started, usage);
                        }
                        None
                    }
                }
            }
        },
    );
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/event-stream")
        .header(header::CACHE_CONTROL, "no-cache, no-transform")
        .body(Body::from_stream(stream))
        .unwrap()
}

pub async fn gemini(Json(payload): Json<Value>) -> Response<Body> {
    let request_id = next_request_id();
    let started = Instant::now();
    let model = model(&payload).to_owned();
    info!(
        request_id,
        provider = "gemini",
        model,
        "AI request received"
    );

    let key = match api_key("GEMINI_API_KEY", request_id, "gemini") {
        Ok(key) => key,
        Err(response) => return response,
    };
    let model = match payload.get("model").and_then(Value::as_str) {
        Some(model) => model.to_owned(),
        None => {
            warn!(
                request_id,
                provider = "gemini",
                "AI request rejected because model is missing"
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Model is required" })),
            )
                .into_response();
        }
    };
    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent?key={key}"
    );
    let request_body = json!({
        "contents": payload.get("contents").cloned().unwrap_or(Value::Array(vec![]))
    });
    let response = match Client::new().post(url).json(&request_body).send().await {
        Ok(response) => response,
        Err(error) => return upstream_error(error, request_id, "gemini"),
    };
    let status = response.status();
    let body = response
        .json::<Value>()
        .await
        .unwrap_or_else(|error| json!({ "error": error.to_string() }));
    if status.is_success() {
        log_completed(
            request_id,
            "gemini",
            &model,
            status,
            started,
            gemini_usage(&body),
        );
        (
            status,
            Json(json!({
                "success": true,
                "data": body
            })),
        )
            .into_response()
    } else {
        warn!(
            request_id,
            provider = "gemini",
            model,
            status = status.as_u16(),
            duration_ms = started.elapsed().as_millis() as u64,
            "AI provider returned an error response"
        );
        (status, Json(body)).into_response()
    }
}

fn collect_stream_usage(buffer: &mut String, usage: &mut TokenUsage) {
    while let Some(event_end) = buffer.find("\n\n") {
        let event = buffer[..event_end].to_owned();
        buffer.drain(..event_end + 2);

        for data in event
            .lines()
            .filter_map(|line| line.strip_prefix("data:"))
            .map(str::trim)
        {
            if data.is_empty() || data == "[DONE]" {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(data) {
                usage.update_anthropic(&value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_anthropic_token_usage() {
        let usage = anthropic_usage(&json!({
            "usage": {
                "input_tokens": 100,
                "output_tokens": 25,
                "cache_creation_input_tokens": 10,
                "cache_read_input_tokens": 50
            }
        }));

        assert_eq!(usage.input, 100);
        assert_eq!(usage.output, 25);
        assert_eq!(usage.cache_creation, 10);
        assert_eq!(usage.cache_read, 50);
        assert_eq!(usage.total(), 185);
    }

    #[test]
    fn uses_gemini_reported_total() {
        let usage = gemini_usage(&json!({
            "usageMetadata": {
                "promptTokenCount": 100,
                "candidatesTokenCount": 25,
                "cachedContentTokenCount": 40,
                "totalTokenCount": 125
            }
        }));

        assert_eq!(usage.input, 100);
        assert_eq!(usage.output, 25);
        assert_eq!(usage.cache_read, 40);
        assert_eq!(usage.total(), 125);
    }

    #[test]
    fn extracts_usage_from_split_anthropic_stream_events() {
        let mut buffer = String::new();
        let mut usage = TokenUsage::default();

        buffer.push_str(
            "event: message_start\ndata: {\"message\":{\"usage\":{\"input_tokens\":80}}}\n\n",
        );
        buffer.push_str("event: message_delta\ndata: {\"usage\":{\"output_tokens\":2");
        collect_stream_usage(&mut buffer, &mut usage);
        assert_eq!(usage.input, 80);
        assert_eq!(usage.output, 0);

        buffer.push_str("0}}\n\n");
        collect_stream_usage(&mut buffer, &mut usage);
        assert_eq!(usage.output, 20);
        assert_eq!(usage.total(), 100);
    }
}
