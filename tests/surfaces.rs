//! Integration tests for the gateway surfaces added alongside chat: media
//! sessions, files, context caches, RAG collections, model deployments,
//! licences, and the scanner.
//!
//! Each test asserts the method, path, and body the SDK actually puts on the
//! wire — the things a typed struct cannot catch on its own. The gateway is
//! mocked with a tiny in-process HTTP server (tokio only, no extra
//! dev-dependencies); production is never called.

use std::sync::{Arc, Mutex};

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use quantum_sdk::{
    CacheCreateRequest, Client, CollectionSearchRequest, CreateCollectionRequest,
    DeployModelRequest, DiffRequest, MediaSessionChatRequest, MediaSessionCreateRequest,
};

const TEST_KEY: &str = "qai_test_key";

/// A captured HTTP request as seen by the mock gateway.
#[derive(Debug, Clone)]
struct Captured {
    method: String,
    path: String,
    body: String,
}

impl Captured {
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

    fn client(&self) -> Client {
        Client::builder(TEST_KEY)
            .base_url(&self.base_url)
            .build()
            .expect("mock client builds")
    }
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Spawns a mock gateway that answers every request with 200 and the given
/// JSON body.
async fn mock_gateway(body: &str) -> MockGateway {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let requests: Arc<Mutex<Vec<Captured>>> = Arc::new(Mutex::new(Vec::new()));
    let captured = requests.clone();
    let body = body.to_string();

    tokio::spawn(async move {
        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };
            let captured = captured.clone();
            let body = body.clone();

            tokio::spawn(async move {
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

                let content_length: usize = lines
                    .filter_map(|line| line.split_once(':'))
                    .find(|(name, _)| name.trim().eq_ignore_ascii_case("content-length"))
                    .and_then(|(_, value)| value.trim().parse().ok())
                    .unwrap_or(0);

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
                    body: req_body,
                });

                let resp = format!(
                    "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                    body.len()
                );
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

#[tokio::test]
async fn media_session_create_posts_the_file_and_model() {
    let gw = mock_gateway(
        r#"{"id":"ms_1","file_uri":"files/abc","mime_type":"video/mp4",
            "cache_name":"cachedContents/x","model":"gemini-3.1-flash-lite",
            "cache_token_count":51234,"history":[],"message_count":0}"#,
    )
    .await;

    let session = gw
        .client()
        .media_session_create(&MediaSessionCreateRequest {
            file_uri: "files/abc".into(),
            mime_type: "video/mp4".into(),
            model: "gemini-3.1-flash-lite".into(),
            cache_ttl_seconds: Some(7200),
            ..Default::default()
        })
        .await
        .expect("create succeeds");

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/media-sessions");
    assert_eq!(req.body_json()["cache_ttl_seconds"], 7200);
    assert_eq!(session.cache_name, "cachedContents/x");
    assert_eq!(session.cache_token_count, 51234);
}

#[tokio::test]
async fn media_session_chat_hits_the_session_subpath() {
    let gw = mock_gateway(
        r#"{"session_id":"ms_1","answer":"chapter three","history":[
            {"role":"user","content":"what is next?","at":"2026-01-01T00:00:00Z"},
            {"role":"assistant","content":"chapter three","at":"2026-01-01T00:00:01Z"}]}"#,
    )
    .await;

    let resp = gw
        .client()
        .media_session_chat(
            "ms_1",
            &MediaSessionChatRequest {
                message: "what is next?".into(),
                max_tokens: Some(512),
                ..Default::default()
            },
        )
        .await
        .expect("chat succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/media-sessions/ms_1/chat");
    assert_eq!(req.body_json()["message"], "what is next?");
    assert_eq!(resp.history.len(), 2);
    assert_eq!(resp.answer, "chapter three");
}

#[tokio::test]
async fn cache_delete_accepts_the_full_resource_name() {
    let gw = mock_gateway(r#"{"deleted":true}"#).await;

    let resp = gw
        .client()
        .cache_delete("cachedContents/abc123")
        .await
        .expect("delete succeeds");

    let req = gw.only_request();
    assert_eq!(req.method, "DELETE");
    assert_eq!(req.path, "/qai/v1/caches/cachedContents/abc123");
    assert!(resp.deleted);
}

#[tokio::test]
async fn cache_create_sends_the_model_scope() {
    let gw = mock_gateway(
        r#"{"cache_name":"cachedContents/abc","model":"gemini-3.1-flash-lite",
            "expires_at":"2026-01-01T01:00:00Z","display_name":"abc","token_count":51234}"#,
    )
    .await;

    let resp = gw
        .client()
        .cache_create(&CacheCreateRequest {
            file_uri: "files/abc".into(),
            mime_type: "video/mp4".into(),
            model: "gemini-3.1-flash-lite".into(),
            system_instruction: Some("answer from the video only".into()),
            ..Default::default()
        })
        .await
        .expect("create succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/caches");
    let body = req.body_json();
    assert_eq!(body["model"], "gemini-3.1-flash-lite");
    assert_eq!(body["system_instruction"], "answer from the video only");
    assert!(body.get("ttl_seconds").is_none());
    assert_eq!(resp.token_count, 51234);
}

