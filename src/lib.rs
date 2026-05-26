use async_openai::types::chat::{
    ChatCompletionRequestAssistantMessageContent, ChatCompletionRequestMessage,
    ChatCompletionRequestSystemMessageContent, ChatCompletionRequestUserMessageContent,
    ChatCompletionRequestUserMessageContentPart,
};
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
