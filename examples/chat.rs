//! Example: basic chat request with the Quantum AI SDK.
//!
//! Usage:
//!
//!   export QAI_API_KEY=your-api-key
//!   cargo run --example chat

use futures_util::StreamExt;
use quantum_sdk::{ChatMessage, ChatRequest, Client};

#[tokio::main]
async fn main() {
    let api_key =
        std::env::var("QAI_API_KEY").expect("QAI_API_KEY environment variable is required");

    let client = Client::new(api_key);

    // --- Non-streaming example ---
    println!("=== Non-streaming Chat ===");

    let resp = client
        .chat(&ChatRequest {
            model: "claude-opus-4-8".into(),
            messages: vec![ChatMessage::user(
                "What is quantum computing in one sentence?",
            )],
            ..Default::default()
        })
        .await
        .expect("Chat failed");

    println!("Model: {}", resp.model);
    // stop_reason is canonical across every provider — match it directly.
    if resp.is_refusal() {
        println!("Request was declined (stop_reason = refusal); no answer to show.");
        return;
    }
    println!("Stop reason: {}", resp.stop_reason);
    println!("Response: {}", resp.text());
    if let Some(usage) = &resp.usage {
        println!(
            "Tokens: {} in / {} out (cost: {} ticks)",
            usage.input_tokens, usage.output_tokens, usage.cost_ticks
        );
    }
    println!("Request ID: {}\n", resp.request_id);

    // --- Streaming example ---
    println!("=== Streaming Chat ===");

    let mut stream = client
        .chat_stream(&ChatRequest {
            model: "claude-opus-4-8".into(),
            messages: vec![ChatMessage::user("Count from 1 to 5, one number per line.")],
            ..Default::default()
        })
        .await
        .expect("ChatStream failed");

    while let Some(ev) = stream.next().await {
        match ev.event_type.as_str() {
            "content_delta" => {
                if let Some(delta) = &ev.delta {
                    print!("{}", delta.text);
                }
            }
            "usage" => {
                if let Some(usage) = &ev.usage {
                    println!("\n[Cost: {} ticks]", usage.cost_ticks);
                }
            }
            "error" => {
                eprintln!("Stream error: {}", ev.error.as_deref().unwrap_or("unknown"));
                std::process::exit(1);
            }
            "done" => {
                println!("\n[Stream complete]");
            }
            _ => {}
        }
    }

    // --- Qwen hybrid-thinking example ---
    // reasoning_effort maps onto DashScope's enable_thinking: "none" turns
    // thinking off entirely; any other value enables it. The thinking trace
    // arrives in a separate channel from the answer.
    println!("=== Qwen Hybrid Thinking ===");

    let resp = client
        .chat(&ChatRequest {
            model: "qwen3.8-max".into(),
            messages: vec![ChatMessage::user(
                "In one sentence, why do hash maps have O(1) lookups?",
            )],
            reasoning_effort: Some("high".into()),
            ..Default::default()
        })
        .await
        .expect("Qwen chat failed");

    println!("Model: {}", resp.model);
    println!(
        "Thinking: {} chars of chain-of-thought",
        resp.thinking().len()
    );
    println!("Response: {}", resp.text());
    if let Some(usage) = &resp.usage {
        println!(
            "Tokens: {} in / {} out (cost: {} ticks)",
            usage.input_tokens, usage.output_tokens, usage.cost_ticks
        );
    }
    println!("Request ID: {}", resp.request_id);
}