#[tokio::test]
async fn collection_search_posts_to_the_collections_subpath() {
    let gw = mock_gateway(
        r#"{"results":[{"content":"ticks are 1e-10 USD","score":0.9,"collection":"docs",
                        "collection_id":"c1","document_id":"d1","filename":"billing.md",
                        "is_shared":false}],
            "query":"billing","collections_searched":1,"request_id":"req_1"}"#,
    )
    .await;

    let results = gw
        .client()
        .collections_search(&CollectionSearchRequest {
            query: "billing".into(),
            collection_ids: vec!["c1".into()],
            max_chunks: Some(3),
        })
        .await
        .expect("search succeeds");

    let req = gw.only_request();
    // The gateway serves this under /rag/collections/search, not
    // /rag/search/collections.
    assert_eq!(req.path, "/qai/v1/rag/collections/search");
    let body = req.body_json();
    assert_eq!(body["max_chunks"], 3);
    assert_eq!(body["collection_ids"][0], "c1");
    assert_eq!(results[0].collection, "docs");
}

#[tokio::test]
async fn collection_get_returns_the_collection_and_its_documents() {
    let gw = mock_gateway(
        r#"{"collection":{"id":"c1","owner":"u1","provider":"xai","name":"docs",
                          "provider_collection_id":"xc1","document_count":1,
                          "created_at":"2026-01-01T00:00:00Z"},
            "documents":[{"id":"d1","collection_id":"c1","file_id":"file_9",
                          "filename":"spec.pdf","status":"indexed","chunks":12,
                          "uploaded_at":"2026-01-01T00:00:00Z"}]}"#,
    )
    .await;

    let detail = gw
        .client()
        .collections_get("c1")
        .await
        .expect("get succeeds");

    let req = gw.only_request();
    assert_eq!(req.method, "GET");
    assert_eq!(req.path, "/qai/v1/rag/collections/c1");
    assert_eq!(detail.collection.name, "docs");
    assert_eq!(detail.documents[0].filename, "spec.pdf");
}

#[tokio::test]
async fn collection_create_carries_description_and_provider() {
    let gw = mock_gateway(
        r#"{"id":"c1","owner":"u1","provider":"xai","name":"docs",
            "description":"gateway docs","provider_collection_id":"xc1",
            "document_count":0,"created_at":"2026-01-01T00:00:00Z"}"#,
    )
    .await;

    let collection = gw
        .client()
        .collections_create(&CreateCollectionRequest {
            name: "docs".into(),
            description: Some("gateway docs".into()),
            provider: Some("xai".into()),
        })
        .await
        .expect("create succeeds");

    let body = gw.only_request().body_json();
    assert_eq!(body["description"], "gateway docs");
    assert_eq!(body["provider"], "xai");
    assert_eq!(collection.provider_collection_id, "xc1");
}

#[tokio::test]
async fn deploy_model_estimate_never_sets_confirmed() {
    let gw = mock_gateway(
        r#"{"cost_per_hour_usd":30.5,"total_estimate_usd":61.0,"total_ticks":610000000000,
            "duration_hours":2,"model_display_name":"Nemotron 3 Super 120B",
            "model":"publishers/nvidia/models/nemotron-3-super","machine_type":"a4-highgpu-8g",
            "accelerator_type":"NVIDIA_B200","accelerator_count":8,"region":"us-east1",
            "note":"resubmit with confirmed:true to deploy"}"#,
    )
    .await;

    let estimate = gw
        .client()
        .compute_deploy_model_estimate(&DeployModelRequest {
            model: "nemotron-3-super-120b".into(),
            confirmed: Some(true), // must be stripped by the estimate call
            ..Default::default()
        })
        .await
        .expect("estimate succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/compute/deploy-model");
    assert!(
        req.body_json().get("confirmed").is_none(),
        "the estimate call must not confirm a deployment"
    );
    assert_eq!(estimate.duration_hours, 2);
    assert_eq!(estimate.total_ticks, 610_000_000_000);
}

