use async_openai::types::chat::{
    ChatChoice, ChatChoiceStream, ChatCompletionRequestAssistantMessageContent,
    ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageContent,
    ChatCompletionRequestUserMessageContent, ChatCompletionRequestUserMessageContentPart,
    ChatCompletionResponseMessage, ChatCompletionStreamResponseDelta, CompletionUsage,
    CreateChatCompletionRequest, CreateChatCompletionResponse, CreateChatCompletionStreamResponse,
    FinishReason, Role,
};
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
use zhconv::{zhconv, Variant};

pub const BASE_ID: &str = "tongwen-s2tw";

/// Extracts text from message content.
/// Handles both standard strings and OpenAI multi-part content arrays.
pub fn extract_text(msg: &ChatCompletionRequestMessage) -> String {
    match msg {
        ChatCompletionRequestMessage::User(user_msg) => match &user_msg.content {
            ChatCompletionRequestUserMessageContent::Text(text) => text.clone(),
            ChatCompletionRequestUserMessageContent::Array(parts) => {
                let mut result = String::new();
                for part in parts {
                    match part {
                        ChatCompletionRequestUserMessageContentPart::Text(text_content) => {
                            result.push_str(&text_content.text);
                        }
                        _ => {}
                    }
                }
                result
            }
        },
        ChatCompletionRequestMessage::Assistant(assistant_msg) => match &assistant_msg.content {
            Some(ChatCompletionRequestAssistantMessageContent::Text(text)) => text.clone(),
            _ => String::new(),
        },
        ChatCompletionRequestMessage::System(system_msg) => match &system_msg.content {
            ChatCompletionRequestSystemMessageContent::Text(text) => text.clone(),
            _ => String::new(),
        },
        _ => String::new(),
    }
}

/// Traverses the messages from last to first to find the first "user" message.
/// If no "user" message exists, falls back to the very last message.
pub fn pick_input(messages: &[ChatCompletionRequestMessage]) -> String {
    for msg in messages.iter().rev() {
        if matches!(msg, ChatCompletionRequestMessage::User(_)) {
            return extract_text(msg);
        }
    }
    if let Some(last) = messages.last() {
        extract_text(last)
    } else {
        String::new()
    }
}

/// Voiceink adapter preprocessor. Strips transcript tags and trims.
pub fn strip_transcript_tags(s: &str) -> String {
    s.replace("<TRANSCRIPT>", "")
        .replace("</TRANSCRIPT>", "")
        .trim()
        .to_string()
}

/// Converts text from Simplified Chinese (or any variant) to Traditional Taiwan Chinese.
pub fn convert_s2tw(input: &str) -> String {
    zhconv(input, Variant::ZhTW)
}

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

async fn handle_health() -> &'static str {
    "ok"
}

async fn handle_models() -> impl IntoResponse {
    let models = vec![
        serde_json::json!({
            "id": format!("{}-voiceink", BASE_ID),
            "object": "model",
            "created": 0,
            "owned_by": "tongwen",
        }),
        serde_json::json!({
            "id": BASE_ID,
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
    payload_res: Result<Json<serde_json::Value>, axum::extract::rejection::JsonRejection>,
) -> Response {
    let Json(mut value) = match payload_res {
        Ok(body) => body,
        Err(_) => {
            return api_error(
                "Invalid JSON body",
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
            );
        }
    };

    if value.get("model").is_none()
        || value["model"].is_null()
        || value["model"].as_str().map_or(true, |s| s.is_empty())
    {
        if let serde_json::Value::Object(ref mut map) = value {
            map.insert(
                "model".to_string(),
                serde_json::Value::String(BASE_ID.to_string()),
            );
        }
    }

    let body: CreateChatCompletionRequest = match serde_json::from_value(value) {
        Ok(body) => body,
        Err(err) => {
            return api_error(
                &format!("Invalid request payload: {}", err),
                StatusCode::BAD_REQUEST,
                "invalid_request_error",
            );
        }
    };

    if body.messages.is_empty() {
        return api_error(
            "`messages` must be a non-empty array",
            StatusCode::BAD_REQUEST,
            "invalid_request_error",
        );
    }

    let model = body.model.clone();
    let voiceink_model = format!("{}-voiceink", BASE_ID);

    let raw = pick_input(&body.messages);
    let input = if model == voiceink_model {
        strip_transcript_tags(&raw)
    } else {
        raw
    };

    let output = convert_s2tw(&input);
    let is_stream = body.stream.unwrap_or(false);

    if is_stream {
        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4().simple());
        let created = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as u32;

        let chunk1 = CreateChatCompletionStreamResponse {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            system_fingerprint: None,
            choices: vec![ChatChoiceStream {
                index: 0,
                delta: ChatCompletionStreamResponseDelta {
                    role: Some(Role::Assistant),
                    content: None,
                    function_call: None,
                    tool_calls: None,
                    refusal: None,
                },
                logprobs: None,
                finish_reason: None,
            }],
            usage: None,
            service_tier: None,
        };

        let chunk_final = CreateChatCompletionStreamResponse {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created,
            model: model.clone(),
            system_fingerprint: None,
            choices: vec![ChatChoiceStream {
                index: 0,
                delta: ChatCompletionStreamResponseDelta {
                    role: None,
                    content: None,
                    function_call: None,
                    tool_calls: None,
                    refusal: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: None,
            service_tier: None,
        };

        let mut events = Vec::new();
        events.push(Event::default().json_data(&chunk1).unwrap());

        for ch in output.chars() {
            let chunk_char = CreateChatCompletionStreamResponse {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created,
                model: model.clone(),
                system_fingerprint: None,
                choices: vec![ChatChoiceStream {
                    index: 0,
                    delta: ChatCompletionStreamResponseDelta {
                        role: None,
                        content: Some(ch.to_string()),
                        function_call: None,
                        tool_calls: None,
                        refusal: None,
                    },
                    logprobs: None,
                    finish_reason: None,
                }],
                usage: None,
                service_tier: None,
            };
            events.push(Event::default().json_data(&chunk_char).unwrap());
        }

        events.push(Event::default().json_data(&chunk_final).unwrap());
        events.push(Event::default().data("[DONE]"));

        let event_stream = futures_util::stream::iter(events.into_iter().map(Ok::<_, Infallible>));

        Sse::new(event_stream).into_response()
    } else {
        let response = CreateChatCompletionResponse {
            id: format!("chatcmpl-{}", uuid::Uuid::new_v4().simple()),
            object: "chat.completion".to_string(),
            created: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as u32,
            model,
            system_fingerprint: None,
            choices: vec![ChatChoice {
                index: 0,
                message: ChatCompletionResponseMessage {
                    role: Role::Assistant,
                    content: Some(output.clone()),
                    tool_calls: None,
                    function_call: None,
                    refusal: None,
                    audio: None,
                    annotations: None,
                },
                logprobs: None,
                finish_reason: Some(FinishReason::Stop),
            }],
            usage: Some(CompletionUsage {
                prompt_tokens: input.chars().count() as u32,
                completion_tokens: output.chars().count() as u32,
                total_tokens: (input.chars().count() + output.chars().count()) as u32,
                completion_tokens_details: None,
                prompt_tokens_details: None,
            }),
            service_tier: None,
        };
        Json(response).into_response()
    }
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
