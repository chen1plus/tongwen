use axum::{
    Router,
    extract::Json,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde_json::json;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::CorsLayer;

use crate::pipeline;

pub const BASE_ID: &str = "tongwen";

fn extract_text_from_value(msg: &serde_json::Value) -> String {
    // content 可以是 string 或 array
    if let Some(content) = msg.get("content") {
        if let Some(s) = content.as_str() {
            return s.to_string();
        }
        if let Some(arr) = content.as_array() {
            let mut out = String::new();
            for part in arr {
                if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                    out.push_str(t);
                } else if let Some(t) = part.get("content").and_then(|v| v.as_str()) {
                    out.push_str(t);
                } else if let Some(s) = part.as_str() {
                    out.push_str(s);
                }
            }
            return out;
        }
    }
    String::new()
}

fn pick_input(messages: &[serde_json::Value]) -> String {
    for msg in messages.iter().rev() {
        if msg.get("role").and_then(|v| v.as_str()) == Some("user") {
            return extract_text_from_value(msg);
        }
    }
    if let Some(last) = messages.last() {
        return extract_text_from_value(last);
    }
    String::new()
}

fn api_error(message: &str, status: StatusCode, error_type: &str) -> Response {
    let body = json!({
        "error": {
            "message": message,
            "type": error_type
        }
    });
    (status, Json(body)).into_response()
}

async fn handle_health() -> &'static str {
    "ok"
}

async fn handle_models() -> impl IntoResponse {
    let models = vec![
        json!({
            "id": format!("{}-voiceink", BASE_ID),
            "object": "model",
            "created": 0,
            "owned_by": "tongwen",
        }),
        json!({
            "id": BASE_ID,
            "object": "model",
            "created": 0,
            "owned_by": "tongwen",
        }),
    ];
    (
        StatusCode::OK,
        Json(json!({
            "object": "list",
            "data": models,
        })),
    )
}

async fn handle_chat(
    payload_res: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(mut value) = match payload_res {
        Ok(v) => v,
        Err(_) => {
            return api_error(
                "Invalid JSON body",
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
            );
        }
    };

    // model 缺漏/空值 → 補默認
    let model_missing = value.get("model").is_none()
        || value["model"].is_null()
        || value["model"]
            .as_str()
            .map(|s| s.is_empty())
            .unwrap_or(false);
    if model_missing {
        if let Some(obj) = value.as_object_mut() {
            obj.insert(
                "model".to_string(),
                serde_json::Value::String(BASE_ID.to_string()),
            );
        }
    }

    let model = value
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(BASE_ID)
        .to_string();

    let messages = match value.get("messages").and_then(|v| v.as_array()) {
        Some(arr) => arr.clone(),
        None => {
            return api_error(
                "Invalid request payload: missing messages",
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
            );
        }
    };

    if messages.is_empty() {
        return api_error(
            "`messages` must be a non-empty array",
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
        );
    }

    let raw = pick_input(&messages);
    let input = if model == format!("{}-voiceink", BASE_ID) {
        pipeline::strip_transcript_tags(&raw)
    } else {
        raw.clone()
    };

    let output = pipeline::post_process(&input);

    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as u32;
    let resp = json!({
        "id": format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
        "object": "chat.completion",
        "created": created,
        "model": model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": output
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": input.chars().count() as u32,
            "completion_tokens": output.chars().count() as u32,
            "total_tokens": (input.chars().count() + output.chars().count()) as u32
        }
    });
    Json(resp).into_response()
}

async fn handle_404() -> Response {
    api_error("Not Found", StatusCode::NOT_FOUND, "not_found")
}

pub fn app() -> Router {
    Router::new()
        .route("/health", get(handle_health))
        .route("/v1/models", get(handle_models))
        .route("/v1/chat/completions", post(handle_chat))
        .fallback(handle_404)
        .layer(CorsLayer::permissive())
}

pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
