//! Integration tests for the HeyGen v3 gateway routes (avatar realtime,
//! audio sounds search, template v3, batch videos).
//!
//! The gateway is mocked with a tiny in-process HTTP server (tokio only —
//! no extra dev-dependencies) that captures each request's method, path,
//! headers, and body for assertion, and replies with a canned response.
//! Production is never called.

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use quantum_sdk::{
    AudioSoundsQuery, AvatarRealtimeRequest, AvatarRealtimeTextRequest, Client, Error,
    VideoBatchStatusQuery, VideoBatchSubmitRequest, VideoTemplateGenerateRequest,
};

const TEST_KEY: &str = "qai_test_key";

/// A captured HTTP request as seen by the mock gateway.
#[derive(Debug, Clone)]
struct Captured {
    method: String,
    path: String,
    /// Header names lowercased.
    headers: Vec<(String, String)>,
    body: String,
}

impl Captured {
    fn header(&self, name: &str) -> Option<&str> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.as_str())
    }

    fn body_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.body).expect("request body should be JSON")
    }
}

/// Handle to a running mock gateway serving one fixed response.
struct MockGateway {
    base_url: String,
    requests: Arc<Mutex<Vec<Captured>>>,
}

impl MockGateway {
    /// The single request captured by the mock (fails if 0 or >1 arrived).
    fn only_request(&self) -> Captured {
        let reqs = self.requests.lock().unwrap();
        assert_eq!(reqs.len(), 1, "expected exactly one request, got {reqs:?}");
        reqs[0].clone()
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Spawns a mock gateway that answers every request with the given status,
/// extra headers, and JSON body.
async fn mock_gateway(
    status: u16,
    extra_headers: Vec<(String, String)>,
    body: String,
) -> MockGateway {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let requests: Arc<Mutex<Vec<Captured>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let captured = captured.clone();
            let extra_headers = extra_headers.clone();
            let body = body.clone();

            tokio::spawn(async move {
                // Read until end of headers.
                let mut buf: Vec<u8> = Vec::new();
                let mut tmp = [0u8; 8192];
                let header_end = loop {
                    if let Some(pos) = find_subslice(&buf, b"\r\n\r\n") {
                        break pos;
                    }
                    let Ok(n) = stream.read(&mut tmp).await else {
                        return;
                    };
                    if n == 0 {
                        return;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                };

                let head = String::from_utf8_lossy(&buf[..header_end]).to_string();
                let mut lines = head.split("\r\n");
                let request_line = lines.next().unwrap_or_default();
                let mut parts = request_line.split_whitespace();
                let method = parts.next().unwrap_or_default().to_string();
                let path = parts.next().unwrap_or_default().to_string();

                let headers: Vec<(String, String)> = lines
                    .filter_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        Some((name.trim().to_ascii_lowercase(), value.trim().to_string()))
                    })
                    .collect();

                let content_length: usize = headers
                    .iter()
                    .find(|(n, _)| n == "content-length")
                    .and_then(|(_, v)| v.parse().ok())
                    .unwrap_or(0);

                // Read the remainder of the body.
                let body_start = header_end + 4;
                while buf.len() < body_start + content_length {
                    let Ok(n) = stream.read(&mut tmp).await else {
                        return;
                    };
                    if n == 0 {
                        break;
                    }
                    buf.extend_from_slice(&tmp[..n]);
                }
                let req_body = String::from_utf8_lossy(
                    &buf[body_start..buf.len().min(body_start + content_length)],
                )
                .to_string();

                captured.lock().unwrap().push(Captured {
                    method,
                    path,
                    headers,
                    body: req_body,
                });

                let reason = match status {
                    200 => "OK",
                    202 => "Accepted",
                    400 => "Bad Request",
                    402 => "Payment Required",
                    404 => "Not Found",
                    _ => "Unknown",
                };
                let mut resp = format!(
                    "HTTP/1.1 {status} {reason}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n",
                    body.len()
                );
                for (name, value) in &extra_headers {
                    resp.push_str(&format!("{name}: {value}\r\n"));
                }
                resp.push_str("\r\n");
                resp.push_str(&body);
                let _ = stream.write_all(resp.as_bytes()).await;
                let _ = stream.shutdown().await;
            });
        }
    });

    MockGateway {
        base_url: format!("http://{addr}"),
        requests,
    }
}

fn client_for(gw: &MockGateway) -> Client {
    Client::builder(TEST_KEY)
        .base_url(gw.base_url.clone())
        .build()
        .unwrap()
}

