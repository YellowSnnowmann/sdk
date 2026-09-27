use serde_json::json;
use tinyhumans_sdk::TinyHumansClient;
use wiremock::matchers::{
    body_bytes, body_json, header, method, path, query_param, query_param_is_missing,
};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ok(data: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({"success": true, "data": data}))
}

#[tokio::test]
async fn write_experience_posts_body() {
    let server = MockServer::start().await;
    let body = json!({"scope": "notes", "content": {"text": "hi"}});
    Mock::given(method("POST"))
        .and(path("/memory/experience"))
        .and(body_json(body.clone()))
        .respond_with(ok(json!({"id": "evt_1"})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().write_experience(&body).await.unwrap();
    assert_eq!(*result, json!({"id": "evt_1"}));
}

#[tokio::test]
async fn recall_posts_body() {
    let server = MockServer::start().await;
    let body = json!({"scope": "notes", "query": "what"});
    Mock::given(method("POST"))
        .and(path("/memory/recall"))
        .and(body_json(body.clone()))
        .respond_with(ok(json!({"hits": []})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().recall(&body).await.unwrap();
    assert_eq!(*result, json!({"hits": []}));
}

#[tokio::test]
async fn list_events_sends_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/memory/events"))
        .and(query_param("scope", "notes"))
        .and(query_param("limit", "5"))
        .respond_with(ok(json!({"events": []})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client
        .memory()
        .list_events(&[
            ("scope", Some("notes".to_string())),
            ("limit", Some("5".to_string())),
        ])
        .await
        .unwrap();
    assert_eq!(*result, json!({"events": []}));
}

#[tokio::test]
async fn get_event_gets_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/memory/events/evt_1"))
        .respond_with(ok(json!({"id": "evt_1"})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().get_event("evt_1").await.unwrap();
    assert_eq!(*result, json!({"id": "evt_1"}));
}

#[tokio::test]
async fn forget_posts_body() {
    let server = MockServer::start().await;
    let body = json!({"scope": "notes"});
    Mock::given(method("POST"))
        .and(path("/memory/forget"))
        .and(body_json(body.clone()))
        .respond_with(ok(json!({"forgotten": 2})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().forget(&body).await.unwrap();
    assert_eq!(*result, json!({"forgotten": 2}));
}

#[tokio::test]
async fn list_scopes_sends_prefix() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/memory/scopes"))
        .and(query_param("prefix", "proj"))
        .respond_with(ok(json!({"scopes": ["proj/a"]})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().list_scopes(Some("proj")).await.unwrap();
    assert_eq!(*result, json!({"scopes": ["proj/a"]}));
}

#[tokio::test]
async fn list_scopes_without_prefix() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/memory/scopes"))
        .and(query_param_is_missing("prefix"))
        .respond_with(ok(json!({"scopes": []})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().list_scopes(None).await.unwrap();
    assert_eq!(*result, json!({"scopes": []}));
}

#[tokio::test]
async fn answer_posts_body() {
    let server = MockServer::start().await;
    let body = json!({"scope": "notes", "question": "what colour?", "cite_sources": true});
    Mock::given(method("POST"))
        .and(path("/memory/answer"))
        .and(body_json(body.clone()))
        .respond_with(ok(json!({"answer": "blue", "citations": []})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().answer(&body).await.unwrap();
    assert_eq!(*result, json!({"answer": "blue", "citations": []}));
}

#[tokio::test]
async fn derived_layers_send_scope_and_paging() {
    let server = MockServer::start().await;
    for route in ["/memory/facts", "/memory/beliefs", "/memory/understanding"] {
        Mock::given(method("GET"))
            .and(path(route))
            .and(query_param("scope", "notes"))
            .and(query_param("cursor", "c1"))
            .respond_with(ok(json!({"items": [], "has_more": false})))
            .mount(&server)
            .await;
    }

    let client = TinyHumansClient::new(server.uri());
    let query = [
        ("scope", Some("notes".to_string())),
        ("cursor", Some("c1".to_string())),
    ];
    let memory = client.memory();
    for result in [
        memory.list_facts(&query).await.unwrap(),
        memory.list_beliefs(&query).await.unwrap(),
        memory.list_understanding(&query).await.unwrap(),
    ] {
        assert_eq!(*result, json!({"items": [], "has_more": false}));
    }
}

#[tokio::test]
async fn derivation_status_sends_scope() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/memory/derivation-status"))
        .and(query_param("scope", "notes"))
        .respond_with(ok(json!({"scope": "notes", "caught_up": true})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().derivation_status("notes").await.unwrap();
    assert_eq!(*result, json!({"scope": "notes", "caught_up": true}));
}

#[tokio::test]
async fn upload_blob_sends_raw_bytes_with_their_type() {
    let server = MockServer::start().await;
    let bytes = vec![0x89, 0x50, 0x4e, 0x47, 0x00, 0xff];
    Mock::given(method("POST"))
        .and(path("/memory/blobs"))
        .and(header("content-type", "image/png"))
        .and(body_bytes(bytes.clone()))
        .respond_with(ok(json!({"blob_id": "blob_abc", "size_bytes": 6})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client
        .memory()
        .upload_blob(bytes, "image/png")
        .await
        .unwrap();
    assert_eq!(*result, json!({"blob_id": "blob_abc", "size_bytes": 6}));
}

#[tokio::test]
async fn get_blob_returns_bytes_and_type() {
    let server = MockServer::start().await;
    let bytes = vec![0x25, 0x50, 0x44, 0x46, 0x00];
    Mock::given(method("GET"))
        .and(path("/memory/blobs/blob_abc"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/pdf")
                .set_body_bytes(bytes.clone()),
        )
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let (body, content_type) = client.memory().get_blob("blob_abc").await.unwrap();
    assert_eq!(body, bytes);
    assert_eq!(content_type.as_deref(), Some("application/pdf"));
}

#[tokio::test]
async fn delete_blob_deletes_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/memory/blobs/blob_abc"))
        .respond_with(ok(json!({"deleted": true})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.memory().delete_blob("blob_abc").await.unwrap();
    assert_eq!(*result, json!({"deleted": true}));
}
