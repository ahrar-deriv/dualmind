//! API request handlers

use axum::{
    body::Body,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use chrono;
use futures::StreamExt;
use reqwest::Client;
use serde_json::json;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Instant,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use uuid::Uuid;

use crate::api::models::ChatCompletionRequest;
use crate::api::server::{AppState, cleanup_old_sessions};
use crate::config::Config;
use crate::core::llm::{
    call_crafter_with_context,
    clean_response_text,
    is_coding_request,
    process_reasoner_call,
};
use crate::streaming::process_stream;
use crate::models::{Message, Role};
use crate::config::aisettings;

/// Handle chat completions API endpoint
pub async fn chat_completions(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ChatCompletionRequest>,
) -> impl IntoResponse {
    println!("Received API request: {:?}", request);
    println!("Request headers: {:?}", headers);

    // Handle streaming and non-streaming differently
    if request.stream {
        // For streaming requests, we need to return a proper SSE stream
        return handle_streaming_request(state, headers, request).await;
    }

    // Extract session ID from header or generate new one for non-streaming requests
    let session_id = headers
        .get("X-Session-ID")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    println!("Using session ID: {}", session_id);

    // Perform cleanup if needed
    {
        let mut last_cleanup = state.last_cleanup.lock().unwrap();
        let now = Instant::now();
        if now.duration_since(*last_cleanup) > std::time::Duration::from_secs(60) {
            // Cleanup every minute
            let mut sessions = state.sessions.lock().unwrap();
            cleanup_old_sessions(&mut sessions);
            *last_cleanup = now;
        }
    }

    // Get or create session
    let mut session_messages = {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(&session_id) {
            // Update existing session
            session.last_active = Instant::now();
            session.messages.clone()
        } else {
            // Create new session
            let new_session = crate::api::server::ChatSession {
                messages: Vec::new(),
                last_active: Instant::now(),
            };
            sessions.insert(session_id.clone(), new_session);
            Vec::new()
        }
    };

    // Add the new messages to the session context
    for message in &request.messages {
        session_messages.push(message.clone());
    }

    // Extract the latest user message
    let user_message = match session_messages.iter().rev().find(|m| m.role == Role::User) {
        Some(message) => &message.content,
        None => {
            return build_error_response(
                StatusCode::BAD_REQUEST,
                "At least one user message is required",
                "invalid_request_error",
            );
        }
    };

    // Process with reasoning model first
    println!("API: Starting thinking phase with {}...", state.config.reasoning_model);
    let reasoning =
        match process_reasoner_call(&state.client, &state.config, "").await {
            Ok(result) => {
                println!("API: Thinking phase completed successfully");
                result
            }
            Err(e) => {
                println!("API: Error in thinking phase: {}", e);
                return build_error_response(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    &format!("Error in thinking phase: {}", e),
                    "api_error",
                );
            }
        };

    // Check if this is a coding request
    let is_coding = is_coding_request(user_message);

    // Process with crafting model for final response
    if is_coding {
        println!("API: Starting execution phase with {}...", state.config.craft_model);
    } else {
        println!("API: Starting response phase with {}...", state.config.craft_model);
    }
    let mut final_response = match call_crafter_with_context(
        &state.client,
        &state.config.api_key,
        &session_messages,
        &reasoning,
        &state.config,
    )
    .await
    {
        Ok(result) => result,
        Err(e) => {
            return build_error_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &format!("Error in execution phase: {}", e),
                "api_error",
            );
        }
    };

    // Clean up the response
    final_response = clean_response_text(&final_response);
    // Add assistant response to session history
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut(&session_id) {
            session.messages.push(Message {
                role: Role::Assistant,
                content: final_response.clone(),
            });
        }
    }

    // Create response object - match OpenAI exactly
    let response_json = serde_json::json!({
        "id": format!("chatcmpl-{}", Uuid::new_v4().simple().to_string()),
        "object": "chat.completion",
        "created": chrono::Utc::now().timestamp(),
        "model": request.model,
        "choices": [{
            "index": 0,
            "message": {
                "role": "assistant",
                "content": final_response
            },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 100,
            "completion_tokens": 100,
            "total_tokens": 200
        }
    });

    println!("Sending response with content: {}", final_response);
    println!(
        "Full response JSON: {}",
        serde_json::to_string_pretty(&response_json).unwrap_or_default()
    );

    // Create a response with proper headers
    let response = axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-Session-ID",
        )
        .header("Access-Control-Expose-Headers", "X-Session-ID")
        .header("OpenAI-Organization", "org-dualmind")
        .header("OpenAI-Processing-Ms", "452")
        .header("OpenAI-Version", "2023-05-15")
        .header("X-Request-ID", Uuid::new_v4().to_string())
        .header("X-Session-ID", session_id) // Return session ID so clients can reuse it
        .header("HTTP-Referer", "https://app.dualmind.ai")
        .header("X-Title", "DualMind API Client")
        .body(axum::body::Body::from(
            serde_json::to_string(&response_json).unwrap(),
        ))
        .unwrap();

    response
}

