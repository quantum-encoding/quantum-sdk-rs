# quantum-sdk

Rust client SDK for the [Quantum AI API](https://api.quantumencoding.ai).

```bash
cargo add quantum-sdk
```

Every code block below is compiled by `cargo test --doc`, so it matches the
crate you install.

## Quick Start

```rust,no_run
use quantum_sdk::{ChatMessage, ChatRequest, Client};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new("qai_k_your_key_here")?;
    let response = client.chat(&ChatRequest {
        model: "qwen3.8-max".into(),
        messages: vec![ChatMessage::user("Hello! What is quantum computing?")],
        ..Default::default()
    }).await?;
    println!("{}", response.text());
    Ok(())
}
```

## Features

- 110+ endpoints across 11 AI providers and 50+ models
- Async/await with Tokio runtime
- Streaming via `ChatStream` with SSE parsing
- Strongly typed request/response structs
- Agent orchestration with SSE event streams
- GPU/CPU compute rental (requires per-account admin approval)
- Batch processing (async jobs)
- Zero-copy deserialization with serde

## Examples

### Chat Completion

```rust,no_run
use quantum_sdk::{ChatMessage, ChatRequest, Client};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;

    let response = client.chat(&ChatRequest {
        model: "claude-opus-4-8".into(),
        messages: vec![
            ChatMessage::system("You are a helpful assistant."),
            ChatMessage::user("Explain ownership in Rust"),
        ],
        temperature: Some(0.7),
        max_tokens: Some(1000),
        ..Default::default()
    }).await?;

    println!("{}", response.text());
    Ok(())
}
```

### Qwen (Alibaba Model Studio)

Hybrid-thinking Qwen models stream their chain of thought alongside the
answer. `reasoning_effort` maps onto Qwen's `enable_thinking` — `"none"`
switches thinking off (cheaper, faster); `None` keeps the model default.
Lineup: `qwen3.8-max`, `qwen3.7-plus`, `qwen3.6-flash`, `qwen-turbo`,
`qwen3-coder-plus`, `qwen3-coder-flash`, `qwen-vl-max` (vision).

```rust,no_run
use quantum_sdk::{ChatMessage, ChatRequest, Client};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;

    let response = client.chat(&ChatRequest {
        model: "qwen3.8-max".into(),
        messages: vec![ChatMessage::user("Plan a migration from REST to gRPC")],
        reasoning_effort: Some("high".into()),
        ..Default::default()
    }).await?;

    println!("thinking: {}", response.thinking());
    println!("answer: {}", response.text());
    Ok(())
}
```

### Streaming

`ChatStream` yields `StreamEvent`s, not `Result`s: a failure after the
stream opens arrives as an event whose `error` is set (type `error`,
`invalid_request` or `rate_limit`), followed by `done`.

```rust,no_run
use futures_util::StreamExt;
use quantum_sdk::{ChatMessage, ChatRequest, Client};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;

    let mut stream = client.chat_stream(&ChatRequest {
        model: "claude-opus-4-8".into(),
        messages: vec![ChatMessage::user("Write a haiku about Rust")],
        ..Default::default()
    }).await?;

    while let Some(event) = stream.next().await {
        if let Some(delta) = &event.delta {
            print!("{}", delta.text);
        }
        if let Some(message) = &event.error {
            eprintln!("stream failed: {message}");
        }
        if let Some(usage) = &event.usage {
            // On a stream, output_tokens excludes reasoning; cost_ticks covers both.
            println!("\n{} ticks", usage.cost_ticks);
        }
    }
    Ok(())
}
```

### Image Generation

```rust,no_run
use quantum_sdk::{Client, ImageRequest};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;

    let images = client.generate_image(&ImageRequest {
        model: "grok-imagine-image".into(),
        prompt: "A cosmic duck in space".into(),
        ..Default::default()
    }).await?;

    for image in &images.images {
        println!("{} bytes of base64 {}", image.base64.len(), image.format);
    }
    Ok(())
}
```

### Text-to-Speech

```rust,no_run
use quantum_sdk::{Client, TextToSpeechRequest};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;

    let audio = client.speak(&TextToSpeechRequest {
        model: "gpt-4o-mini-tts".into(),
        text: "Welcome to Quantum AI!".into(),
        voice: Some("alloy".into()),
        output_format: Some("mp3".into()),
        ..Default::default()
    }).await?;

    println!("{} bytes of {}", audio.size_bytes, audio.format);
    Ok(())
}
```

### Web Search

```rust,no_run
use quantum_sdk::{Client, WebSearchRequest};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;

    let results = client.web_search(&WebSearchRequest {
        query: "latest Rust releases 2026".into(),
        ..Default::default()
    }).await?;

    for result in &results.web {
        println!("{}: {}", result.title, result.url);
    }
    Ok(())
}
```

## Retries and idempotency

A `GET` is replayed on 429, 502, 503 and 504 (up to three times). A `POST`
is replayed on 429 only, honouring the gateway's `Retry-After`. The gateway
does not dedupe on `Idempotency-Key` for chat, session chat or any media
route, and key-minting and Stripe checkout routes have no dedupe at all, so
replaying a `POST` after a 5xx that masked a completed operation would run
and charge it again. Routes that do dedupe (agent, jobs, batch, search,
scanner, rag, documents, vision, voice, compute, inference, missions,
cloudrun, security) can opt in through `post_json_with_idempotency`; see
its docs for the dedupe cache's limits.

## All Endpoints

| Category | Endpoints | Description |
|----------|-----------|-------------|
| Chat | 3 | Text generation, session chat (buffered and streaming) |
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
| Compute | 7 | GPU/CPU rental (admin-approved accounts only) |
| Auth | 5 | Apple, Google and Firebase sign-in, key verification, sign-out |
| Keys | 8 | Create, list, revoke, rotate, usage, device, ephemeral and partner keys |
| Account | 5 | Balance, usage, summary, deletion |
| Credits | 8 | Packs, tiers, purchase, lifetime plans |
| Batch | 4 | Async batch job processing |
| Realtime | 3 | Voice sessions |
| Models | 2 | Model list + pricing |

## Authentication

Pass your API key when creating the client. `Client::new` fails, rather
than panicking, on a key that cannot be an HTTP header value (a trailing
newline read from a file is the usual cause).

```rust,no_run
use quantum_sdk::Client;

fn main() -> quantum_sdk::Result<()> {
    let client = Client::new("qai_k_your_key_here")?;
    let _ = client;
    Ok(())
}
```

The SDK sends the key as both `Authorization: Bearer <key>` and
`X-API-Key: <key>` on every request, streaming included; the gateway reads
`X-API-Key` first and falls back to the bearer. Both `qai_...` (primary) and
`qai_k_...` (scoped) keys are supported.

Get your API key at [cosmicduck.dev](https://cosmicduck.dev).

### Signing a person in (no developer key)

An app that signs its user in never holds a developer key. Exchange the
identity token from Google, Apple or Firebase for a session, then build the
client on the session token — it is the credential for everything after,
and the response also carries the account's default API key for clients
that persist a key instead.

```rust,no_run
use quantum_sdk::{AuthGoogleRequest, Client};

#[tokio::main]
async fn main() -> quantum_sdk::Result<()> {
    let google_id_token = std::env::var("GOOGLE_ID_TOKEN").unwrap_or_default();

    let bootstrap = Client::new("unauthenticated")?;
    let session = bootstrap.auth_google(&AuthGoogleRequest {
        id_token: google_id_token,
        client_id: "your-oauth-client-id.apps.googleusercontent.com".into(),
        device_id: None,
    }).await?;

    let client = Client::new(session.token)?;
    // ... chat, images, audio — billed to the signed-in account.

    client.revoke_session().await?; // sign out (session tokens only)
    Ok(())
}
```

`auth_apple` and `auth_firebase` answer with the same `AuthResponse`. A
service that accepts a customer's `qai_k_` key can resolve its owner with
`verify_key`.

## Pricing

See [api.quantumencoding.ai/pricing](https://api.quantumencoding.ai/pricing) for current rates,
or call `get_pricing()` for the same table keyed by model id.

The **Lifetime tier** offers 0% margin at-cost pricing via a one-time payment.

## Other SDKs

| Language | Package | Install |
|----------|---------|---------|
| **Rust** | quantum-sdk | `cargo add quantum-sdk` |
| Go | quantum-sdk | `go get github.com/quantum-encoding/quantum-sdk` |
| TypeScript | quantum-ai-sdk | `npm i quantum-ai-sdk` |
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
