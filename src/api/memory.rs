//! Hosted agent memory: write experiences, recall, list and read events, forget, list scopes.
//!
//! Every call runs as the caller's own memory tenant and is billed from the
//! caller's credits. Bodies are passed through to the memory service, so extra
//! fields beyond `scope` are forwarded unchanged.

use reqwest::Method;
use serde_json::Value;

use super::types::DynamicResponse;
use crate::{enc, Error, HttpClient, QueryParam};

/// Typed client for the `/memory/*` routes.
pub struct MemoryApi<'a> {
    http: &'a HttpClient,
}

impl<'a> MemoryApi<'a> {
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
}
