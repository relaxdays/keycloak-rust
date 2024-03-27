use std::future::Future;

use crate::{rest::types::RealmRepresentation, Error};

/// realm-level methods of the keycloak api
pub trait KeycloakRealmExt {
    fn realm_info(&self) -> impl Future<Output = Result<RealmRepresentation, Error>> + Send;
}

impl<A: crate::AuthenticationProvider + Send + Sync> KeycloakRealmExt for crate::Keycloak<A> {
    async fn realm_info(&self) -> Result<RealmRepresentation, Error> {
        self.refresh_if_necessary().await?;
        let client = self.api_client.read().await;
        let response = client
            .get_realm(&self.config.realm)
            .await
            .map_err(crate::error::progenitor)?;
        Ok(response.into_inner())
    }
}
