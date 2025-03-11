//! AI provider-specific settings and formatting

use serde_json::json;

/// Format a streaming chunk for OpenAI-compatible clients
pub fn format_openai_chunk(content: &str, completion_id: &str, created_timestamp: u64, model: &str) -> String {
    let chunk_json = json!({
        "id": completion_id,
        "object": "chat.completion.chunk",
        "created": created_timestamp,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {
                "content": content
            },
            "finish_reason": null
        }]
    });
    
    format!("data: {}\n\n", chunk_json.to_string())
}

/// Format the initial role message for OpenAI-compatible clients
pub fn format_openai_role_chunk(completion_id: &str, created_timestamp: u64, model: &str) -> String {
    let chunk_json = json!({
        "id": completion_id,
        "object": "chat.completion.chunk",
        "created": created_timestamp,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {
                "role": "assistant"
            },
            "finish_reason": null
        }]
    });
    
    format!("data: {}\n\n", chunk_json.to_string())
}

/// Format the final message with finish_reason for OpenAI-compatible clients
pub fn format_openai_finish_chunk(completion_id: &str, created_timestamp: u64, model: &str) -> String {
    let chunk_json = json!({
        "id": completion_id,
        "object": "chat.completion.chunk",
        "created": created_timestamp,
        "model": model,
        "choices": [{
            "index": 0,
            "delta": {},
            "finish_reason": "stop"
        }]
    });
    
    format!("data: {}\n\n", chunk_json.to_string())
}

/// Format the [DONE] message
pub fn format_done_message() -> String {
    "data: [DONE]\n\n".to_string()
}

/// Determine if the API provider is OpenRouter
pub fn is_openrouter(api_url: &str) -> bool {
    api_url.contains("openrouter")
}

/// Determine if the API provider is LiteLLM
pub fn is_litellm(api_url: &str) -> bool {
    api_url.contains("litellm")
}

pub fn get_system_prompt() -> String {
    "You are DualMind, an AI assistant powered by a reasoning model and a crafting model...".to_string()
} 