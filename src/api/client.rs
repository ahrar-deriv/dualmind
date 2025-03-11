//! API client implementation

/// Test the API client
pub async fn test() -> Result<(), Box<dyn std::error::Error>> {
    let client = reqwest::Client::new();

    let response = client
        .post("http://localhost:3000/v1/chat/completions")
        .header("Content-Type", "application/json")
        .json(&serde_json::json!({
            "model": "dualmind",
            "messages": [{"role": "user", "content": "Hello world"}],
            "stream": false
        }))
        .send()
        .await?;

    println!("Status: {}", response.status());
    let body = response.text().await?;
    println!("Response body: {}", body);

    Ok(())
} 