fn assert_auth(req: &Captured) {
    assert_eq!(
        req.header("authorization"),
        Some(format!("Bearer {TEST_KEY}").as_str())
    );
    assert_eq!(req.header("x-api-key"), Some(TEST_KEY));
}

// ---------------------------------------------------------------------------
// 1. POST /qai/v1/avatar/realtime
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_avatar_realtime_session() {
    let gw = mock_gateway(
        200,
        vec![
            ("X-QAI-Cost-Ticks".into(), "345000000000".into()),
            ("X-QAI-Balance-After".into(), "655000000000".into()),
        ],
        r#"{
            "stream_id": "rt_9f2c1a",
            "status": "pending",
            "prepaid_seconds": 300,
            "cost_ticks": 345000000000,
            "request_id": "req_abc123def456"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .create_avatar_realtime_session(&AvatarRealtimeRequest {
            session_type: "text_stream".into(),
            avatar_id: "Abigail_expressive_2024112501".into(),
            voice_id: Some("73c0b6a2e29d4d38aca41454bf58c955".into()),
            text: Some("Hello! Let me think about that...".into()),
            audio: None,
            max_duration_seconds: 300,
        })
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/avatar/realtime");
    assert_auth(&req);
    let body = req.body_json();
    assert_eq!(body["type"], "text_stream");
    assert_eq!(body["avatar_id"], "Abigail_expressive_2024112501");
    assert_eq!(body["voice_id"], "73c0b6a2e29d4d38aca41454bf58c955");
    assert_eq!(body["text"], "Hello! Let me think about that...");
    assert_eq!(body["max_duration_seconds"], 300);
    assert!(
        body.get("audio").is_none(),
        "audio must be omitted for text_stream"
    );

    assert_eq!(resp.stream_id, "rt_9f2c1a");
    assert_eq!(resp.status, "pending");
    assert_eq!(resp.prepaid_seconds, 300);
    assert_eq!(resp.cost_ticks, 345_000_000_000);
    // balance_after only surfaces via the X-QAI-Balance-After header.
    assert_eq!(resp.balance_after, 655_000_000_000);
    assert_eq!(resp.request_id, "req_abc123def456");
}

// ---------------------------------------------------------------------------
// 2. GET /qai/v1/avatar/realtime/{id}
// ---------------------------------------------------------------------------

#[tokio::test]
async fn get_avatar_realtime_session_streaming() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "stream_id": "rt_9f2c1a",
            "status": "streaming",
            "hls_url": "https://cdn.heygen.com/realtime/rt_9f2c1a/index.m3u8",
            "request_id": "req_abc123def457"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .get_avatar_realtime_session("rt_9f2c1a")
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/qai/v1/avatar/realtime/rt_9f2c1a");
    assert_auth(&req);

    assert_eq!(resp.stream_id, "rt_9f2c1a");
    assert_eq!(resp.status, "streaming");
    assert_eq!(
        resp.hls_url.as_deref(),
        Some("https://cdn.heygen.com/realtime/rt_9f2c1a/index.m3u8")
    );
    assert!(resp.error_message.is_none());
    assert!(resp.end_reason.is_none());
}

#[tokio::test]
async fn get_avatar_realtime_session_completed_omits_optionals() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "stream_id": "rt_9f2c1a",
            "status": "completed",
            "end_reason": "idle_timeout",
            "request_id": "req_x"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .get_avatar_realtime_session("rt_9f2c1a")
        .await
        .unwrap();
    assert_eq!(resp.status, "completed");
    assert!(resp.hls_url.is_none());
    assert_eq!(resp.end_reason.as_deref(), Some("idle_timeout"));
}

// ---------------------------------------------------------------------------
// 3. POST /qai/v1/avatar/realtime/{id}/text
// ---------------------------------------------------------------------------

#[tokio::test]
async fn send_avatar_realtime_text() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "ok": true,
            "buffered_bytes": 512,
            "final": true,
            "request_id": "req_abc123def458"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .send_avatar_realtime_text(
            "rt_9f2c1a",
            &AvatarRealtimeTextRequest {
                delta: " and here is the rest of my answer.".into(),
                is_final: true,
            },
        )
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/avatar/realtime/rt_9f2c1a/text");
    assert_auth(&req);
    let body = req.body_json();
    assert_eq!(body["delta"], " and here is the rest of my answer.");
    assert_eq!(body["final"], true);

    assert!(resp.ok);
    assert_eq!(resp.buffered_bytes, 512);
    assert!(resp.is_final);
    assert_eq!(resp.request_id, "req_abc123def458");
}

