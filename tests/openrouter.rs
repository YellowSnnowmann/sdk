//! Direct OpenRouter proxy: catalogs, the four request shapes, and the async
//! video flow.
//!
//! The distinction these pin is which routes carry the `{ success, data }`
//! envelope and which return the upstream provider's payload verbatim. Chat,
//! completions, messages and embeddings are passthroughs — unwrapping them
//! would strip the caller's actual response — while the catalogs and the media
//! routes are enveloped like every other agent integration.

use serde_json::json;
use tinyhumans_sdk::api::agent_integrations::{
    ContentPartImage, FrameImage, OpenRouterImageRequest, OpenRouterImageResponse,
    OpenRouterMediaModelsResponse, OpenRouterModelsResponse, OpenRouterVideoJob,
    OpenRouterVideoRequest,
};
use tinyhumans_sdk::TinyHumansClient;
use wiremock::matchers::{body_json, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn list_models_is_typed_and_paginated() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/models"))
        .and(query_param("author", "anthropic"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "object": "list",
                "data": [{
                    "id": "anthropic/claude-sonnet-4.5",
                    "display_name": "Claude Sonnet 4.5",
                    "context_length": 1000000,
                    "input_modalities": ["text", "image"],
                    "supports_tools": true,
                    "supports_thinking": true,
                    "pricing": {"input_per_1m": 3.0, "output_per_1m": 15.0, "cached_input_per_1m": 0.3}
                }],
                "total": 1, "limit": 100, "offset": 0
            }
        })))
        .mount(&server)
        .await;

    let response: OpenRouterModelsResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .list_openrouter_models(&[("author", Some("anthropic".into()))])
        .await
        .unwrap();

    assert_eq!(response.total, 1);
    // The id is the bare slug, which is what callers send back as `model`.
    assert_eq!(response.data[0].id, "anthropic/claude-sonnet-4.5");
    assert_eq!(response.data[0].pricing.cached_input_per_1m, Some(0.3));
    assert!(response.data[0].supports_tools);
}

#[tokio::test]
async fn chat_completion_returns_the_upstream_payload_unwrapped() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/chat/completions"))
        .and(body_json(json!({
            "model": "anthropic/claude-sonnet-4.5",
            "messages": [{"role": "user", "content": "hi"}]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "gen-1",
            "choices": [{"message": {"role": "assistant", "content": "hello"}}],
            "usage": {"prompt_tokens": 1, "completion_tokens": 1, "total_tokens": 2}
        })))
        .mount(&server)
        .await;

    let response = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_chat_completion(&json!({
            "model": "anthropic/claude-sonnet-4.5",
            "messages": [{"role": "user", "content": "hi"}]
        }))
        .await
        .unwrap();

    // A passthrough route has no envelope to unwrap; the OpenAI-shaped body is
    // handed back as-is.
    assert_eq!(response["id"], "gen-1");
    assert_eq!(response["choices"][0]["message"]["content"], "hello");
}

#[tokio::test]
async fn arbitrary_openrouter_parameters_are_forwarded() {
    let server = MockServer::start().await;
    // The request type is `impl Serialize` precisely so provider-specific knobs
    // reach upstream instead of being dropped by a closed struct.
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/chat/completions"))
        .and(body_json(json!({
            "model": "anthropic/claude-sonnet-4.5",
            "messages": [],
            "reasoning": {"effort": "high"},
            "transforms": ["middle-out"],
            "provider": {"order": ["anthropic"]}
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "gen-2"})))
        .mount(&server)
        .await;

    let response = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_chat_completion(&json!({
            "model": "anthropic/claude-sonnet-4.5",
            "messages": [],
            "reasoning": {"effort": "high"},
            "transforms": ["middle-out"],
            "provider": {"order": ["anthropic"]}
        }))
        .await
        .unwrap();
    assert_eq!(response["id"], "gen-2");
}

