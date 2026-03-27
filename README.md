# quantum-sdk

Rust client SDK for the [Quantum AI API](https://api.quantumencoding.ai).

```bash
cargo add quantum-sdk
```

## Quick Start

```rust
use quantum_sdk::Client;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = Client::new("qai_k_your_key_here");
    let response = client.chat("gemini-2.5-flash", "Hello! What is quantum computing?").await?;
    println!("{}", response.text());
    Ok(())
}
```

## Features

- 110+ endpoints across 10 AI providers and 45+ models
- Async/await with Tokio runtime
- Streaming via `ChatStream` with SSE parsing
- Strongly typed request/response structs
- Agent orchestration with SSE event streams
- GPU/CPU compute rental
- Batch processing (50% discount)
- Zero-copy deserialization with serde

## Examples

### Chat Completion

```rust
use quantum_sdk::{Client, ChatRequest, ChatMessage};

let client = Client::new("qai_k_your_key_here");

let response = client.chat_request(ChatRequest {
    model: "claude-sonnet-4-6".into(),
    messages: vec![
        ChatMessage::system("You are a helpful assistant."),
        ChatMessage::user("Explain ownership in Rust"),
    ],
    temperature: Some(0.7),
    max_tokens: Some(1000),
    ..Default::default()
}).await?;

println!("{}", response.text());
```

### Streaming

```rust
use quantum_sdk::{Client, ChatRequest, ChatMessage};
use futures::StreamExt;

let client = Client::new("qai_k_your_key_here");

let mut stream = client.chat_stream(ChatRequest {
    model: "claude-sonnet-4-6".into(),
    messages: vec![ChatMessage::user("Write a haiku about Rust")],
    ..Default::default()
}).await?;

while let Some(event) = stream.next().await {
    let event = event?;
    if let Some(delta) = event.delta_text() {
        print!("{}", delta);
    }
}
```

### Image Generation

```rust
let images = client.generate_image("grok-imagine-image", "A cosmic duck in space").await?;
for image in &images.images {
    println!("{}", image.url.as_deref().unwrap_or("base64"));
}
```

### Text-to-Speech

```rust
let audio = client.speak("Welcome to Quantum AI!", "alloy", "mp3").await?;
println!("{}", audio.audio_url);
```

### Web Search

```rust
let results = client.web_search("latest Rust releases 2026").await?;
for result in &results.results {
    println!("{}: {}", result.title, result.url);
}
```

### Agent Orchestration

```rust
use futures::StreamExt;

let mut stream = client.agent_run("Research quantum computing breakthroughs").await?;
while let Some(event) = stream.next().await {
    let event = event?;
    match event.event_type.as_str() {
        "content_delta" => print!("{}", event.content.unwrap_or_default()),
        "done" => println!("\n--- Done ---"),
        _ => {}
    }
}
```

## All Endpoints

| Category | Endpoints | Description |
|----------|-----------|-------------|
| Chat | 2 | Text generation + session chat |
| Agent | 2 | Multi-step orchestration + missions |
| Images | 2 | Generation + editing |
| Video | 7 | Generation, studio, translation, avatars |
| Audio | 13 | TTS, STT, music, dialogue, dubbing, voice design |
| Voices | 5 | Clone, list, delete, library, design |
| Embeddings | 1 | Text embeddings |
| RAG | 4 | Vertex AI + SurrealDB search |
| Documents | 3 | Extract, chunk, process |
| Search | 3 | Web search, context, answers |
| Scanner | 11 | Code scanning, type queries, diffs |
| Scraper | 2 | Doc scraping + screenshots |
| Jobs | 3 | Async job management |
| Compute | 7 | GPU/CPU rental |
| Keys | 3 | API key management |
| Account | 3 | Balance, usage, summary |
| Credits | 6 | Packs, tiers, lifetime, purchase |
| Batch | 4 | 50% discount batch processing |
| Realtime | 3 | Voice sessions |
| Models | 2 | Model list + pricing |

## Authentication

Pass your API key when creating the client:

```rust
let client = Client::new("qai_k_your_key_here");
```

The SDK sends it as the `X-API-Key` header. Both `qai_...` (primary) and `qai_k_...` (scoped) keys are supported. You can also use `Authorization: Bearer <key>`.

Get your API key at [cosmicduck.dev](https://cosmicduck.dev).

## Pricing

See [api.quantumencoding.ai/pricing](https://api.quantumencoding.ai/pricing) for current rates.

The **Lifetime tier** offers 0% margin at-cost pricing via a one-time payment.

## Other SDKs

All SDKs are at v0.4.0 with type parity verified by scanner.

| Language | Package | Install |
|----------|---------|---------|
| **Rust** | quantum-sdk | `cargo add quantum-sdk` |
| Go | quantum-sdk | `go get github.com/quantum-encoding/quantum-sdk` |
| TypeScript | @quantum-encoding/quantum-sdk | `npm i @quantum-encoding/quantum-sdk` |
| Python | quantum-sdk | `pip install quantum-sdk` |
| Swift | QuantumSDK | Swift Package Manager |
| Kotlin | quantum-sdk | Gradle dependency |

MCP server: `npx @quantum-encoding/ai-conductor-mcp`

## API Reference

- Interactive docs: [api.quantumencoding.ai/docs](https://api.quantumencoding.ai/docs)
- OpenAPI spec: [api.quantumencoding.ai/openapi.yaml](https://api.quantumencoding.ai/openapi.yaml)
- LLM context: [api.quantumencoding.ai/llms.txt](https://api.quantumencoding.ai/llms.txt)

## License

MIT
