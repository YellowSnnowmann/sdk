use serde_json::json;
use tinyhumans_sdk::TinyHumansClient;
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn exposes_user_team_routes() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/teams"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"success":true,"data":[]})))
        .mount(&server)
        .await;
    assert_eq!(
        TinyHumansClient::new(server.uri())
            .teams()
            .list_teams()
            .await
            .unwrap(),
        json!([])
    );
}

#[tokio::test]
async fn get_my_usage_for_range_sends_range_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/teams/me/usage"))
        .and(query_param("range", "30d"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {"range": "30d"}
        })))
        .mount(&server)
        .await;

    assert_eq!(
        TinyHumansClient::new(server.uri())
            .teams()
            .get_my_usage_for_range("30d")
            .await
            .unwrap(),
        json!({"range": "30d"})
    );
}
