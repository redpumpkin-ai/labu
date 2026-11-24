//! Provider adapters: how a given LLM API turns a request into a match key
//! and a recorded response into plain text.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// One LLM API dialect. A trait is Rust's version of an interface: the
/// record and replay code only talk to `Provider`, so adding a second API
/// later means adding one more `impl`, not touching the core.
pub trait Provider {
    /// Short name stored in the cassette header, e.g. "openai-compatible".
    fn name(&self) -> &'static str;

    /// Stable id for a request (sha256 hex, 64 chars). Two requests that
    /// mean the same thing must give the same key.
    fn match_key(&self, method: &str, path: &str, body: &[u8]) -> String;

    /// The model-written text inside a response body. Used for scanning
    /// only; the cassette always keeps the original bytes.
    fn response_text(&self, content_type: &str, body: &[u8]) -> String;
}

/// Any server that speaks the OpenAI HTTP API (Bifrost, OpenAI, vLLM, ...).
pub struct OpenAiCompatible;

impl Provider for OpenAiCompatible {
    fn name(&self) -> &'static str {
        "openai-compatible"
    }

    fn match_key(&self, method: &str, path: &str, body: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(method.as_bytes());
        hasher.update(b"\n");
        hasher.update(path.as_bytes());
        hasher.update(b"\n");
        hasher.update(canonical_body(body));
        hex::encode(hasher.finalize())
    }

    fn response_text(&self, content_type: &str, body: &[u8]) -> String {
        // "text/event-stream; charset=utf-8" -> "text/event-stream"
        let media_type = content_type
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();

        let mut text = String::new();
        match media_type.as_str() {
            "text/event-stream" => {
                // Streaming: each "data: {...}" line carries a small delta.
                for line in String::from_utf8_lossy(body).lines() {
                    let Some(data) = line.strip_prefix("data:") else {
                        continue;
                    };
                    // Lines such as "data: [DONE]" are not JSON; skip them.
                    if let Ok(chunk) = serde_json::from_str::<Value>(data.trim()) {
                        push_choice_text(&chunk, "delta", &mut text);
                    }
                }
            }
            "application/json" => {
                if let Ok(json) = serde_json::from_slice::<Value>(body) {
                    push_choice_text(&json, "message", &mut text);
                }
            }
            _ => text = String::from_utf8_lossy(body).into_owned(),
        }
        text
    }
}

/// JSON bodies are re-serialized so key order and whitespace do not matter
/// (`serde_json` keeps object keys sorted unless `preserve_order` is on).
/// Anything that is not JSON is hashed as the raw bytes.
fn canonical_body(body: &[u8]) -> Vec<u8> {
    match serde_json::from_slice::<Value>(body) {
        Ok(json) => serde_json::to_vec(&json).unwrap_or_else(|_| body.to_vec()),
        Err(_) => body.to_vec(),
    }
}

/// Appends `choices[].<field>.content` and every tool call's arguments.
/// `field` is "delta" for stream chunks and "message" for full responses.
fn push_choice_text(json: &Value, field: &str, out: &mut String) {
    let Some(choices) = json["choices"].as_array() else {
        return;
    };
    for choice in choices {
        let part = &choice[field];
        if let Some(content) = part["content"].as_str() {
            out.push_str(content);
        }
        if let Some(tool_calls) = part["tool_calls"].as_array() {
            for call in tool_calls {
                if let Some(arguments) = call["function"]["arguments"].as_str() {
                    out.push_str(arguments);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const P: OpenAiCompatible = OpenAiCompatible;

    #[test]
    fn key_ignores_json_key_order_and_whitespace() {
        let a = br#"{"model":"m","messages":[{"role":"user","content":"hi"}]}"#;
        let b = br#"{ "messages": [ {"content":"hi","role":"user"} ], "model": "m" }"#;
        assert_eq!(
            P.match_key("POST", "/v1/chat/completions", a),
            P.match_key("POST", "/v1/chat/completions", b)
        );
    }

    #[test]
    fn key_changes_with_content_path_method_and_stream_flag() {
        let k = |m, p, b: &[u8]| P.match_key(m, p, b);
        let base = k("POST", "/v1/chat/completions", br#"{"x":1}"#);
        assert_ne!(base, k("POST", "/v1/chat/completions", br#"{"x":2}"#));
        assert_ne!(base, k("POST", "/v1/embeddings", br#"{"x":1}"#));
        assert_ne!(base, k("GET", "/v1/chat/completions", br#"{"x":1}"#));
        assert_ne!(
            k("POST", "/p", br#"{"stream":true}"#),
            k("POST", "/p", br#"{"stream":false}"#)
        );
    }

    #[test]
    fn key_of_non_json_body_uses_raw_bytes() {
        assert_eq!(P.match_key("POST", "/p", b"\xff\x00").len(), 64);
        assert_ne!(
            P.match_key("POST", "/p", b"\xff\x00"),
            P.match_key("POST", "/p", b"\xff\x01")
        );
        assert_eq!(P.match_key("GET", "/v1/models", b"").len(), 64);
    }

    #[test]
    fn text_from_sse_concatenates_content_and_tool_arguments() {
        let sse = b"data: {\"choices\":[{\"delta\":{\"content\":\"ha\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"content\":\"lo\"}}]}\n\ndata: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"function\":{\"arguments\":\"{\\\"a\\\":1}\"}}]}}]}\n\ndata: not json\n\ndata: [DONE]\n\n";
        assert_eq!(
            P.response_text("text/event-stream; charset=utf-8", sse),
            "halo{\"a\":1}"
        );
    }

    #[test]
    fn text_from_json_completion_and_embeddings() {
        let chat = br#"{"choices":[{"message":{"content":"halo","tool_calls":[{"function":{"arguments":"{}"}}]}}]}"#;
        assert_eq!(P.response_text("application/json", chat), "halo{}");
        let emb = br#"{"data":[{"embedding":[0.1234567890123456,0.5]}]}"#;
        assert_eq!(P.response_text("application/json", emb), "");
    }

    #[test]
    fn text_from_unknown_content_type_is_lossy_body() {
        assert_eq!(P.response_text("text/plain", b"abc"), "abc");
    }
}