#[tokio::test]
async fn streaming_is_rejected_before_it_reaches_the_wire() {
    // `HttpClient::send` (the transport `passthrough` uses) awaits
    // `response.text()` and buffers the whole body, so a `stream: true`
    // request would come back as one opaque non-JSON string instead of
    // incremental SSE events. No mock is registered for any of these routes,
    // so a `Status`/`Http` error (rather than `StreamingNotSupported`) would
    // mean the guard let the request reach the wire.
    let server = MockServer::start().await;
    let client = TinyHumansClient::new(server.uri());

    let chat = client
        .agent_integrations()
        .openrouter_chat_completion(
            &json!({"model": "anthropic/claude-sonnet-4.5", "stream": true}),
        )
        .await
        .unwrap_err();
    assert!(
        matches!(chat, tinyhumans_sdk::Error::StreamingNotSupported(ref path) if path == "/agent-integrations/openrouter/chat/completions"),
        "unexpected error: {chat:?}"
    );

    let completion = client
        .agent_integrations()
        .openrouter_completion(&json!({"model": "anthropic/claude-sonnet-4.5", "stream": true}))
        .await
        .unwrap_err();
    assert!(matches!(
        completion,
        tinyhumans_sdk::Error::StreamingNotSupported(_)
    ));

    let message = client
        .agent_integrations()
        .openrouter_message(&json!({"model": "anthropic/claude-sonnet-4.5", "stream": true}))
        .await
        .unwrap_err();
    assert!(matches!(
        message,
        tinyhumans_sdk::Error::StreamingNotSupported(_)
    ));

    // `stream: false` (and omitting it) is unaffected — still reaches the wire.
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/chat/completions"))
        .and(body_json(
            json!({"model": "anthropic/claude-sonnet-4.5", "stream": false}),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"id": "gen-3"})))
        .mount(&server)
        .await;
    let response = client
        .agent_integrations()
        .openrouter_chat_completion(
            &json!({"model": "anthropic/claude-sonnet-4.5", "stream": false}),
        )
        .await
        .unwrap();
    assert_eq!(response["id"], "gen-3");
}

#[tokio::test]
async fn messages_speaks_the_anthropic_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/messages"))
        .and(body_json(json!({
            "model": "anthropic/claude-sonnet-4.5",
            "system": "be terse",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 64
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "msg_1",
            "type": "message",
            "role": "assistant",
            "content": [{"type": "text", "text": "hello"}],
            "usage": {"input_tokens": 12, "output_tokens": 15}
        })))
        .mount(&server)
        .await;

    let response = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_message(&json!({
            "model": "anthropic/claude-sonnet-4.5",
            "system": "be terse",
            "messages": [{"role": "user", "content": "hi"}],
            "max_tokens": 64
        }))
        .await
        .unwrap();

    // Anthropic's envelope, not OpenAI's: content blocks and input/output tokens.
    assert_eq!(response["type"], "message");
    assert_eq!(response["content"][0]["text"], "hello");
    assert_eq!(response["usage"]["input_tokens"], 12);
}

#[tokio::test]
async fn system_one_preserves_the_typesafe_wire_shape() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/systemone"))
        .and(body_json(json!({
            "model": "jev-latest",
            "state": {"ticket": "I was charged twice"},
            "questions": {
                "refund": {"type": "noul", "instructions": "Is a refund requested?"}
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "id": "gen-dec-1",
            "model": "typesafe/jev-1.13-20260917",
            "provider": "TypeSafe",
            "answers": {"refund": {"type": "noul", "noul": 0.98}},
            "usage": {"input_tokens": 275, "output_tokens": 20, "cost": 0.00003}
        })))
        .mount(&server)
        .await;

    let response = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_system_one(&json!({
            "model": "jev-latest",
            "state": {"ticket": "I was charged twice"},
            "questions": {
                "refund": {"type": "noul", "instructions": "Is a refund requested?"}
            }
        }))
        .await
        .unwrap();

    assert_eq!(response["answers"]["refund"]["type"], "noul");
    assert_eq!(response["answers"]["refund"]["noul"], 0.98);
    assert_eq!(response["usage"]["cost"], 0.00003);
}

#[tokio::test]
async fn embedding_models_come_from_their_own_catalog() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/embeddings/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "object": "list",
                "data": [{
                    "id": "openai/text-embedding-3-small",
                    "display_name": "Text Embedding 3 Small",
                    "pricing": {"input_per_1m": 0.02, "output_per_1m": 0.0}
                }],
                "total": 1, "limit": 100, "offset": 0
            }
        })))
        .mount(&server)
        .await;

    let response: OpenRouterModelsResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .list_openrouter_embedding_models(&[])
        .await
        .unwrap();
    assert_eq!(response.data[0].id, "openai/text-embedding-3-small");
    assert_eq!(response.data[0].pricing.output_per_1m, 0.0);
}

