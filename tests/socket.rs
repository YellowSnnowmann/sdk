#![cfg(feature = "socket")]

use serde::Deserialize;
use serde_json::json;
use tinyhumans_sdk::socket::{SocketEvent, SocketPayload};
use tinyhumans_sdk::{Error, TinyHumansClient};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Joined {
    channel_id: String,
}

#[test]
fn socket_event_decodes_typed_json_payloads() {
    let event = SocketEvent::new(
        "bot:joined",
        SocketPayload::Json(vec![json!({ "channelId": "channel-1" })]),
    );

    let joined: Joined = event.decode().unwrap();
    assert_eq!(joined.channel_id, "channel-1");
}

#[test]
fn binary_socket_payload_is_not_misdecoded_as_json() {
    let event = SocketEvent::new("agent:audio:chunk", SocketPayload::Binary(vec![1, 2, 3]));
    let error = event.decode::<Joined>().unwrap_err();
    assert!(matches!(error, Error::UnexpectedSocketPayload(_)));
}

#[tokio::test]
async fn socket_connection_requires_a_bearer_token() {
    let error = TinyHumansClient::new("https://api.tinyhumans.ai")
        .connect_socket()
        .await
        .unwrap_err();
    assert!(matches!(error, Error::MissingSocketToken));
}

#[test]
fn unknown_socket_events_remain_available_to_generic_consumers() {
    let event = SocketEvent::new("bot:joined", SocketPayload::Json(vec![json!({"ok": true})]));
    assert_eq!(event.name, "bot:joined");
    assert!(matches!(event.payload, SocketPayload::Json(_)));
}
