//! Terminal interface implementation

use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader};
use reqwest::Client;

use crate::config::Config;
use crate::core::llm::{
    call_reasoner_with_context, is_coding_request, stream_crafter_response,
};
use crate::models::{Message, Role};

/// Start the terminal interface
pub async fn start(
    client: Client,
    config: Config,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("🤖 DualMind Chat Interface");
    println!("Type 'exit' to quit\n");
    println!("TIP: Run with --api to start the API server instead");

    // Create a simple session for the terminal interface
    let mut session_messages: Vec<Message> = Vec::new();

    loop {
        print!("You: ");
        std::io::stdout().flush()?;

        let mut message = String::new();
        let mut stdin = BufReader::new(tokio::io::stdin());
        stdin.read_line(&mut message).await?;
        let message = message.trim();

        if message.eq_ignore_ascii_case("exit") {
            break;
        }

        // Add user message to session
        session_messages.push(Message {
            role: Role::User,
            content: message.to_string(),
        });

        // Check if this is a coding request
        let is_coding = is_coding_request(&message);

        // First call to reasoning model
        println!("\n🧠 Thinking phase ({} reasoning)...", config.reasoning_model);
        let reasoning = match call_reasoner_with_context(&client, &config.api_key, &session_messages, &config).await
        {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error in thinking phase: {}", e);
                continue;
            }
        };

        // Second call to crafting model with streaming output
        if is_coding {
            println!("\n💻 Execution phase ({} coding)...", config.craft_model);
        } else {
            println!("\n💬 Response phase ({})...", config.craft_model);
        }
        println!("\nAssistant: ");

        // Stream the crafter response directly to the user
        match stream_crafter_response(&client, &config.api_key, &session_messages, &reasoning, &config).await {
            Ok(final_response) => {
                println!(); // Add a newline after the streamed response

                // Add assistant response to session
                session_messages.push(Message {
                    role: Role::Assistant,
                    content: final_response,
                });
            }
            Err(e) => {
                eprintln!("Error in execution phase: {}", e);
            }
        }
    }

    Ok(())
} 