#[tokio::test]
async fn embeddings_returns_the_upstream_payload() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/embeddings"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "object": "list",
            "data": [{"object": "embedding", "index": 0, "embedding": [0.1, 0.2]}],
            "usage": {"prompt_tokens": 8, "total_tokens": 8}
        })))
        .mount(&server)
        .await;

    let response = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_embeddings(&json!({
            "model": "openai/text-embedding-3-small",
            "input": "hello"
        }))
        .await
        .unwrap();
    assert_eq!(response["data"][0]["index"], 0);
    assert_eq!(response["usage"]["prompt_tokens"], 8);
}

#[tokio::test]
async fn image_models_carry_no_generation_price() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/images/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "object": "list",
                "data": [{"id": "bytedance-seed/seedream-4.5", "display_name": "Seedream 4.5"}],
                "total": 1, "limit": 100, "offset": 0
            }
        })))
        .mount(&server)
        .await;

    let response: OpenRouterMediaModelsResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .list_openrouter_image_models(&[])
        .await
        .unwrap();
    // Images bill at the cost the generation response reports, so the catalog
    // publishes no price to quote up front.
    assert_eq!(response.data[0].price_per_generation, None);
}

#[tokio::test]
async fn video_models_publish_a_flat_generation_price() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/videos/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "object": "list",
                "data": [{
                    "id": "google/veo-3.1",
                    "display_name": "Veo 3.1",
                    "price_per_generation": 0.5
                }],
                "total": 1, "limit": 100, "offset": 0
            }
        })))
        .mount(&server)
        .await;

    let response: OpenRouterMediaModelsResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .list_openrouter_video_models(&[])
        .await
        .unwrap();
    assert_eq!(response.data[0].price_per_generation, Some(0.5));
}

#[tokio::test]
async fn image_generation_unwraps_the_envelope() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/images"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "created": 1,
                "data": [{"b64_json": "aGk="}],
                "usage": {"cost": 0.04}
            }
        })))
        .mount(&server)
        .await;

    let response = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_create_image(&json!({
            "model": "bytedance-seed/seedream-4.5",
            "prompt": "a red panda"
        }))
        .await
        .unwrap();
    assert_eq!(response["data"][0]["b64_json"], "aGk=");
}

#[tokio::test]
async fn the_video_flow_submits_polls_and_downloads() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/videos"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {"id": "job-abc", "status": "pending", "polling_url": "/api/v1/videos/job-abc"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/videos/job-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {"id": "job-abc", "status": "completed", "generation_id": "gen-xyz"}
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/agent-integrations/openrouter/videos/job-abc/content",
        ))
        .and(query_param("index", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes([0_u8, 1, 2, 255]))
        .mount(&server)
        .await;

    let api = TinyHumansClient::new(server.uri());

    let job: OpenRouterVideoJob = api
        .agent_integrations()
        .openrouter_create_video(&json!({"model": "google/veo-3.1", "prompt": "a mountain"}))
        .await
        .unwrap();
    assert_eq!(job.id, "job-abc");
    assert_eq!(job.status, "pending");

    let polled = api
        .agent_integrations()
        .get_openrouter_video("job-abc")
        .await
        .unwrap();
    assert_eq!(polled.status, "completed");
    assert_eq!(polled.generation_id.as_deref(), Some("gen-xyz"));

    let bytes = api
        .agent_integrations()
        .openrouter_video_content("job-abc", Some(1))
        .await
        .unwrap();
    assert_eq!(bytes, vec![0, 1, 2, 255]);
}

#[tokio::test]
async fn video_content_omits_the_index_when_unset() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/agent-integrations/openrouter/videos/job-abc/content",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_bytes([7_u8]))
        .mount(&server)
        .await;

    let bytes = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_video_content("job-abc", None)
        .await
        .unwrap();
    assert_eq!(bytes, vec![7]);
}

// --- Typed media DTOs (openrouter_media) ---

