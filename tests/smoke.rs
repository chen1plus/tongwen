use futures_util::StreamExt;
use tokio::net::TcpListener;
use tongwen::app;

/// Helper to get the base URL to test against.
/// If TONGWEN_URL is set in environment, returns that.
/// Otherwise, spawns a new server instance on a random local port.
async fn get_base_url() -> (String, Option<tokio::task::JoinHandle<()>>) {
    if let Ok(url) = std::env::var("TONGWEN_URL") {
        (url, None)
    } else {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("Failed to bind random port");
        let addr = listener.local_addr().expect("Failed to get local address");
        let router = app();

        let handle = tokio::spawn(async move {
            axum::serve(listener, router)
                .await
                .expect("Server failed to run");
        });

        (format!("http://{}", addr), Some(handle))
    }
}

async fn run_step<F, Fut>(name: &str, f: F)
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<(), String>>,
{
    match f().await {
        Ok(_) => println!("ok  {}", name),
        Err(e) => {
            eprintln!("FAIL {}: {}", name, e);
            panic!("Test step failed: {}", name);
        }
    }
}

#[tokio::test]
async fn smoke_test() {
    let (base_url, _server_handle) = get_base_url().await;
    let client = reqwest::Client::new();

    // 1. GET /health
    run_step("GET /health", || async {
        let res = client
            .get(format!("{}/health", base_url))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("Expected 200 OK, got: {}", res.status()));
        }

        let body = res
            .text()
            .await
            .map_err(|e| format!("Failed to read body: {}", e))?;

        if body != "ok" {
            return Err(format!("Expected body 'ok', got: {:?}", body));
        }

        Ok(())
    })
    .await;

    // 2. GET /v1/models
    run_step("GET /v1/models", || async {
        let res = client
            .get(format!("{}/v1/models", base_url))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("Expected 200 OK, got: {}", res.status()));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON body: {}", e))?;

        let data = body
            .get("data")
            .and_then(|d| d.as_array())
            .ok_or_else(|| "Missing or invalid 'data' array in models response".to_string())?;

        if data.is_empty() {
            return Err("Expected models 'data' list to be non-empty".to_string());
        }

        Ok(())
    })
    .await;

    // 3. POST /v1/chat/completions (non-stream)
    run_step("POST /v1/chat/completions (non-stream)", || async {
        let payload = serde_json::json!({
            "messages": [{"role": "user", "content": "汉字转换：软件、电脑、网络"}]
        });

        let res = client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("Expected 200 OK, got: {}", res.status()));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON body: {}", e))?;

        let out = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.get(0))
            .and_then(|first| first.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| {
                "Failed to extract content from choices[0].message.content".to_string()
            })?;

        if !out.contains("漢字") {
            return Err(format!(
                "Expected content to include '漢字', got: {:?}",
                out
            ));
        }

        println!("     → {}", out);
        Ok(())
    })
    .await;

    // 4. POST /v1/chat/completions (voiceink strips <TRANSCRIPT>)
    run_step("POST /v1/chat/completions (voiceink strips <TRANSCRIPT>)", || async {
        let payload = serde_json::json!({
            "model": "tongwen-s2tw-voiceink",
            "messages": [{"role": "user", "content": "<TRANSCRIPT>语音转录的简体内容</TRANSCRIPT>"}]
        });

        let res = client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("Expected 200 OK, got: {}", res.status()));
        }

        let body: serde_json::Value = res
            .json()
            .await
            .map_err(|e| format!("Failed to parse JSON body: {}", e))?;

        let out = body
            .get("choices")
            .and_then(|c| c.as_array())
            .and_then(|a| a.get(0))
            .and_then(|first| first.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .ok_or_else(|| "Failed to extract content from choices[0].message.content".to_string())?;

        if out.contains('<') || out.contains('>') {
            return Err(format!("Expected tags stripped, got: {:?}", out));
        }

        if out != "語音轉錄的簡體內容" {
            return Err(format!("Unexpected output, got: {:?}", out));
        }

        println!("     → {}", out);
        Ok(())
    })
    .await;

    // 5. POST /v1/chat/completions (default model keeps tags)
    run_step(
        "POST /v1/chat/completions (default model keeps tags)",
        || async {
            let payload = serde_json::json!({
                "messages": [{"role": "user", "content": "<TRANSCRIPT>简体</TRANSCRIPT>"}]
            });

            let res = client
                .post(format!("{}/v1/chat/completions", base_url))
                .json(&payload)
                .send()
                .await
                .map_err(|e| format!("Request failed: {}", e))?;

            if res.status() != reqwest::StatusCode::OK {
                return Err(format!("Expected 200 OK, got: {}", res.status()));
            }

            let body: serde_json::Value = res
                .json()
                .await
                .map_err(|e| format!("Failed to parse JSON body: {}", e))?;

            let out = body
                .get("choices")
                .and_then(|c| c.as_array())
                .and_then(|a| a.get(0))
                .and_then(|first| first.get("message"))
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_str())
                .ok_or_else(|| {
                    "Failed to extract content from choices[0].message.content".to_string()
                })?;

            if !out.contains("<TRANSCRIPT>") || !out.contains("</TRANSCRIPT>") {
                return Err(format!("Tags should be preserved, got: {:?}", out));
            }

            Ok(())
        },
    )
    .await;

    // 6. POST /v1/chat/completions (stream)
    run_step("POST /v1/chat/completions (stream)", || async {
        let payload = serde_json::json!({
            "stream": true,
            "messages": [{"role": "user", "content": "简体变繁体"}]
        });

        let res = client
            .post(format!("{}/v1/chat/completions", base_url))
            .json(&payload)
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?;

        if res.status() != reqwest::StatusCode::OK {
            return Err(format!("Expected 200 OK, got: {}", res.status()));
        }

        let mut stream = res.bytes_stream();
        let mut buf = String::new();
        let mut acc = String::new();
        let mut saw_done = false;

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| format!("Stream read error: {}", e))?;
            let text = String::from_utf8(chunk.to_vec())
                .map_err(|e| format!("Invalid UTF-8 chunk: {}", e))?;
            buf.push_str(&text);

            while let Some(pos) = buf.find('\n') {
                let line = buf[..pos].to_string();
                buf = buf[pos + 1..].to_string();

                let line = line.trim();
                if line.is_empty() {
                    continue;
                }

                if line.starts_with("data: ") {
                    let payload = line["data: ".len()..].trim();
                    if payload == "[DONE]" {
                        saw_done = true;
                        continue;
                    }

                    if let Ok(j) = serde_json::from_str::<serde_json::Value>(payload) {
                        if let Some(delta) = j
                            .get("choices")
                            .and_then(|c| c.as_array())
                            .and_then(|a| a.get(0))
                            .and_then(|first| first.get("delta"))
                            .and_then(|d| d.get("content"))
                            .and_then(|c| c.as_str())
                        {
                            acc.push_str(delta);
                        }
                    }
                }
            }
        }

        // Process leftover buffer in case there is no trailing newline (unlikely but safe)
        let remaining = buf.trim();
        if !remaining.is_empty() && remaining.starts_with("data: ") {
            let payload = remaining["data: ".len()..].trim();
            if payload == "[DONE]" {
                saw_done = true;
            } else if let Ok(j) = serde_json::from_str::<serde_json::Value>(payload) {
                if let Some(delta) = j
                    .get("choices")
                    .and_then(|c| c.as_array())
                    .and_then(|a| a.get(0))
                    .and_then(|first| first.get("delta"))
                    .and_then(|d| d.get("content"))
                    .and_then(|c| c.as_str())
                {
                    acc.push_str(delta);
                }
            }
        }

        if !saw_done {
            return Err("Stream missing [DONE] indicator".to_string());
        }

        if !acc.contains("簡體") {
            return Err(format!("Expected output to contain '簡體', got: {:?}", acc));
        }

        println!("     → {}", acc);
        Ok(())
    })
    .await;

    println!("\nall good ✓");
}
