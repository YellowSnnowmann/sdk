//! Auth and account state: magic-link login, OAuth, current user, and tokens.

use reqwest::Method;

use super::types::{
    DynamicResponse, EmailLinkRequest, IntegrationTokenRequest, LoginTokenRequest,
    RedeemKeyGrantRequest,
};
use crate::{enc, Error, HttpClient, QueryParam};

/// Typed client for the `/auth/*` routes.
pub struct AuthApi<'a> {
    http: &'a HttpClient,
}

impl<'a> AuthApi<'a> {
    pub fn new(http: &'a HttpClient) -> Self {
        Self { http }
    }

    /// Create a short-lived token to link a Telegram or Discord account.
    pub async fn create_channel_link_token(&self, channel: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/channels/{}/link-token", enc(channel));
        self.http
            .send_typed(Method::POST, &path, &[], None, true)
            .await
    }

    /// Send a magic link for email login.
    pub async fn send_email_link(
        &self,
        request: &EmailLinkRequest,
    ) -> Result<DynamicResponse, Error> {
        let body = serde_json::to_value(request).expect("email link request is serializable");
        self.http
            .send_typed(
                Method::POST,
                "/auth/email/send-link",
                &[],
                Some(&body),
                true,
            )
            .await
    }

    /// Verify a magic link and complete email login (302 redirect).
    pub async fn verify_email(&self, token: &str) -> Result<DynamicResponse, Error> {
        let query: [QueryParam; 1] = [("token", Some(token.to_string()))];
        self.http
            .send_typed(Method::GET, "/auth/email/verify", &query, None, true)
            .await
    }

    /// Consume a one-time login token, exchanging it for a session.
    pub async fn consume_login_token(
        &self,
        request: &LoginTokenRequest,
    ) -> Result<DynamicResponse, Error> {
        let body = serde_json::to_value(request).expect("login token request is serializable");
        self.http
            .send_typed(
                Method::POST,
                "/auth/login-token/consume",
                &[],
                Some(&body),
                true,
            )
            .await
    }

    /// Start a PKCE key grant (browser flow). Redirects to provider sign-in;
    /// on approval, posts a one-time `code` to `callback_url` (`mode=code`,
    /// the default) or ends on this backend's own dashboard (`mode=manual`).
    /// Redeem the resulting code with [`Self::redeem_key_grant`] (`mode=code`)
    /// or [`Self::describe_key_grant`]/[`Self::issue_key_grant`]
    /// (`mode=manual`).
    pub async fn start_key_grant(
        &self,
        callback_url: &str,
        query: &[QueryParam],
    ) -> Result<DynamicResponse, Error> {
        let mut full_query = vec![("callback_url", Some(callback_url.to_string()))];
        full_query.extend_from_slice(query);
        self.http
            .send_typed(Method::GET, "/auth/key", &full_query, None, true)
            .await
    }

    /// Start a PKCE key grant with explicitly requested API-key scopes.
    ///
    /// `connections` is granted to a loopback callback only when it is named
    /// here. Provisioned tenant origins receive it regardless of this list.
    pub async fn start_key_grant_with_scopes(
        &self,
        callback_url: &str,
        scopes: &[&str],
        query: &[QueryParam],
    ) -> Result<DynamicResponse, Error> {
        let mut full_query = query.to_vec();
        if !scopes.is_empty() {
            full_query.push(("scopes", Some(scopes.join(","))));
        }
        self.start_key_grant(callback_url, &full_query).await
    }

    /// Redeem a PKCE key grant for an API key (`mode=code` flow).
    /// Unauthenticated: possession of the `code_verifier` is the proof.
    /// Returns the plaintext key exactly once, in `data.key`.
    pub async fn redeem_key_grant(
        &self,
        request: &RedeemKeyGrantRequest,
    ) -> Result<DynamicResponse, Error> {
        let body = serde_json::to_value(request).expect("key grant request is serializable");
        self.http
            .send_typed(Method::POST, "/auth/keys", &[], Some(&body), true)
            .await
    }

    /// Describe a pending manual key grant (`mode=manual` flow): the origin
    /// that asked, the label the key will carry, and its scopes. Does not
    /// spend the grant.
    pub async fn describe_key_grant(&self, code: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/key/grant/{}", enc(code));
        self.http
            .send_typed(Method::GET, &path, &[], None, true)
            .await
    }

    /// Spend a pending manual key grant and mint its key, returning the
    /// plaintext once in `data.key`.
    pub async fn issue_key_grant(&self, code: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/key/grant/{}/issue", enc(code));
        self.http
            .send_typed(Method::POST, &path, &[], None, true)
            .await
    }

    /// Get the currently authenticated user.
    pub async fn me(&self) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/auth/me", &[], None, true)
            .await
    }

    /// List the user's connected third-party integrations.
    pub async fn list_integrations(&self) -> Result<DynamicResponse, Error> {
        self.http
            .send_typed(Method::GET, "/auth/integrations", &[], None, true)
            .await
    }

    /// Disconnect a third-party integration.
    pub async fn delete_integration(&self, integration_id: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/integrations/{}", enc(integration_id));
        self.http
            .send_typed(Method::DELETE, &path, &[], None, true)
            .await
    }

    /// Issue an access token for a connected integration.
    pub async fn create_integration_token(
        &self,
        integration_id: &str,
        request: &IntegrationTokenRequest,
    ) -> Result<DynamicResponse, Error> {
        let body =
            serde_json::to_value(request).expect("integration token request is serializable");
        let path = format!("/auth/integrations/{}/tokens", enc(integration_id));
        self.http
            .send_typed(Method::POST, &path, &[], Some(&body), true)
            .await
    }

    /// OAuth provider callback endpoint (returns a redirect).
    pub async fn oauth_callback(&self, provider: &str) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/{}/callback", enc(provider));
        self.http
            .send_typed(Method::GET, &path, &[], None, true)
            .await
    }

    /// Start an OAuth connect flow for a provider.
    pub async fn oauth_connect(
        &self,
        provider: &str,
        query: &[QueryParam],
    ) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/{}/connect", enc(provider));
        self.http
            .send_typed(Method::GET, &path, query, None, true)
            .await
    }

    /// Start an OAuth login flow for a provider (returns a redirect).
    pub async fn oauth_login(
        &self,
        provider: &str,
        query: &[QueryParam],
    ) -> Result<DynamicResponse, Error> {
        let path = format!("/auth/{}/login", enc(provider));
        self.http
            .send_typed(Method::GET, &path, query, None, true)
            .await
    }
}