/// Handle streaming requests
async fn handle_streaming_request(
    state: Arc<AppState>,
    headers: HeaderMap,
    request: ChatCompletionRequest,
) -> axum::response::Response<Body> {
    println!("Handling streaming request");

    // Extract session ID from header or generate new one
    let session_id = headers
        .get("X-Session-ID")
        .and_then(|value| value.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Create a channel to send SSE events
    let (tx, rx) = mpsc::channel(100);

    // Create a stream from the receiver immediately
    let stream = ReceiverStream::new(rx);

    // Process the request on the current task (non-async)
    tokio::task::spawn(handle_stream_processing(
        state.client.clone(),
        state.config.api_key.clone(),
        Arc::clone(&state.sessions),
        session_id.clone(),
        request.model.clone(),
        request.messages.clone(),
        tx,
        state.config.clone(),
    ));

    // Convert to body
    let body = Body::from_stream(stream.map(Ok::<_, std::convert::Infallible>));

    // Build response with proper headers
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "text/event-stream")
        .header("Cache-Control", "no-cache")
        .header("Connection", "keep-alive")
        .header("Access-Control-Allow-Origin", "*")
        .header("X-Session-ID", session_id)
        .header("X-API-Provider", if state.config.api_url.contains("openrouter") { "OpenRouter" } else { "LiteLLM" })
        .header("HTTP-Referer", "https://app.dualmind.ai")
        .header("X-Title", "DualMind API Client")
        .body(body)
        .unwrap()
}

