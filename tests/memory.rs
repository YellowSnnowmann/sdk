use serde_json::json;
use tinyhumans_sdk::TinyHumansClient;
use wiremock::matchers::{body_json, method, path, query_param, query_param_is_missing};
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
