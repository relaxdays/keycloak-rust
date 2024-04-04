use std::future::Future;

use crate::{
    rest::types::{ClientScopeRepresentation, RealmRepresentation},
    Error,
};

type Result<T, E = Error> = std::result::Result<T, E>;

/// realm-level methods of the keycloak api
pub trait KeycloakRealmExt {
    fn realm_info(&self) -> impl Future<Output = Result<RealmRepresentation>> + Send;

    /// get all client scopes configured in the realm
    fn client_scopes(&self) -> impl Future<Output = Result<Vec<ClientScopeRepresentation>>> + Send;
}

impl<A: crate::AuthenticationProvider + Send + Sync> KeycloakRealmExt for crate::Keycloak<A> {
    #[tracing::instrument(skip(self))]
    async fn realm_info(&self) -> Result<RealmRepresentation> {
        self.refresh_if_necessary().await?;
        let client = self.api_client.read().await;
        let response = client
            .get_realm(&self.config.realm)
            .await
            .map_err(crate::error::progenitor)?;
        Ok(response.into_inner())
    }

    #[tracing::instrument(skip(self))]
    async fn client_scopes(&self) -> Result<Vec<ClientScopeRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying all client scopes");
        let response = api_client
            .get_realm_client_scopes(&self.config.realm)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }
}