#[tokio::test]
async fn typed_image_request_forwards_every_field_including_input_references() {
    let server = MockServer::start().await;
    let expected_body = json!({
        "model": "bytedance-seed/seedream-4.5",
        "prompt": "a red panda astronaut",
        "n": 2,
        "aspect_ratio": "16:9",
        "resolution": "2K",
        "seed": 42,
        "input_references": [
            {"type": "image_url", "image_url": {"url": "https://example.com/ref.png"}}
        ]
    });
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/images"))
        .and(body_json(expected_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "created": 1,
                "data": [{"b64_json": "aGk=", "media_type": "image/png"}],
                "usage": {"cost": 0.04, "prompt_tokens": 0, "completion_tokens": 10, "total_tokens": 10}
            }
        })))
        .mount(&server)
        .await;

    let mut request =
        OpenRouterImageRequest::new("bytedance-seed/seedream-4.5", "a red panda astronaut");
    request.n = Some(2);
    request.aspect_ratio = Some("16:9".into());
    request.resolution = Some("2K".into());
    request.seed = Some(42);
    request.input_references = vec![ContentPartImage::image_url("https://example.com/ref.png")];

    let response: OpenRouterImageResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_images(&request)
        .await
        .unwrap();
    assert_eq!(response.data[0].b64_json, "aGk=");
    assert_eq!(response.data[0].media_type.as_deref(), Some("image/png"));
    assert_eq!(response.usage.cost, Some(0.04));
}

#[tokio::test]
async fn typed_video_request_forwards_frame_images_and_input_references() {
    let server = MockServer::start().await;
    let expected_body = json!({
        "model": "google/veo-3.1",
        "prompt": "a mountain",
        "duration": 8,
        "resolution": "720p",
        "aspect_ratio": "16:9",
        "generate_audio": true,
        "seed": 7,
        "frame_images": [
            {"type": "image_url", "image_url": {"url": "https://example.com/first.png"}, "frame_type": "first_frame"},
            {"type": "image_url", "image_url": {"url": "https://example.com/last.png"}, "frame_type": "last_frame"}
        ],
        "input_references": [
            {"type": "image_url", "image_url": {"url": "https://example.com/ref.png"}}
        ]
    });
    Mock::given(method("POST"))
        .and(path("/agent-integrations/openrouter/videos"))
        .and(body_json(expected_body))
        .respond_with(ResponseTemplate::new(202).set_body_json(json!({
            "success": true,
            "data": {"id": "job-xyz", "status": "pending", "polling_url": "/api/v1/videos/job-xyz"}
        })))
        .mount(&server)
        .await;

    let mut request = OpenRouterVideoRequest::new("google/veo-3.1");
    request.prompt = Some("a mountain".into());
    request.duration = Some(8);
    request.resolution = Some("720p".into());
    request.aspect_ratio = Some("16:9".into());
    request.generate_audio = Some(true);
    request.seed = Some(7);
    request.frame_images = vec![
        FrameImage::first_frame("https://example.com/first.png"),
        FrameImage::last_frame("https://example.com/last.png"),
    ];
    request.input_references = vec![ContentPartImage::image_url("https://example.com/ref.png")];

    let job: OpenRouterVideoJob = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_videos(&request)
        .await
        .unwrap();
    assert_eq!(job.id, "job-xyz");
    assert_eq!(job.status, "pending");
}

#[tokio::test]
async fn typed_media_requests_reject_streaming_before_the_wire() {
    // `openrouter_images`/`openrouter_videos` buffer the whole response body
    // and deserialize it as JSON (`OpenRouterImageResponse`/`OpenRouterVideoJob`),
    // so a `stream: true` request would come back as an SSE event stream and
    // fail to decode instead of erroring clearly. No mock is registered for
    // either route, so a transport/decode error (rather than
    // `StreamingNotSupported`) would mean the guard let the request through.
    let server = MockServer::start().await;
    let client = TinyHumansClient::new(server.uri());

    let mut image_request = OpenRouterImageRequest::new("bytedance-seed/seedream-4.5", "a cat");
    image_request.stream = Some(true);
    let image_err = client
        .agent_integrations()
        .openrouter_images(&image_request)
        .await
        .unwrap_err();
    assert!(
        matches!(image_err, tinyhumans_sdk::Error::StreamingNotSupported(ref path) if path == "/agent-integrations/openrouter/images"),
        "unexpected error: {image_err:?}"
    );

    let mut video_request = OpenRouterVideoRequest::new("google/veo-3.1");
    video_request
        .extra
        .insert("stream".to_owned(), serde_json::Value::Bool(true));
    let video_err = client
        .agent_integrations()
        .openrouter_videos(&video_request)
        .await
        .unwrap_err();
    assert!(
        matches!(video_err, tinyhumans_sdk::Error::StreamingNotSupported(ref path) if path == "/agent-integrations/openrouter/videos"),
        "unexpected error: {video_err:?}"
    );
}