// ---------------------------------------------------------------------------
// 4. POST /qai/v1/avatar/realtime/{id}/cancel
// ---------------------------------------------------------------------------

#[tokio::test]
async fn cancel_avatar_realtime_session() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "stream_id": "rt_9f2c1a",
            "cancelled": true,
            "request_id": "req_abc123def459"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .cancel_avatar_realtime_session("rt_9f2c1a")
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/avatar/realtime/rt_9f2c1a/cancel");
    assert_auth(&req);

    assert_eq!(resp.stream_id, "rt_9f2c1a");
    assert!(resp.cancelled);
    assert_eq!(resp.request_id, "req_abc123def459");
}

// ---------------------------------------------------------------------------
// 5. GET /qai/v1/audio/sounds
// ---------------------------------------------------------------------------

#[tokio::test]
async fn search_audio_sounds() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "sounds": [
                {
                    "id": "trk_8842aa",
                    "name": "Uplifting Corporate",
                    "description": "Bright, optimistic corporate track with piano and strings",
                    "audio_url": "https://resource.heygen.ai/sounds/trk_8842aa.wav?sig=abc",
                    "duration": 94.5,
                    "score": 0.91,
                    "type": "music"
                }
            ],
            "has_more": true,
            "next_token": "eyJvZmZzZXQiOjEwfQ",
            "request_id": "req_abc123def45a"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .search_audio_sounds(&AudioSoundsQuery {
            query: "calm piano".into(),
            sound_type: Some("music".into()),
            limit: Some(10),
            min_score: Some(0.7),
            token: None,
        })
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(
        req.path,
        "/qai/v1/audio/sounds?query=calm%20piano&type=music&limit=10&min_score=0.7"
    );
    assert_auth(&req);

    assert_eq!(resp.sounds.len(), 1);
    let track = &resp.sounds[0];
    assert_eq!(track.id, "trk_8842aa");
    assert_eq!(track.name, "Uplifting Corporate");
    assert_eq!(track.duration, 94.5);
    assert_eq!(track.score, 0.91);
    assert_eq!(track.sound_type, "music");
    assert!(track.audio_url.ends_with(".wav?sig=abc"));
    assert!(resp.has_more);
    assert_eq!(resp.next_token, "eyJvZmZzZXQiOjEwfQ");
    assert_eq!(resp.request_id, "req_abc123def45a");
}

// ---------------------------------------------------------------------------
// 6. GET /qai/v1/video/template/{id}
// ---------------------------------------------------------------------------

#[tokio::test]
async fn video_template_detail() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "template": {
                "id": "tmpl_5f0a",
                "name": "Product Launch",
                "aspect_ratio": "16:9",
                "variables": {
                    "headline": { "type": "text", "content": "Default headline" },
                    "presenter": { "type": "character", "character_id": "Abigail_expressive_2024112501", "character_type": "avatar" }
                },
                "scenes": [
                    {
                        "scene_id": "scene_1",
                        "script": "Introducing {{headline}}...",
                        "variables": [ { "name": "headline", "variable_type": "text" } ]
                    }
                ]
            },
            "request_id": "req_abc123def45b"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client.video_template_detail("tmpl_5f0a").await.unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/qai/v1/video/template/tmpl_5f0a");
    assert_auth(&req);

    assert_eq!(resp.template.id, "tmpl_5f0a");
    assert_eq!(resp.template.name, "Product Launch");
    assert_eq!(resp.template.aspect_ratio, "16:9");
    // Union variable values round-trip as raw JSON.
    let headline = &resp.template.variables["headline"];
    assert_eq!(headline["type"], "text");
    assert_eq!(headline["content"], "Default headline");
    let presenter = &resp.template.variables["presenter"];
    assert_eq!(presenter["type"], "character");
    assert_eq!(presenter["character_id"], "Abigail_expressive_2024112501");
    assert_eq!(resp.template.scenes.len(), 1);
    assert_eq!(resp.template.scenes[0].scene_id, "scene_1");
    assert_eq!(
        resp.template.scenes[0].script,
        "Introducing {{headline}}..."
    );
    assert_eq!(resp.template.scenes[0].variables[0].name, "headline");
    assert_eq!(resp.template.scenes[0].variables[0].variable_type, "text");
    assert_eq!(resp.request_id, "req_abc123def45b");
}

