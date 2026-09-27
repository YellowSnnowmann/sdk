//! Hosted agent memory: write experiences, recall, answer questions, list and
//! read events, read the derived layers (facts, beliefs, understanding), upload
//! files, forget, list scopes.
//!
//! Every call runs as the caller's own memory tenant and is billed from the
//! caller's credits. Most bodies are passed through to the memory service, so
//! extra fields beyond `scope` are forwarded unchanged; `answer` is the
//! exception and accepts only its documented fields.

use reqwest::Method;
use serde_json::Value;

use super::types::DynamicResponse;
use crate::{enc, Error, HttpClient, QueryParam};

/// Typed client for the `/memory/*` routes.
pub struct MemoryApi<'a> {
    http: &'a HttpClient,
}

impl<'a> MemoryApi<'a> {
    /// Create a new memory API client.
    pub fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Write an experience (`{scope, content, ...}`) into memory.
    pub async fn write_experience(&self, body: &Value) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::POST, "/memory/experience", &[], Some(body), true)
            .await
    }

    /// Recall from memory (`{scope, query, ...}`).
    pub async fn recall(&self, body: &Value) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::POST, "/memory/recall", &[], Some(body), true)
            .await
    }

    /// List stored events; `scope` is required, other query params pass through.
    pub async fn list_events(&self, query: &[QueryParam]) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/memory/events", query, None, true)
            .await
    }

    /// Read one stored event.
    pub async fn get_event(&self, id: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/memory/events/{}", enc(id));
        self.http
            .send_typed(Method::GET, &path, &[], None, true)
            .await
    }

    /// Forget memories in a scope (`{scope, ...}`).
    pub async fn forget(&self, body: &Value) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::POST, "/memory/forget", &[], Some(body), true)
            .await
    }

    /// List the caller's memory scopes, optionally by prefix.
    pub async fn list_scopes(&self, prefix: Option<&str>) -> Result<DynamicResponse, Error> {
        let query: [QueryParam; 1] = [("prefix", prefix.map(str::to_owned))];
        self.http
            .send_typed(Method::GET, "/memory/scopes", &query, None, true)
            .await
    }

    /// Answer a question from memory (`{scope, question, ...}`) with an LLM.
    /// The model is chosen by the service; `answer_model` and `stream` are refused.
    pub async fn answer(&self, body: &Value) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::POST, "/memory/answer", &[], Some(body), true)
            .await
    }

    /// List the facts derived from a scope; `scope` is required, paging params
    /// (`cursor`, `limit`) pass through.
    pub async fn list_facts(&self, query: &[QueryParam]) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/memory/facts", query, None, true)
            .await
    }

    /// List the beliefs derived from a scope; `scope` is required.
    pub async fn list_beliefs(&self, query: &[QueryParam]) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/memory/beliefs", query, None, true)
            .await
    }

    /// List the understanding derived from a scope; `scope` is required.
    pub async fn list_understanding(
        &self,
        query: &[QueryParam],
    ) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/memory/understanding", query, None, true)
            .await
    }

    /// How far derivation has caught up with the writes in `scope`.
    pub async fn derivation_status(&self, scope: &str) -> Result<DynamicResponse, Error> {
        let query: [QueryParam; 1] = [("scope", Some(scope.to_owned()))];
        self.http
            .send_typed(Method::GET, "/memory/derivation-status", &query, None, true)
            .await
    }

    /// Upload a file (up to 20 MiB) as the raw body under its own MIME type.
    /// Reference the returned `blob_id` from an experience whose `content` is
    /// `{"kind": "blob_ref", "blob_id": ...}` to have it extracted into memory.
    pub async fn upload_blob(
        &self,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<DynamicResponse, Error> {
        let value = self
            .http
            .send_raw_body(Method::POST, "/memory/blobs", bytes, content_type)
            .await?;
        Ok(serde_json::from_value(value)?)
    }

    /// Download an uploaded file: its bytes and the Content-Type it was stored under.
    pub async fn get_blob(&self, id: &str) -> Result<(Vec<u8>, Option<String>), Error> {
        let path = format!("/memory/blobs/{}", enc(id));
        self.http
            .send_bytes_query_with_content_type(Method::GET, &path, &[])
            .await
    }

    /// Delete an uploaded file.
    pub async fn delete_blob(&self, id: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/memory/blobs/{}", enc(id));
        self.http
            .send_typed(Method::DELETE, &path, &[], None, true)
            .await
    }
}
