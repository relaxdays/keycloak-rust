use std::future::Future;
use std::time::{Duration, Instant};

use crate::error::ErrorKind;
use crate::KeycloakConfig;

/// trait for handling authentication to keycloak
///
/// most users of this crate should probably use the [`DirectGrantAuth`] implementation that logs in using keycloak's
/// direct access grants.
pub trait AuthenticationProvider {
    /// login to keycloak
    ///
    /// this is usually only called exactly once from within [`Keycloak::new`](crate::Keycloak::new). afterwards,
    /// [`access_token`](AuthenticationProvider::access_token) should return the obtained access token.
    fn login(
        &mut self,
        cfg: &KeycloakConfig,
    ) -> impl Future<Output = Result<(), crate::Error>> + Send;
    /// refresh access token
    ///
    /// this is called whenever [`needs_refresh`](AuthenticationProvider::needs_refresh) returns `true` before sending
    /// a request to keycloak. this should re-authenticate to keycloak and obtain a new access token to be returned by
    /// subsequent calls to [`access_token`](AuthenticationProvider::access_token).
    fn refresh(
        &mut self,
        cfg: &KeycloakConfig,
    ) -> impl Future<Output = Result<(), crate::Error>> + Send;
    /// get access token
    fn access_token(&self) -> Option<&str>;
    /// check whether the current access token is still valid
    fn token_is_valid(&self) -> bool;
    /// check whether this authentication provider requires refreshing (e.g. when the access token is about to expire)
    fn needs_refresh(&self) -> bool;
    /// check whether this authentication provider can handle re-authentication at all
    ///
    /// [`refresh`](AuthenticationProvider::refresh) will never be called if this returns `false`.
    fn can_refresh(&self) -> bool;
}

/// abstraction layer allowing to use any of the standard implementations of [`AuthenticationProvider`]
///
/// as [`AuthenticationProvider`] is not object safe, creating something like
/// `Keycloak<Box<dyn AuthentcationProvider>>` won't work and we need this enum to work around this
pub enum Auth {
    AccessToken(AccessTokenAuth),
    DirectGrant(DirectGrantAuth),
}

impl AuthenticationProvider for Auth {
    async fn login(&mut self, cfg: &KeycloakConfig) -> Result<(), crate::Error> {
        match self {
            Self::AccessToken(a) => a.login(cfg).await,
            Self::DirectGrant(a) => a.login(cfg).await,
        }
    }
    async fn refresh(&mut self, cfg: &KeycloakConfig) -> Result<(), crate::Error> {
        match self {
            Self::AccessToken(a) => a.refresh(cfg).await,
            Self::DirectGrant(a) => a.refresh(cfg).await,
        }
    }
    fn access_token(&self) -> Option<&str> {
        match self {
            Self::AccessToken(a) => a.access_token(),
            Self::DirectGrant(a) => a.access_token(),
        }
    }
    fn token_is_valid(&self) -> bool {
        match self {
            Self::AccessToken(a) => a.token_is_valid(),
            Self::DirectGrant(a) => a.token_is_valid(),
        }
    }
    fn needs_refresh(&self) -> bool {
        match self {
            Self::AccessToken(a) => a.needs_refresh(),
            Self::DirectGrant(a) => a.needs_refresh(),
        }
    }
    fn can_refresh(&self) -> bool {
        match self {
            Self::AccessToken(a) => a.can_refresh(),
            Self::DirectGrant(a) => a.can_refresh(),
        }
    }
}

pub struct AccessTokenAuth {
    access_token: String,
}

impl AccessTokenAuth {
    pub fn new(access_token: String) -> Self {
        Self { access_token }
    }
}

impl AuthenticationProvider for AccessTokenAuth {
    async fn login(&mut self, _: &KeycloakConfig) -> Result<(), crate::Error> {
        Ok(())
    }

    async fn refresh(&mut self, _: &KeycloakConfig) -> Result<(), crate::Error> {
        Err(crate::Error::new_kind(ErrorKind::Authentication))
    }

    fn access_token(&self) -> Option<&str> {
        Some(&self.access_token)
    }

    fn token_is_valid(&self) -> bool {
        // TODO: try to decode the jwt and get expiry?
        true
    }

    fn needs_refresh(&self) -> bool {
        !self.token_is_valid()
    }

    fn can_refresh(&self) -> bool {
        false
    }
}

struct Tokens {
    access_token: String,
    expiry: Instant,
    refresh_token: String,
    refresh_expiry: Instant,
}

pub struct DirectGrantAuth {
    client_id: String,
    client_secret: Option<String>,
    username: String,
    password: String,
    tokens: Option<Tokens>,
    client: reqwest::Client,
}

impl DirectGrantAuth {
    pub fn new(
        client_id: &str,
        client_secret: Option<&str>,
        username: &str,
        password: &str,
    ) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.map(Into::into),
            username: username.into(),
            password: password.into(),
            tokens: None,
            client: reqwest::Client::new(),
        }
    }

    async fn request_tokens(
        &self,
        cfg: &KeycloakConfig,
        request: &crate::rest::TokenRequest<'_>,
    ) -> Result<Tokens, crate::Error> {
        let url = format!(
            "{}/realms/{}/protocol/openid-connect/token",
            cfg.base_url, cfg.realm
        );
        let response = self.client.post(url).form(&request).send().await?;
        if !response.status().is_success() {
            return Err(crate::error::error_response(response).await);
        }
        let token: crate::rest::TokenResponse = response.json().await?;

        let time = Instant::now();
        Ok(Tokens {
            access_token: token.access_token,
            expiry: time + Duration::from_secs(token.expires_in.into()),
            refresh_token: token.refresh_token,
            refresh_expiry: time + Duration::from_secs(token.refresh_expires_in.into()),
        })
    }
}

impl AuthenticationProvider for DirectGrantAuth {
    async fn login(&mut self, cfg: &KeycloakConfig) -> Result<(), crate::Error> {
        let request = crate::rest::TokenRequest::new_password(
            &self.client_id,
            self.client_secret.as_deref(),
            &self.username,
            &self.password,
        );
        let tokens = self.request_tokens(cfg, &request).await?;
        self.tokens = Some(tokens);
        Ok(())
    }

    async fn refresh(&mut self, cfg: &KeycloakConfig) -> Result<(), crate::Error> {
        let new_tokens = if let Some(tokens) = &self.tokens {
            let request = crate::rest::TokenRequest::new_refresh(
                &self.client_id,
                self.client_secret.as_deref(),
                &tokens.refresh_token,
            );
            self.request_tokens(cfg, &request).await?
        } else {
            return Err(crate::Error::new_kind(ErrorKind::MissingAccessToken));
        };
        self.tokens = Some(new_tokens);
        Ok(())
    }

    fn access_token(&self) -> Option<&str> {
        self.tokens.as_ref().map(|t| t.access_token.as_str())
    }

    fn token_is_valid(&self) -> bool {
        match &self.tokens {
            None => false,
            Some(t) => t.expiry >= Instant::now(),
        }
    }

    fn needs_refresh(&self) -> bool {
        match &self.tokens {
            None => false,
            Some(t) => {
                // TODO: make the offset configurable?
                (t.expiry - Duration::from_secs(10)) < Instant::now()
            }
        }
    }

    fn can_refresh(&self) -> bool {
        match &self.tokens {
            None => false,
            Some(t) => t.refresh_expiry >= Instant::now(),
        }
    }
}
