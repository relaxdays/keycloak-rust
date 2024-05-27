use std::fmt::Display;

use bytes::Bytes;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("{kind}")]
pub struct KeycloakError {
    kind: ErrorKind,
    source: Option<InnerError>,
}

#[derive(Debug, Copy, Clone)]
pub enum ResourceType {
    Client,
    Group,
    User,
}

#[derive(Debug, Clone, Error)]
pub enum ErrorKind {
    #[error("failed to deserialize")]
    Deserialize,
    #[error("reqwest error")]
    Reqwest,
    #[error("no access token available")]
    MissingAccessToken,
    #[error("available token(s) expired")]
    TokenExpired,
    #[error("authentication failed")]
    Authentication,
    #[error("http response error (status code {status})")]
    ResponseError {
        status: StatusCode,
        response: Option<bytes::Bytes>,
    },
    #[error("{0}")]
    KeycloakError(KeycloakErrorBody),
    #[error("api error")]
    ApiError,
    #[error("requested {0} resource doesn't exist")]
    NotFound(ResourceType),
    #[error("multiple matching {0} resources returned")]
    NotUnique(ResourceType),
    #[error("missing id")]
    MissingId,
    #[error("missing field in data: {0}")]
    MissingField(String),
    #[error("wrong type (expected {0}, got {1})")]
    WrongType(String, String),
    #[error("unspecified error")]
    Other,
}

/// JSON body returned by Keycloak for errors
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct KeycloakErrorBody {
    pub error: String,
    pub error_description: Option<String>,
}

impl KeycloakError {
    pub(crate) fn new<E>(kind: ErrorKind, inner: Option<E>) -> Self
    where
        E: Into<InnerError>,
    {
        Self {
            kind,
            source: inner.map(Into::into),
        }
    }

    pub(crate) fn new_kind(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }

    /// get the http response status code associated with this error (if any)
    pub fn status(&self) -> Option<StatusCode> {
        if let ErrorKind::ResponseError { status, .. } = self.kind() {
            return Some(*status);
        }
        if let Some(inner) = self.source.as_ref() {
            match inner {
                InnerError::Keycloak(e) => return e.status(),
                InnerError::Progenitor(e) => return e.status(),
                InnerError::Reqwest(e) => return e.status(),
                _ => {}
            }
        }
        None
    }
}

pub fn deserialize(err: serde_json::Error) -> KeycloakError {
    KeycloakError::new(ErrorKind::Deserialize, Some(err))
}

pub fn reqwest(err: reqwest::Error) -> KeycloakError {
    KeycloakError::new(ErrorKind::Reqwest, Some(err))
}

fn from_response(status: StatusCode, bytes: Bytes) -> KeycloakError {
    if let Ok(body) = serde_json::from_slice::<KeycloakErrorBody>(&bytes) {
        KeycloakError::new(
            ErrorKind::ResponseError {
                status,
                response: Some(bytes),
            },
            Some(KeycloakError::new_kind(ErrorKind::KeycloakError(body))),
        )
    } else {
        KeycloakError::new_kind(ErrorKind::ResponseError {
            status,
            response: Some(bytes),
        })
    }
}

pub async fn error_response(resp: reqwest::Response) -> KeycloakError {
    let status = resp.status();
    match resp.bytes().await {
        Ok(bytes) => from_response(status, bytes),
        Err(e) => KeycloakError::new(
            ErrorKind::ResponseError {
                status,
                response: None,
            },
            Some(e),
        ),
    }
}

pub fn progenitor(err: progenitor_client::Error) -> KeycloakError {
    let inner: InnerError = if let Some(status) = err.status() {
        match err {
            progenitor_client::Error::InvalidResponsePayload(bytes, _) => {
                from_response(status, bytes).into()
            }
            _ => KeycloakError::new(
                ErrorKind::ResponseError {
                    status,
                    response: None,
                },
                Some(err),
            )
            .into(),
        }
    } else {
        err.into()
    };
    KeycloakError::new(ErrorKind::ApiError, Some(inner))
}

impl Display for KeycloakErrorBody {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.error))?;
        if let Some(description) = &self.error_description {
            f.write_fmt(format_args!(": {description}"))?;
        }
        Ok(())
    }
}

impl Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Client => write!(f, "client"),
            Self::Group => write!(f, "group"),
            Self::User => write!(f, "user"),
        }
    }
}

/// enum of possible inner errors for a [`KeycloakError`]
#[derive(Debug)]
pub enum InnerError {
    // boxed to prevent `KeycloakError` containing itself
    // boxed here instead of in `KeycloakError` to prevent a double box for `Other`
    Keycloak(Box<KeycloakError>),
    Serde(serde_json::Error),
    Progenitor(progenitor_client::Error<()>),
    Reqwest(reqwest::Error),
    Other(Box<dyn std::error::Error + Send + Sync>),
}

impl InnerError {
    /// explicitly allow using a boxed and type-erased error as inner error
    ///
    /// this is only required until we can use ✨specialization✨ to implement
    /// `From<E: std::error::Error> for InnerError` while also keeping the existing,
    /// more specific implementations
    pub fn from_any<E: std::error::Error + Send + Sync + 'static>(err: E) -> Self {
        Self::Other(Box::new(err))
    }
}

impl std::ops::Deref for InnerError {
    type Target = dyn std::error::Error + Send + Sync;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Keycloak(e) => e,
            Self::Serde(e) => e,
            Self::Progenitor(e) => e,
            Self::Reqwest(e) => e,
            Self::Other(e) => e.deref(),
        }
    }
}

impl From<KeycloakError> for InnerError {
    fn from(value: KeycloakError) -> Self {
        Self::Keycloak(Box::new(value))
    }
}

impl From<serde_json::Error> for InnerError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<progenitor_client::Error<()>> for InnerError {
    fn from(value: progenitor_client::Error<()>) -> Self {
        Self::Progenitor(value)
    }
}

impl From<reqwest::Error> for InnerError {
    fn from(value: reqwest::Error) -> Self {
        Self::Reqwest(value)
    }
}
