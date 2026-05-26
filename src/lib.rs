use serde::Deserialize;
use zhconv::{zhconv, Variant};

pub const BASE_ID: &str = "tongwen-s2tw";

#[derive(Debug, Deserialize, Clone)]
pub struct ChatMessage {
    pub role: String,
    pub content: serde_json::Value,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub model: Option<String>,
    pub stream: Option<bool>,
}

/// Extracts text from message content.
/// Handles both standard strings and OpenAI multi-part content arrays.
pub fn extract_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => {
            let mut result = String::new();
            for part in arr {
                match part {
                    serde_json::Value::String(s) => {
                        result.push_str(s);
                    }
                    serde_json::Value::Object(obj) => {
                        if let Some(serde_json::Value::String(text)) = obj.get("text") {
                            result.push_str(text);
                        }
                    }
                    _ => {}
                }
            }
            result
        }
        _ => String::new(),
    }
}

/// Traverses the messages from last to first to find the first "user" message.
/// If no "user" message exists, falls back to the very last message.
pub fn pick_input(messages: &[ChatMessage]) -> String {
    for msg in messages.iter().rev() {
        if msg.role == "user" {
            return extract_text(&msg.content);
        }
    }
    if let Some(last) = messages.last() {
        extract_text(&last.content)
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
