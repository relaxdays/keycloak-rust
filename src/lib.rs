pub mod auth;
pub mod error;
pub mod rest;

use self::auth::AuthenticationProvider;
use std::time::Duration;
use tokio::sync::RwLock;

pub type Error = self::error::KeycloakError;
pub use self::error::ErrorKind;

#[derive(Debug, Clone)]
pub struct KeycloakConfig {
    pub base_url: String,
    pub realm: String,
}

/// high-level keycloak api client
pub struct Keycloak<A: AuthenticationProvider> {
    config: KeycloakConfig,
    /// low-level api client
    ///
    /// this is an rwlock to make sure we can change the inner reqwest client and add default headers for access tokens
    api_client: RwLock<self::rest::Client>,
    auth: RwLock<A>,
}

impl<A: AuthenticationProvider> Keycloak<A> {
    pub async fn new(base_url: &str, realm: &str, mut auth: A) -> Result<Self, crate::Error> {
        let config = KeycloakConfig {
            base_url: base_url.into(),
            realm: realm.into(),
        };
        if let Err(e) = auth.login(&config).await {
            return if matches!(e.kind(), ErrorKind::Authentication) {
                Err(e)
            } else {
                Err(Error::new(ErrorKind::Authentication, Some(e)))
            };
        }
        if !auth.token_is_valid() {
            return Err(Error::new_kind(ErrorKind::MissingAccessToken));
        }
        let Some(access_token) = auth.access_token() else {
            return Err(Error::new_kind(ErrorKind::MissingAccessToken));
        };
        let client = Self::build_client(access_token);
        Ok(Self {
            config,
            auth: RwLock::new(auth),
            api_client: RwLock::new(self::rest::Client::new_with_client(base_url, client)),
        })
    }

    pub fn config(&self) -> &KeycloakConfig {
        &self.config
    }

    fn build_client(access_token: &str) -> reqwest::Client {
        reqwest::ClientBuilder::new()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(30))
            .default_headers({
                let mut headers = reqwest::header::HeaderMap::new();
                headers.append(
                    reqwest::header::AUTHORIZATION,
                    reqwest::header::HeaderValue::from_str(&format!("Bearer {}", access_token,))
                        .expect("BUG: access token format failed"),
                );
                headers
            })
            .build()
            .expect("BUG: reqwest client builder failed")
    }

    async fn refresh_if_necessary(&self) -> Result<(), crate::Error> {
        tracing::debug!("Checking for token refresh");
        {
            let auth = self.auth.read().await;
            if auth.token_is_valid() {
                tracing::trace!("Token still valid");
                return Ok(());
            }
            if !auth.can_refresh() {
                return Err(Error::new_kind(ErrorKind::TokenExpired));
            }
        }
        // token is invalid (expired) and we can refresh
        tracing::debug!("Refreshing access token");
        let new_client = {
            let mut auth = self.auth.write().await;
            auth.refresh(&self.config).await?;
            let Some(new_token) = auth.access_token() else {
                tracing::warn!("Token refresh failed to get an access token!");
                return Err(Error::new_kind(ErrorKind::MissingAccessToken));
            };
            Self::build_client(new_token)
        };
        let mut api_client = self.api_client.write().await;
        api_client.client = new_client;
        Ok(())
    }

    pub async fn server_info(&self) -> Result<crate::rest::ServerInfo, crate::Error> {
        self.refresh_if_necessary().await?;
        let client = self.api_client.read().await;
        let client = &client.client;
        // this is not part of the openapi spec?
        let response = client
            .get(format!("{}/admin/serverinfo", self.config.base_url))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(crate::error::error_response(response).await);
        }

        let bytes = response.bytes().await?;
        let data = serde_json::from_slice(&bytes).map_err(crate::error::deserialize)?;
        Ok(data)
    }

    pub async fn realm_info(
        &self,
    ) -> Result<crate::rest::types::RealmRepresentation, crate::Error> {
        self.refresh_if_necessary().await?;
        let client = self.api_client.read().await;
        let response = client
            .get_realm(&self.config.realm)
            .await
            .map_err(crate::error::progenitor)?;
        Ok(response.into_inner())
    }
}
