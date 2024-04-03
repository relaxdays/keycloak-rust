use std::fmt::Display;

use bytes::Bytes;
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub(crate) type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Error)]
#[error("{kind}")]
pub struct KeycloakError {
    kind: ErrorKind,
    source: Option<BoxError>,
}

#[derive(Debug, Copy, Clone)]
pub enum ResourceType {
    Client,
    Group,
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
        status: u16,
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
    #[error("unspecified error")]
    Other,
}

/// JSON body returned by Keycloak for errors
#[derive(Debug, Clone, Deserialize, Serialize, Error)]
pub struct KeycloakErrorBody {
    pub error: String,
    pub error_description: Option<String>,
}

impl KeycloakError {
    pub fn new<E>(kind: ErrorKind, inner: Option<E>) -> Self
    where
        E: Into<BoxError>,
    {
        Self {
            kind,
            source: inner.map(Into::into),
        }
    }

    pub fn new_kind(kind: ErrorKind) -> Self {
        Self { kind, source: None }
    }

    pub fn kind(&self) -> &ErrorKind {
        &self.kind
    }
}

pub fn deserialize(err: serde_json::Error) -> KeycloakError {
    KeycloakError::new(ErrorKind::Deserialize, Some(err))
}

pub fn reqwest(err: reqwest::Error) -> KeycloakError {
    KeycloakError::new(ErrorKind::Reqwest, Some(err))
}

fn from_response(status: u16, bytes: Bytes) -> KeycloakError {
    if let Ok(body) = serde_json::from_slice::<KeycloakErrorBody>(&bytes) {
        KeycloakError::new(
            ErrorKind::ResponseError {
                status,
                response: Some(bytes),
            },
            Some(body),
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
        Ok(bytes) => from_response(status.into(), bytes),
        Err(e) => KeycloakError::new(
            ErrorKind::ResponseError {
                status: status.into(),
                response: None,
            },
            Some(e),
        ),
    }
}

pub fn progenitor(err: progenitor_client::Error) -> KeycloakError {
    let inner: BoxError = if let Some(status) = err.status() {
        match err {
            progenitor_client::Error::InvalidResponsePayload(bytes, _) => {
                Box::new(from_response(status.into(), bytes))
            }
            _ => Box::new(KeycloakError::new(
                ErrorKind::ResponseError {
                    status: status.into(),
                    response: None,
                },
                Some(err),
            )),
        }
    } else {
        Box::new(err)
    };
    KeycloakError::new(ErrorKind::ApiError, Some(inner))
}

impl From<serde_json::Error> for KeycloakError {
    fn from(value: serde_json::Error) -> Self {
        deserialize(value)
    }
}

impl From<reqwest::Error> for KeycloakError {
    fn from(value: reqwest::Error) -> Self {
        reqwest(value)
    }
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
        }
    }
}