#[tokio::test]
async fn typed_image_models_carry_capability_descriptors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/images/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "object": "list",
                "data": [{
                    "id": "bytedance-seed/seedream-4.5",
                    "display_name": "Seedream 4.5",
                    "architecture": {"input_modalities": ["text", "image"], "output_modalities": ["image"]},
                    "supported_parameters": {"resolution": {"type": "enum", "values": ["1K", "2K", "4K"]}}
                }],
                "total": 1, "limit": 100, "offset": 0
            }
        })))
        .mount(&server)
        .await;

    let response: OpenRouterMediaModelsResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_image_models(&[])
        .await
        .unwrap();
    let arch = response.data[0].architecture.as_ref().unwrap();
    assert_eq!(arch.output_modalities, vec!["image".to_string()]);
    assert!(response.data[0].supported_parameters.is_some());
}

#[tokio::test]
async fn typed_video_models_carry_capability_descriptors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/videos/models"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "object": "list",
                "data": [{
                    "id": "google/veo-3.1",
                    "display_name": "Veo 3.1",
                    "price_per_generation": 0.5,
                    "supported_resolutions": ["720p"],
                    "supported_aspect_ratios": ["16:9"],
                    "supported_durations": [5, 8],
                    "supported_frame_images": ["first_frame", "last_frame"],
                    "generate_audio": true,
                    "allowed_passthrough_parameters": ["google-vertex.output_config"]
                }],
                "total": 1, "limit": 100, "offset": 0
            }
        })))
        .mount(&server)
        .await;

    let response: OpenRouterMediaModelsResponse = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_video_models(&[])
        .await
        .unwrap();
    let model = &response.data[0];
    assert_eq!(
        model.supported_resolutions.as_deref(),
        Some(&["720p".to_string()][..])
    );
    assert_eq!(model.supported_durations.as_deref(), Some(&[5, 8][..]));
    assert_eq!(model.generate_audio, Some(true));
    assert_eq!(model.supported_sizes, None);
}

#[tokio::test]
async fn polled_video_job_carries_unsigned_urls_and_usage() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/videos/job-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true,
            "data": {
                "id": "job-abc",
                "status": "completed",
                "unsigned_urls": ["https://storage.example.com/video.mp4"],
                "usage": {"cost": 0.5}
            }
        })))
        .mount(&server)
        .await;

    let job = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .get_openrouter_video("job-abc")
        .await
        .unwrap();
    assert_eq!(
        job.unsigned_urls,
        vec!["https://storage.example.com/video.mp4".to_string()]
    );
    assert_eq!(job.usage.unwrap().cost, Some(0.5));
}

#[tokio::test]
async fn video_content_with_type_surfaces_the_upstream_content_type() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/agent-integrations/openrouter/videos/job-abc/content",
        ))
        .and(query_param("index", "1"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_bytes([0_u8, 1, 2, 255])
                .insert_header("content-type", "video/mp4"),
        )
        .mount(&server)
        .await;

    let content = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .openrouter_video_content_with_type("job-abc", Some(1))
        .await
        .unwrap();
    assert_eq!(content.bytes, vec![0, 1, 2, 255]);
    assert_eq!(content.content_type.as_deref(), Some("video/mp4"));
}

#[tokio::test]
async fn video_job_ids_are_path_encoded() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/agent-integrations/openrouter/videos/job%2Fabc"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "success": true, "data": {"id": "job/abc", "status": "pending"}
        })))
        .mount(&server)
        .await;

    let job = TinyHumansClient::new(server.uri())
        .agent_integrations()
        .get_openrouter_video("job/abc")
        .await
        .unwrap();
    assert_eq!(job.id, "job/abc");
}