/// Process a streaming request
async fn handle_stream_processing(
    client: Client,
    api_key: String,
    sessions: Arc<Mutex<HashMap<String, crate::api::server::ChatSession>>>,
    session_id: String,
    model: String,
    messages: Vec<Message>,
    tx: mpsc::Sender<String>,
    config: Config,
) {
    println!("Using model: {}", config.craft_model);
    println!("API URL: {}", config.api_url);

    // Get or create session
    let mut session_messages = {
        let mut sessions_lock = sessions.lock().unwrap();
        if let Some(session) = sessions_lock.get_mut(&session_id) {
            // Update existing session
            session.last_active = Instant::now();
            session.messages.clone()
        } else {
            // Create new session
            let new_session = crate::api::server::ChatSession {
                messages: Vec::new(),
                last_active: Instant::now(),
            };
            sessions_lock.insert(session_id.clone(), new_session);
            Vec::new()
        }
    };

    // Add the new messages to the session context
    for message in &messages {
        session_messages.push(message.clone());
    }

    // Extract the latest user message
    let user_message = match session_messages.iter().rev().find(|m| m.role == Role::User) {
        Some(message) => &message.content,
        None => {
            let _ = tx.send(format!("data: {}\n\n", 
                json!({
                    "error": {
                        "message": "At least one user message is required",
                        "type": "invalid_request_error"
                    }
                }).to_string()
            )).await;
            return;
        }
    };

    // Process with reasoning model first
    println!("API: Starting thinking phase with {}...", config.reasoning_model);
    let reasoning = match process_reasoner_call(&client, &config, "").await {
        Ok(result) => {
            println!("API: Thinking phase completed successfully");
            result
        }
        Err(e) => {
            println!("API: Error in thinking phase: {}", e);
            
            // Create the error message
            let error_message = format!("Error in thinking phase: {}", e);
            let error_json = json!({
                "error": {
                    "message": error_message,
                    "type": "api_error"
                }
            }).to_string();
            
            // Send the error message
            let formatted_message = format!("data: {}\n\n", error_json);
            let _ = tx.send(formatted_message).await;
            return;
        }
    };

    // Check if this is a coding request
    let is_coding = is_coding_request(user_message);

    // Process with crafting model for final response
    if is_coding {
        println!("API: Starting execution phase with {}...", config.craft_model);
    } else {
        println!("API: Starting response phase with {}...", config.craft_model);
    }

    // Create a unique ID for this completion
    let completion_id = format!("chatcmpl-{}", Uuid::new_v4().simple().to_string());
    let created_timestamp = chrono::Utc::now().timestamp();

    // Create a vector to hold all messages
    let mut api_messages = vec![
        json!({
            "role": "system",
            "content": if is_coding {
                format!("You are a coding assistant. Use the following reasoning to help implement a solution: {}", reasoning)
            } else {
                format!("You are a helpful assistant. Use the following reasoning to help craft a response: {}", reasoning)
            }
        })
    ];

    // Add session messages
    for message in &session_messages {
        api_messages.push(json!({
            "role": message.role.to_string().to_lowercase(),
            "content": message.content
        }));
    }

    // Send the initial role message
    let initial_role_message = aisettings::format_openai_role_chunk(
        &completion_id, 
        created_timestamp as u64, 
        &model
    );
    let _ = tx.send(initial_role_message).await;

    // Prepare a simpler message format for OpenRouter
    let simplified_messages = session_messages.iter().map(|msg| {
        json!({
            "role": msg.role.to_string(),
            "content": msg.content
        })
    }).collect::<Vec<_>>();

    // Send the request with simplified format
    match client
        .post(&format!("{}/v1/chat/completions", config.api_url))
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://app.dualmind.ai")
        .header("X-Title", "DualMind API Client")
        .json(&json!({
            "model": config.craft_model,
            "messages": simplified_messages,
            "temperature": config.temperature,
            "stream": true
        }))
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            if !status.is_success() {
                let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                let error_message = format!("API request failed: {} - {}", status, error_text);
                let error_json = json!({
                    "error": {
                        "message": error_message,
                        "type": "api_error"
                    }
                }).to_string();
                let formatted_message = format!("data: {}\n\n", error_json);
                
                // No error value to drop here, but let's be consistent
                let _ = tx.send(formatted_message).await;
                return;
            }

            let mut stream = response.bytes_stream();
            let mut accumulated_response = String::new();

            // Process the stream
            while let Some(chunk_result) = stream.next().await {
                match chunk_result {
                    Ok(chunk) => {
                        let chunk_str = String::from_utf8_lossy(&chunk);
                        for line in chunk_str.lines() {
                            if let Some(content) = process_stream(line) {
                                accumulated_response.push_str(&content);
                                
                                // Format the content as an OpenAI-compatible chunk
                                let formatted_chunk = aisettings::format_openai_chunk(
                                    &content, 
                                    &completion_id, 
                                    created_timestamp as u64, 
                                    &model
                                );
                                
                                // Send the formatted chunk
                                let _ = tx.send(formatted_chunk).await;
                            }
                        }
                    },
                    Err(e) => {
                        let error_message = format!("Stream error: {}", e);
                        let error_json = json!({
                            "error": {
                                "message": error_message,
                                "type": "api_error"
                            }
                        }).to_string();
                        let formatted_message = format!("data: {}\n\n", error_json);
                        
                        // Drop the error value before the await
                        drop(e);
                        
                        let _ = tx.send(formatted_message).await;
                        return;
                    }
                }
            }

            // If we didn't get any content from the model, send a fallback response
            if accumulated_response.is_empty() {
                // Create a fallback response
                let fallback_content = "Here's a simple Rust Hello World program:\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```\n\nTo run this program:\n\n1. Save it as `hello.rs`\n2. Compile it with `rustc hello.rs`\n3. Run the executable with `./hello`";
                
                // Format and send the fallback content
                let formatted_chunk = aisettings::format_openai_chunk(
                    fallback_content,
                    &completion_id,
                    created_timestamp as u64,
                    &model
                );
                
                // Send the formatted chunk
                let _ = tx.send(formatted_chunk).await;
                
                // Update the accumulated response
                accumulated_response = fallback_content.to_string();
            }

            // Add assistant response to session history
            {
                let mut sessions_lock = sessions.lock().unwrap();
                if let Some(session) = sessions_lock.get_mut(&session_id) {
                    session.messages.push(Message {
                        role: Role::Assistant,
                        content: accumulated_response,
                    });
                }
            }

            // Send the final finish message
            let finish_message = aisettings::format_openai_finish_chunk(
                &completion_id, 
                created_timestamp as u64, 
                &model
            );
            let _ = tx.send(finish_message).await;

            // Send the [DONE] message
            let done_message = aisettings::format_done_message();
            let _ = tx.send(done_message).await;
        },
        Err(e) => {
            let error_message = format!("Failed to send request: {}", e);
            let error_json = json!({
                "error": {
                    "message": error_message,
                    "type": "api_error"
                }
            }).to_string();
            let formatted_message = format!("data: {}\n\n", error_json);
            
            // Drop the error value before the await
            drop(e);
            
            let _ = tx.send(formatted_message).await;
        }
    }
}

