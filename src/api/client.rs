use std::future::Future;

use crate::{
    rest::types::{
        AbstractPolicyRepresentation, ClientRepresentation, ClientScopeRepresentation,
        PolicyRepresentation, ProtocolMapperRepresentation, ResourceRepresentation,
        ResourceServerRepresentation, ScopeRepresentation,
    },
    Error, ErrorKind,
};

type Result<T, E = Error> = std::result::Result<T, E>;

/// client-related methods of the keycloak api
pub trait KeycloakClientExt {
    /// get all clients in the realm
    fn clients(&self) -> impl Future<Output = Result<Vec<ClientRepresentation>>> + Send;

    /// get a single client given its client id (oidc client id, not the keycloak internal uuid)
    ///
    /// this will return an error if not exactly one client is found with that client id
    fn client_by_id(
        &self,
        client_id: &str,
    ) -> impl Future<Output = Result<ClientRepresentation>> + Send;

    /// get a single client given its uuid
    fn client_by_uuid(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<ClientRepresentation>> + Send;

    /// get the default client scopes of a client
    ///
    /// this only contains id/name for each client scope. to get the full configuration of these
    /// scopes, use [`client_scopes`](crate::api::KeycloakRealmExt::client_scopes)
    fn client_default_scopes(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<Vec<ClientScopeRepresentation>>> + Send;

    /// get the optional client scopes of a client
    ///
    /// this only contains id/name for each client scope. to get the full configuration of these
    /// scopes, use [`client_scopes`](crate::api::KeycloakRealmExt::client_scopes)
    fn client_optional_scopes(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<Vec<ClientScopeRepresentation>>> + Send;

    /// get a client's authorization service settings
    fn client_authz_resource_server(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<ResourceServerRepresentation>> + Send;

    /// get all authorization resources of a client
    fn client_authz_resources(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<Vec<ResourceRepresentation>>> + Send;

    /// get the permissions associated with a client's authorization resource
    fn client_authz_resource_permissions(
        &self,
        client_uuid: &str,
        resource_id: &str,
    ) -> impl Future<Output = Result<Vec<PolicyRepresentation>>> + Send;

    /// get the authorization scopes associated with a client's authorization resource
    fn client_authz_resource_scopes(
        &self,
        client_uuid: &str,
        resource_id: &str,
    ) -> impl Future<Output = Result<Vec<ScopeRepresentation>>> + Send;

    /// get all authorization scopes of a client
    fn client_authz_scopes(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<Vec<ScopeRepresentation>>> + Send;

    /// get all authorization permissions of a client
    fn client_authz_permissions(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<Vec<AbstractPolicyRepresentation>>> + Send;

    /// get all authorization policies of a client
    fn client_authz_policies(
        &self,
        client_uuid: &str,
    ) -> impl Future<Output = Result<Vec<AbstractPolicyRepresentation>>> + Send;

    /// update a dedicated protocol mapper configured in a client's dedicated client scope
    ///
    /// the `id` must reference an existing protocol mapper in the given [`ProtocolMapperRepresentation`]
    ///
    /// this will update the entire configuration of the protocol mapper to the given values
    fn update_client_protocol_mapper(
        &self,
        client_id: &str,
        mapper: &ProtocolMapperRepresentation,
    ) -> impl Future<Output = Result<()>> + Send;
}

impl<A: crate::AuthenticationProvider + Send + Sync> KeycloakClientExt for crate::Keycloak<A> {
    #[tracing::instrument(skip(self))]
    async fn clients(&self) -> Result<Vec<ClientRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying all clients in realm");
        let clients = paginate_api!(|first, max| {
            api_client
                .get_realm_clients(
                    &self.config.realm,
                    None,
                    Some(first),
                    Some(max),
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(clients)
    }

    #[tracing::instrument(skip(self))]
    async fn client_by_id(&self, client_id: &str) -> Result<ClientRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client in realm by client id");
        let mut clients = paginate_api!(|first, max| {
            api_client
                .get_realm_clients(
                    &self.config.realm,
                    Some(client_id),
                    Some(first),
                    Some(max),
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        if clients.is_empty() {
            return Err(Error::new_kind(ErrorKind::NotFound(
                crate::error::ResourceType::Client,
            )));
        }
        let client = clients.pop().unwrap();
        if !clients.is_empty() {
            return Err(Error::new_kind(ErrorKind::NotUnique(
                crate::error::ResourceType::Client,
            )));
        }
        Ok(client)
    }

    #[tracing::instrument(skip(self))]
    async fn client_by_uuid(&self, client_uuid: &str) -> Result<ClientRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client in realm by uuid");
        let response = api_client
            .get_realm_client(&self.config.realm, client_uuid)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_default_scopes(
        &self,
        client_uuid: &str,
    ) -> Result<Vec<ClientScopeRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying default client scopes");
        let response = api_client
            .get_realm_client_default_client_scopes(&self.config.realm, client_uuid)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_optional_scopes(
        &self,
        client_uuid: &str,
    ) -> Result<Vec<ClientScopeRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying optional client scopes");
        let response = api_client
            .get_realm_client_optional_client_scopes(&self.config.realm, client_uuid)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_resource_server(
        &self,
        client_uuid: &str,
    ) -> Result<ResourceServerRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz resource server");
        let response = api_client
            .get_realm_client_authz_resource_server(&self.config.realm, client_uuid)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_resources(
        &self,
        client_uuid: &str,
    ) -> Result<Vec<ResourceRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz resources");
        let response = paginate_api!(|first, max| {
            api_client
                .get_realm_client_authz_resource_server_resource(
                    &self.config.realm,
                    client_uuid,
                    None,
                    None,
                    None,
                    Some(first),
                    None,
                    Some(max),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_resource_permissions(
        &self,
        client_uuid: &str,
        resource_id: &str,
    ) -> Result<Vec<PolicyRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz resource permissions");
        let response = paginate_api!(|first, max| {
            api_client
                .get_realm_client_authz_resource_server_resource_by_id_permissions(
                    &self.config.realm,
                    client_uuid,
                    resource_id,
                    None,
                    None,
                    None,
                    Some(first),
                    None,
                    Some(max),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_resource_scopes(
        &self,
        client_uuid: &str,
        resource_id: &str,
    ) -> Result<Vec<ScopeRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz resource scopes");
        let response = paginate_api!(|first, max| {
            api_client
                .get_realm_client_authz_resource_server_resource_by_id_scopes(
                    &self.config.realm,
                    client_uuid,
                    resource_id,
                    None,
                    None,
                    None,
                    Some(first),
                    None,
                    Some(max),
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_scopes(&self, client_uuid: &str) -> Result<Vec<ScopeRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz scopes");
        let response = paginate_api!(|first, max| {
            api_client
                .get_realm_client_authz_resource_server_scope(
                    &self.config.realm,
                    client_uuid,
                    Some(first),
                    Some(max),
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_permissions(
        &self,
        client_uuid: &str,
    ) -> Result<Vec<AbstractPolicyRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz scopes");
        let response = paginate_api!(|first, max| {
            api_client
                .get_realm_client_authz_resource_server_permission(
                    &self.config.realm,
                    client_uuid,
                    None,
                    Some(first),
                    Some(max),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn client_authz_policies(
        &self,
        client_uuid: &str,
    ) -> Result<Vec<AbstractPolicyRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying client authz policies");
        let response = paginate_api!(|first, max| {
            api_client
                .get_realm_client_authz_resource_server_policy(
                    &self.config.realm,
                    client_uuid,
                    None,
                    Some(first),
                    Some(max),
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                    None,
                )
                .await
                .map_err(crate::error::progenitor)?
                .into_inner()
        });
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn update_client_protocol_mapper(
        &self,
        client_id: &str,
        mapper: &ProtocolMapperRepresentation,
    ) -> Result<()> {
        let Some(protocol_mapper_id) = mapper.id.as_ref() else {
            return Err(Error::new_kind(ErrorKind::MissingId));
        };
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("updating protocol mapper");
        let response = api_client
            .put_realm_client_protocol_mappers_models_id(
                &self.config.realm,
                client_id,
                &protocol_mapper_id,
                &mapper,
            )
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }
}
