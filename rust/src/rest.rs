use serde::{Deserialize, Serialize};

// re-export the generated rest client
pub use self::generated::Client;

/// types used in the api
pub mod types;

#[allow(dead_code, clippy::all, clippy::pedantic, clippy::nursery)]
mod generated {
    include!(concat!(env!("OUT_DIR"), "/keycloak-api-gen.rs"));
}

#[derive(Debug, Serialize)]
pub struct TokenRequest<'a> {
    client_id: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_secret: Option<&'a str>,
    #[serde(flatten)]
    grant: TokenRequestGrant<'a>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "grant_type", rename_all = "snake_case")]
pub enum TokenRequestGrant<'a> {
    Password {
        username: &'a str,
        password: &'a str,
    },
    RefreshToken {
        refresh_token: &'a str,
    },
}

impl<'a> TokenRequest<'a> {
    pub fn new_password(
        client_id: &'a str,
        client_secret: Option<&'a str>,
        username: &'a str,
        password: &'a str,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            grant: TokenRequestGrant::Password { username, password },
        }
    }

    pub fn new_refresh(
        client_id: &'a str,
        client_secret: Option<&'a str>,
        refresh_token: &'a str,
    ) -> Self {
        Self {
            client_id,
            client_secret,
            grant: TokenRequestGrant::RefreshToken { refresh_token },
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub expires_in: u32,
    pub refresh_token: String,
    pub refresh_expires_in: u32,
    pub session_state: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]

pub struct ServerInfo {
    pub system_info: ServerInfoSystemInfo,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServerInfoSystemInfo {
    pub version: String,
    pub java_version: String,
    pub java_vendor: String,
    pub java_vm: String,
    pub java_vm_version: String,
    pub uptime: String,
    pub uptime_millis: u64,
    pub os_name: String,
    pub os_architecture: String,
    pub os_version: String,
    pub file_encoding: String,
}

#[cfg(test)]
mod test {
    #[test]
    fn test_token_request() {
        let request = super::TokenRequest::new_password("id", None, "user", "pass");
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serialized,
            r#"{"client_id":"id","grant_type":"password","username":"user","password":"pass"}"#
        );

        let request = super::TokenRequest::new_refresh("id", None, "token");
        let serialized = serde_json::to_string(&request).unwrap();
        assert_eq!(
            serialized,
            r#"{"client_id":"id","grant_type":"refresh_token","refresh_token":"token"}"#
        );
    }
}
