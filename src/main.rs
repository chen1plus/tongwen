use axum::{
    extract::Json,
    http::StatusCode,
    response::{sse::Event, IntoResponse, Response, Sse},
    routing::{get, post},
    Router,
};
use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};
use tower_http::cors::CorsLayer;

#[derive(serde::Serialize)]
struct ErrorDetail {
    message: String,
    #[serde(rename = "type")]
    error_type: String,
}

#[derive(serde::Serialize)]
struct ErrorResponse {
    error: ErrorDetail,
}

fn api_error(message: &str, status: StatusCode, error_type: &str) -> Response {
    (
        status,
        Json(ErrorResponse {
            error: ErrorDetail {
                message: message.to_string(),
                error_type: error_type.to_string(),
            },
        }),
    )
        .into_response()
}

#[derive(serde::Serialize)]
struct ChatCompletionMessage {
    role: &'static str,
    content: String,
}

#[derive(serde::Serialize)]
struct ChatCompletionChoice {
    index: usize,
    message: ChatCompletionMessage,
    finish_reason: &'static str,
}

#[derive(serde::Serialize)]
struct ChatCompletionUsage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(serde::Serialize)]
struct ChatCompletionResponse {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChatCompletionChoice>,
    usage: ChatCompletionUsage,
}

#[derive(serde::Serialize, Clone)]
struct ChatCompletionChunkDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    role: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
}

#[derive(serde::Serialize, Clone)]
struct ChatCompletionChunkChoice {
    index: usize,
    delta: ChatCompletionChunkDelta,
    #[serde(serialize_with = "serialize_null_or_str")]
    finish_reason: Option<&'static str>,
}

// Special serializer to output null when None, instead of omitting the field entirely
fn serialize_null_or_str<S>(value: &Option<&'static str>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    match value {
        Some(s) => serializer.serialize_str(s),
        None => serializer.serialize_none(),
    }
}

#[derive(serde::Serialize, Clone)]
struct ChatCompletionChunk {
    id: String,
    object: &'static str,
    created: u64,
    model: String,
    choices: Vec<ChatCompletionChunkChoice>,
}

async fn handle_health() -> &'static str {
    "ok"
}

async fn handle_models() -> impl IntoResponse {
    let models = vec![
        serde_json::json!({
            "id": format!("{}-voiceink", tongwen::BASE_ID),
            "object": "model",
            "created": 0,
            "owned_by": "tongwen",
        }),
        serde_json::json!({
            "id": tongwen::BASE_ID,
            "object": "model",
            "created": 0,
            "owned_by": "tongwen",
        }),
    ];

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "object": "list",
            "data": models,
        })),
    )
}

async fn handle_chat(
    payload_res: Result<Json<tongwen::ChatRequest>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(body) = match payload_res {
        Ok(body) => body,
        Err(_) => {
            return api_error("Invalid JSON body", StatusCode::BAD_REQUEST, "invalid_request_error");
        }
    };

    if body.messages.is_empty() {
        return api_error(
            "`messages` must be a non-empty array",
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
        );
    }

    let model = body.model.clone().unwrap_or_else(|| tongwen::BASE_ID.to_string());
    let voiceink_model = format!("{}-voiceink", tongwen::BASE_ID);

    let raw = tongwen::pick_input(&body.messages);
    let input = if model == voiceink_model {
        tongwen::strip_transcript_tags(&raw)
    } else {
        raw
    };

    let output = tongwen::convert_s2tw(&input);
    let is_stream = body.stream.unwrap_or(false);

    if is_stream {
        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let chunk1 = ChatCompletionChunk {
            id: id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatCompletionChunkDelta {
                    role: Some("assistant"),
                    content: None,
                },
                finish_reason: None,
            }],
        };

        let chunk_final = ChatCompletionChunk {
            id: id.clone(),
            object: "chat.completion.chunk",
            created,
            model: model.clone(),
            choices: vec![ChatCompletionChunkChoice {
                index: 0,
                delta: ChatCompletionChunkDelta {
                    role: None,
                    content: None,
                },
                finish_reason: Some("stop"),
            }],
        };

        let mut events = Vec::new();
        events.push(Event::default().json_data(&chunk1).unwrap());

        for ch in output.chars() {
            let chunk_char = ChatCompletionChunk {
                id: id.clone(),
                object: "chat.completion.chunk",
                created,
                model: model.clone(),
                choices: vec![ChatCompletionChunkChoice {
                    index: 0,
                    delta: ChatCompletionChunkDelta {
                        role: None,
                        content: Some(ch.to_string()),
                    },
                    finish_reason: None,
                }],
            };
            events.push(Event::default().json_data(&chunk_char).unwrap());
        }

        events.push(Event::default().json_data(&chunk_final).unwrap());
        events.push(Event::default().data("[DONE]"));

        let event_stream = futures_util::stream::iter(events.into_iter().map(Ok::<_, Infallible>));

        Sse::new(event_stream).into_response()
    } else {
        let response = ChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
            object: "chat.completion",
            created: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            model,
            choices: vec![ChatCompletionChoice {
                index: 0,
                message: ChatCompletionMessage {
                    role: "assistant",
                    content: output.clone(),
                },
                finish_reason: "stop",
            }],
            usage: ChatCompletionUsage {
                prompt_tokens: input.chars().count(),
                completion_tokens: output.chars().count(),
                total_tokens: input.chars().count() + output.chars().count(),
            },
        };
        Json(response).into_response()
    }
}

async fn handle_404() -> Response {
    api_error("Not Found", StatusCode::NOT_FOUND, "not_found")
}

#[tokio::main]
async fn main() {
    let host = std::env::var("TONGWEN_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("TONGWEN_PORT")
        .ok()
        .and_then(|p| p.parse::<u16>().ok())
        .unwrap_or(1180);

    let app = Router::new()
        .route("/health", get(handle_health))
        .route("/v1/models", get(handle_models))
        .route("/v1/chat/completions", post(handle_chat))
        .fallback(handle_404)
        .layer(CorsLayer::permissive());

    let addr_str = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr_str).await.unwrap();
    println!("Listening on http://{}", addr_str);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();
}

async fn shutdown_signal() {
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