#[tokio::test]
async fn deploy_model_confirms_when_provisioning() {
    let gw = mock_gateway(
        r#"{"deployment_id":"dep_1","status":"provisioning",
            "model_display_name":"Nemotron 3 Super 120B","cost_per_hour_usd":30.5,
            "total_cost_usd":61.0,"expires_at":"2026-01-01T02:00:00Z",
            "operation":"projects/p/operations/o","note":"poll GET ..."}"#,
    )
    .await;

    let accepted = gw
        .client()
        .compute_deploy_model(&DeployModelRequest {
            model: "nemotron-3-super-120b".into(),
            duration_hours: Some(2),
            ..Default::default()
        })
        .await
        .expect("deploy succeeds");

    assert_eq!(gw.only_request().body_json()["confirmed"], true);
    assert_eq!(accepted.deployment_id, "dep_1");
    assert_eq!(accepted.status, "provisioning");
}

#[tokio::test]
async fn licenses_mine_filters_by_app() {
    let gw = mock_gateway(
        r#"{"licenses":[{"id":"lic_1","app":"kitchenshare","sku":"pro","source":"stripe",
                         "source_transaction":"pi_1","issued_at":"2026-01-01T00:00:00Z",
                         "expires_at":"2027-01-01T00:00:00Z","status":"active",
                         "license_key":"ey.jwt"}]}"#,
    )
    .await;

    let resp = gw
        .client()
        .licenses_mine(Some("kitchen share"))
        .await
        .expect("list succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/licenses/mine?app=kitchen%20share");
    assert_eq!(resp.licenses[0].license_key, "ey.jwt");
}

#[tokio::test]
async fn scanner_diff_posts_scan_ids_as_base_and_target() {
    let gw = mock_gateway(
        r#"{"base":"rust","target":"go","missing_types":["ChatUsage"],"extra_types":[],
            "missing_fields":{"ChatRequest":["region"]},"completion":0.9,"total_gaps":2}"#,
    )
    .await;

    let diff = gw
        .client()
        .scanner_diff(&DiffRequest {
            base_scan_id: Some("scan_a".into()),
            target_scan_id: Some("scan_b".into()),
            ..Default::default()
        })
        .await
        .expect("diff succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/scanner/diff");
    let body = req.body_json();
    assert_eq!(body["base"], "scan_a");
    assert_eq!(body["target"], "scan_b");
    assert_eq!(diff.total_gaps, 2);
}

#[tokio::test]
async fn scanner_type_query_url_encodes_the_type_name() {
    let gw = mock_gateway(
        r#"{"type":{"name":"Vec<CodeType>","kind":"struct","file":"src/scanner.rs",
                    "fields":[]},"scan_id":"s1","scan_name":"rust"}"#,
    )
    .await;

    let detail = gw
        .client()
        .scanner_type("s1", "Vec<CodeType>")
        .await
        .expect("type query succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/scanner/scans/s1/types/Vec%3CCodeType%3E");
    assert_eq!(detail.code_type.name, "Vec<CodeType>");
}

#[tokio::test]
async fn agent_runtime_session_stop_posts_the_bare_descriptor() {
    let gw = mock_gateway(r#"{"ok":true}"#).await;

    let session = quantum_sdk::RuntimeSession {
        id: "s1".into(),
        user_id: "u1".into(),
        agent_id: "a1".into(),
        environment_id: "e1".into(),
        backend: "coding-session".into(),
        status: "running".into(),
        upstream_id: "sesn_9".into(),
        created_at: "2026-01-01T00:00:00Z".into(),
    };
    let resp = gw
        .client()
        .agent_runtime_session_stop(&session)
        .await
        .expect("stop succeeds");

    let req = gw.only_request();
    assert_eq!(req.path, "/qai/v1/agent-runtime/sessions/stop");
    // The stop route takes the session itself, not a wrapper object.
    assert_eq!(req.body_json()["upstream_id"], "sesn_9");
    assert!(resp.ok);
}

#[tokio::test]
async fn file_upload_posts_multipart_to_the_files_route() {
    let gw = mock_gateway(
        r#"{"file_uri":"https://generativelanguage.googleapis.com/v1beta/files/abc",
            "name":"files/abc","mime_type":"application/pdf","size_bytes":9,
            "expires_at":"2026-01-03T00:00:00Z"}"#,
    )
    .await;

    let resp = gw
        .client()
        .file_upload("spec.pdf", "application/pdf", b"%PDF-1.7\n".to_vec())
        .await
        .expect("upload succeeds");

    let req = gw.only_request();
    assert_eq!(req.method, "POST");
    assert_eq!(req.path, "/qai/v1/files");
    assert!(
        req.body.contains("filename=\"spec.pdf\""),
        "the file part should carry the filename: {}",
        req.body
    );
    assert_eq!(resp.name, "files/abc");
}