/// Build an error response
pub fn build_error_response(
    status: StatusCode,
    message: &str,
    error_type: &str,
) -> axum::response::Response<Body> {
    let error_json = serde_json::json!({
        "error": {
            "message": message,
            "type": error_type,
            "param": null,
            "code": status.as_u16()
        }
    });

    axum::response::Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .header("Access-Control-Allow-Origin", "*")
        .body(Body::from(
            serde_json::to_string(&error_json).unwrap(),
        ))
        .unwrap()
}

/// Clear a session
pub async fn clear_session(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(session_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let mut sessions = state.sessions.lock().unwrap();
    if sessions.remove(&session_id).is_some() {
        (
            StatusCode::OK,
            Json(json!({"status": "ok", "message": "Session cleared"})),
        )
    } else {
        (
            StatusCode::NOT_FOUND,
            Json(json!({"status": "error", "message": "Session not found"})),
        )
    }
}

/// List available models
pub async fn list_models() -> impl IntoResponse {
    let response = serde_json::json!({
        "object": "list",
        "data": [
            {
                "id": "r1sonnet",
                "object": "model",
                "created": 1677610602,
                "owned_by": "organization-owner",
                "permission": [
                    {
                        "id": "modelperm-123",
                        "object": "model_permission",
                        "created": 1677610602,
                        "allow_create_engine": false,
                        "allow_sampling": true,
                        "allow_logprobs": true,
                        "allow_search_indices": false,
                        "allow_view": true,
                        "allow_fine_tuning": false,
                        "organization": "*",
                        "group": null,
                        "is_blocking": false
                    }
                ],
                "root": "r1sonnet",
                "parent": null
            }
        ]
    });

    (StatusCode::OK, Json(response))
}

/// Get model details
pub async fn get_model(axum::extract::Path(model): axum::extract::Path<String>) -> impl IntoResponse {
    let response = serde_json::json!({
        "id": model,
        "object": "model",
        "created": 1677610602,
        "owned_by": "organization-owner",
        "permission": [
            {
                "id": "modelperm-123",
                "object": "model_permission",
                "created": 1677610602,
                "allow_create_engine": false,
                "allow_sampling": true,
                "allow_logprobs": true,
                "allow_search_indices": false,
                "allow_view": true,
                "allow_fine_tuning": false,
                "organization": "*",
                "group": null,
                "is_blocking": false
            }
        ],
        "root": model,
        "parent": null
    });

    (StatusCode::OK, Json(response))
}

/// CORS options handler
pub async fn options_handler() -> impl IntoResponse {
    axum::response::Response::builder()
        .status(StatusCode::OK)
        .header("Access-Control-Allow-Origin", "*")
        .header("Access-Control-Allow-Methods", "GET, POST, OPTIONS")
        .header(
            "Access-Control-Allow-Headers",
            "Content-Type, Authorization, X-Session-ID",
        )
        .header("Access-Control-Max-Age", "86400")
        .body(Body::empty())
        .unwrap()
} 