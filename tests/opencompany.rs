use serde_json::json;
use tinyhumans_sdk::TinyHumansClient;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn ok(data: serde_json::Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(json!({"success": true, "data": data}))
}

#[tokio::test]
async fn usage_gets_instance_usage_summary() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/opencompany/instances/usage"))
        .respond_with(ok(json!({"instances": []})))
        .mount(&server)
        .await;

    let client = TinyHumansClient::new(server.uri());
    let result = client.opencompany().usage().await.unwrap();
    assert_eq!(result, json!({"instances": []}));
}
