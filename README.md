# Tong Wen (同文)

A high-performance, lightweight, OpenAI-compatible Chinese translation proxy server written in Rust. 

It exposes a standard OpenAI Chat Completions endpoint, allowing you to drop it into any LLM client or application (e.g., translation workflows, dictation tools) as a drop-in replacement, but performs instant and accurate Simplified-to-Traditional Chinese (Taiwan variant) conversion using `zhconv`.

## Features

- **OpenAI API Compatibility**: Implements standard `/v1/chat/completions` and `/v1/models` endpoints.
- **Streaming & Non-Streaming Support**: Works seamlessly with both standard JSON payloads and Server-Sent Events (SSE) streaming (`stream: true`).
- **Two Pre-configured Models**:
  - `tongwen-s2tw`: Standard Simplified-to-Traditional (Taiwan) translation.
  - `tongwen-s2tw-voiceink`: Specially optimized for Voiceink transcripts; automatically strips `<TRANSCRIPT>` and `</TRANSCRIPT>` tags before translating.
- **Ultralight & Lightning Fast**: Built on top of **Axum** and **Tokio** for asynchronous, robust performance under load.
- **Easy Configuration**: Simple environment variable control and permissive CORS out of the box.

## Getting Started

### Prerequisites
Make sure you have [Rust and Cargo](https://rustup.rs/) installed.

### Build and Run
Start the server with the following command:

```bash
cargo run --release
```

By default, the server listens on `http://127.0.0.1:1180`.

### Environment Configuration
You can customize the host and port using environment variables:

```bash
TONGWEN_HOST="0.0.0.0" TONGWEN_PORT="8080" cargo run --release
```

---

## API Usage Examples

### 1. List Available Models
```bash
curl http://localhost:1180/v1/models
```

### 2. Chat Completion (Non-Streaming)
```bash
curl -X POST http://localhost:1180/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "tongwen-s2tw",
    "messages": [
      {
        "role": "user",
        "content": "汉字转换：软件、电脑、网络"
      }
    ]
  }'
```

**Response:**
```json
{
  "id": "chatcmpl-...",
  "object": "chat.completion",
  "created": 1716723456,
  "model": "tongwen-s2tw",
  "choices": [
    {
      "index": 0,
      "message": {
        "role": "assistant",
        "content": "漢字轉換：軟體、電腦、網路"
      },
      "finish_reason": "stop"
    }
  ],
  "usage": {
    "prompt_tokens": 13,
    "completion_tokens": 13,
    "total_tokens": 26
  }
}
```

### 3. Chat Completion (Streaming)
```bash
curl -X POST http://localhost:1180/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "tongwen-s2tw",
    "stream": true,
    "messages": [
      {
        "role": "user",
        "content": "简体变繁体"
      }
    ]
  }'
```