// ---------------------------------------------------------------------------
// 7. POST /qai/v1/video/template/{id} (async job)
// ---------------------------------------------------------------------------

#[tokio::test]
async fn video_template_generate() {
    let gw = mock_gateway(
        202,
        vec![],
        r#"{
            "job_id": "qai_job_3def45c00112",
            "status": "pending",
            "type": "video/template-v3",
            "request_id": "req_abc123def45c"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let mut variables = std::collections::HashMap::new();
    variables.insert(
        "headline".to_string(),
        serde_json::json!({ "type": "text", "content": "Cosmic Duck 2.0" }),
    );

    let resp = client
        .video_template_generate(
            "tmpl_5f0a",
            &VideoTemplateGenerateRequest {
                variables,
                title: Some("Launch video".into()),
                fps: Some(30),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/video/template/tmpl_5f0a");
    assert_auth(&req);
    let body = req.body_json();
    assert_eq!(body["variables"]["headline"]["type"], "text");
    assert_eq!(body["variables"]["headline"]["content"], "Cosmic Duck 2.0");
    assert_eq!(body["title"], "Launch video");
    assert_eq!(body["fps"], 30);
    // Unset optionals must be omitted, not null.
    assert!(body.get("caption").is_none());
    assert!(body.get("scene_ids").is_none());
    assert!(body.get("dimension").is_none());
    assert!(body.get("subtitles").is_none());
    assert!(body.get("reorder_music").is_none());

    assert_eq!(resp.job_id, "qai_job_3def45c00112");
    assert_eq!(resp.status, "pending");
    assert_eq!(resp.job_type.as_deref(), Some("video/template-v3"));
    assert_eq!(resp.request_id.as_deref(), Some("req_abc123def45c"));
}

// ---------------------------------------------------------------------------
// 8. POST /qai/v1/video/batch
// ---------------------------------------------------------------------------

#[tokio::test]
async fn video_batch_submit() {
    let gw = mock_gateway(
        202,
        vec![],
        r#"{
            "batch_id": "batch_66aa1c",
            "status": "processing",
            "total_items": 2,
            "request_id": "req_abc123def45d"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .video_batch_submit(&VideoBatchSubmitRequest {
            title: Some("Onboarding videos".into()),
            videos: vec![
                serde_json::json!({
                    "type": "avatar",
                    "avatar_id": "Abigail_expressive_2024112501",
                    "voice_id": "73c0b6a2",
                    "script": "Welcome to the team!"
                }),
                serde_json::json!({
                    "type": "avatar",
                    "avatar_id": "Abigail_expressive_2024112501",
                    "voice_id": "73c0b6a2",
                    "script": "Here is how billing works."
                }),
            ],
        })
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/video/batch");
    assert_auth(&req);
    let body = req.body_json();
    assert_eq!(body["title"], "Onboarding videos");
    let videos = body["videos"].as_array().unwrap();
    assert_eq!(videos.len(), 2);
    assert_eq!(videos[0]["type"], "avatar");
    assert_eq!(videos[0]["script"], "Welcome to the team!");

    assert_eq!(resp.batch_id, "batch_66aa1c");
    assert_eq!(resp.status, "processing");
    assert_eq!(resp.total_items, 2);
    assert_eq!(resp.request_id, "req_abc123def45d");
}

// ---------------------------------------------------------------------------
// 9. GET /qai/v1/video/batch/{id}
// ---------------------------------------------------------------------------

#[tokio::test]
async fn video_batch_status_settled() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "batch_id": "batch_66aa1c",
            "title": "Onboarding videos",
            "status": "completed",
            "total_items": 3,
            "counts_by_status": { "completed": 2, "failed": 1 },
            "created_at": 1752741600,
            "items": [
                { "item_index": 0, "status": "completed", "video_id": "vid_001", "video_url": "https://resource.heygen.ai/video/vid_001.mp4?sig=x" },
                { "item_index": 1, "status": "completed", "video_id": "vid_002", "video_url": "https://resource.heygen.ai/video/vid_002.mp4?sig=y" },
                { "item_index": 2, "status": "failed", "error": { "code": "avatar_not_found", "message": "avatar id not found" } }
            ],
            "has_more": false,
            "next_token": "",
            "billing_status": "settled",
            "cost_ticks": 46000000000,
            "request_id": "req_abc123def45e"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .video_batch_status(
            "batch_66aa1c",
            &VideoBatchStatusQuery {
                limit: Some(50),
                token: Some("eyJvZmZzZXQiOjB9".into()),
            },
        )
        .await
        .unwrap();

    let req = gw.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(
        req.path,
        "/qai/v1/video/batch/batch_66aa1c?limit=50&token=eyJvZmZzZXQiOjB9"
    );
    assert_auth(&req);

    assert_eq!(resp.batch_id, "batch_66aa1c");
    assert_eq!(resp.title, "Onboarding videos");
    assert_eq!(resp.status, "completed");
    assert_eq!(resp.total_items, 3);
    assert_eq!(resp.counts_by_status["completed"], 2);
    assert_eq!(resp.counts_by_status["failed"], 1);
    assert_eq!(resp.created_at, 1_752_741_600);
    assert_eq!(resp.items.len(), 3);
    assert_eq!(resp.items[0].item_index, 0);
    assert_eq!(resp.items[0].status, "completed");
    assert_eq!(resp.items[0].video_id.as_deref(), Some("vid_001"));
    assert!(
        resp.items[0]
            .video_url
            .as_deref()
            .unwrap()
            .contains("vid_001.mp4")
    );
    let failed = &resp.items[2];
    assert_eq!(failed.status, "failed");
    assert!(failed.video_url.is_none());
    let err = failed.error.as_ref().unwrap();
    assert_eq!(err.code, "avatar_not_found");
    assert_eq!(err.message, "avatar id not found");
    assert!(!resp.has_more);
    assert_eq!(resp.next_token, "");
    assert_eq!(resp.billing_status, "settled");
    assert_eq!(resp.cost_ticks, 46_000_000_000);
    assert_eq!(resp.request_id, "req_abc123def45e");
}

#[tokio::test]
async fn video_batch_status_unsettled_withholds_urls() {
    let gw = mock_gateway(
        200,
        vec![],
        r#"{
            "batch_id": "batch_66aa1c",
            "title": "",
            "status": "completed",
            "total_items": 1,
            "counts_by_status": { "completed": 1 },
            "created_at": 1752741600,
            "items": [
                { "item_index": 0, "status": "completed", "video_id": "vid_001" }
            ],
            "has_more": false,
            "next_token": "",
            "billing_status": "settlement_pending",
            "cost_ticks": 0,
            "request_id": "req_y"
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let resp = client
        .video_batch_status("batch_66aa1c", &VideoBatchStatusQuery::default())
        .await
        .unwrap();

    // No query params when the query struct is empty.
    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/video/batch/batch_66aa1c");

    assert_eq!(resp.billing_status, "settlement_pending");
    assert_eq!(resp.cost_ticks, 0);
    assert!(
        resp.items[0].video_url.is_none(),
        "URLs are withheld until settled"
    );
}

// ---------------------------------------------------------------------------
// Error envelope decoding
// ---------------------------------------------------------------------------

#[tokio::test]
async fn insufficient_balance_error_envelope() {
    let gw = mock_gateway(
        402,
        vec![],
        r#"{
            "error": {
                "message": "out of credits — top up to continue",
                "type": "insufficient_balance",
                "code": "INSUFFICIENT_BALANCE"
            }
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let err = client
        .create_avatar_realtime_session(&AvatarRealtimeRequest {
            session_type: "tts".into(),
            avatar_id: "av".into(),
            voice_id: Some("v".into()),
            text: Some("hi".into()),
            audio: None,
            max_duration_seconds: 60,
        })
        .await
        .unwrap_err();

    match err {
        Error::Api(api) => {
            assert_eq!(api.status_code, 402);
            assert_eq!(api.code, "INSUFFICIENT_BALANCE");
            assert!(api.message.contains("out of credits"));
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}

#[tokio::test]
async fn not_found_error_envelope() {
    let gw = mock_gateway(
        404,
        vec![],
        r#"{
            "error": {
                "message": "session rt_missing not found",
                "type": "not_found",
                "code": "not_found"
            }
        }"#
        .to_string(),
    )
    .await;
    let client = client_for(&gw);

    let err = client
        .get_avatar_realtime_session("rt_missing")
        .await
        .unwrap_err();

    match err {
        Error::Api(api) => {
            assert_eq!(api.status_code, 404);
            assert_eq!(api.code, "not_found");
        }
        other => panic!("expected Error::Api, got {other:?}"),
    }
}
