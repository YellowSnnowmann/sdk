use reqwest::Method;
use serde::{Deserialize, Serialize};

use super::types::DynamicResponse;
use crate::{enc, Error, HttpClient};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
/// What a key may reach, named by feature. `Connections` is machine-only: the
/// backend grants it automatically to a provisioned tenant origin during the
/// `GET /auth/key` PKCE flow and never accepts it from `POST /api-keys`, so
/// it is not a variant of [`CreatableApiKeyScope`], the type
/// [`CreateApiKeyRequest`] actually uses.
pub enum ApiKeyScope {
    Inference,
    Voice,
    Search,
    Media,
    Storage,
    Meetings,
    Account,
    Companies,
    Connections,
}

/// Scopes a caller may request when creating a key through `POST /api-keys`.
/// A subset of [`ApiKeyScope`] that deliberately excludes `Connections`,
/// which that endpoint rejects.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CreatableApiKeyScope {
    Inference,
    Voice,
    Search,
    Media,
    Storage,
    Meetings,
    Account,
    Companies,
}

impl From<CreatableApiKeyScope> for ApiKeyScope {
    fn from(scope: CreatableApiKeyScope) -> Self {
        match scope {
            CreatableApiKeyScope::Inference => Self::Inference,
            CreatableApiKeyScope::Voice => Self::Voice,
            CreatableApiKeyScope::Search => Self::Search,
            CreatableApiKeyScope::Media => Self::Media,
            CreatableApiKeyScope::Storage => Self::Storage,
            CreatableApiKeyScope::Meetings => Self::Meetings,
            CreatableApiKeyScope::Account => Self::Account,
            CreatableApiKeyScope::Companies => Self::Companies,
        }
    }
}

/// Narrow a full [`ApiKeyScope`] (say, one read back from a listed key) to
/// the create-request subset. Fails with [`Error::ScopeNotCreatable`] for
/// `Connections`, so a caller learns why client-side instead of getting a
/// 400 from the backend after the request already went out.
impl TryFrom<ApiKeyScope> for CreatableApiKeyScope {
    type Error = Error;

    fn try_from(scope: ApiKeyScope) -> Result<Self, Error> {
        Ok(match scope {
            ApiKeyScope::Inference => Self::Inference,
            ApiKeyScope::Voice => Self::Voice,
            ApiKeyScope::Search => Self::Search,
            ApiKeyScope::Media => Self::Media,
            ApiKeyScope::Storage => Self::Storage,
            ApiKeyScope::Meetings => Self::Meetings,
            ApiKeyScope::Account => Self::Account,
            ApiKeyScope::Companies => Self::Companies,
            ApiKeyScope::Connections => return Err(Error::ScopeNotCreatable(scope)),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateApiKeyRequest {
    pub name: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub scopes: Vec<CreatableApiKeyScope>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allowed_ips: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

pub struct ApiKeysApi<'a> {
    http: &'a HttpClient,
}

impl<'a> ApiKeysApi<'a> {
    pub fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    pub async fn list(&self) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/api-keys", &[], None, true)
            .await
    }

    pub async fn create(&self, request: &CreateApiKeyRequest) -> Result<DynamicResponse, Error> {
        let body = serde_json::to_value(request).expect("API key request is serializable");
        self.http
            .send_typed(Method::POST, "/api-keys", &[], Some(&body), true)
            .await
    }

    pub async fn revoke(&self, key_id: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/api-keys/{}", enc(key_id));
        self.http
            .send_typed(Method::DELETE, &path, &[], None, true)
            .await
    }
}
