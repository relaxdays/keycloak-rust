use std::future::Future;

use crate::{
    rest::types::{RoleRepresentation, UserRepresentation},
    Error, ErrorKind,
};

type Result<T, E = Error> = std::result::Result<T, E>;

/// user-related methods of the keycloak api
pub trait KeycloakUserExt {
    /// get a single user by their username
    fn user_by_name(
        &self,
        username: &str,
    ) -> impl Future<Output = Result<UserRepresentation>> + Send;

    /// get a user's realm roles given their uuid
    fn user_realm_roles(
        &self,
        user_id: &str,
    ) -> impl Future<Output = Result<Vec<RoleRepresentation>>> + Send;

    /// add realm roles to a user
    fn user_add_realm_roles(
        &self,
        user_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> impl Future<Output = Result<()>> + Send;

    /// remove realm roles from a user
    fn user_remove_realm_roles(
        &self,
        user_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> impl Future<Output = Result<()>> + Send;
}

impl<A: crate::AuthenticationProvider + Send + Sync> KeycloakUserExt for crate::Keycloak<A> {
    #[tracing::instrument(skip(self))]
    async fn user_by_name(&self, username: &str) -> Result<UserRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying user by name");
        let mut response = api_client
            .get_realm_users(
                &self.config.realm,
                None,
                None,
                None,
                None,
                Some(true),
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                None,
                Some(username),
            )
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        if response.is_empty() {
            return Err(Error::new_kind(ErrorKind::NotFound(
                crate::error::ResourceType::User,
            )));
        }
        let user = response.pop().unwrap();
        if !response.is_empty() {
            return Err(Error::new_kind(ErrorKind::NotUnique(
                crate::error::ResourceType::User,
            )));
        }
        Ok(user)
    }

    #[tracing::instrument(skip(self))]
    async fn user_realm_roles(&self, user_id: &str) -> Result<Vec<RoleRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying user realm roles");
        let response = api_client
            .get_realm_user_role_mappings_realm(&self.config.realm, user_id)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn user_add_realm_roles(
        &self,
        user_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> Result<()> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("adding roles to user");
        let response = api_client
            .post_realm_user_role_mappings_realm(&self.config.realm, user_id, roles)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn user_remove_realm_roles(
        &self,
        user_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> Result<()> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("removing roles from user");
        let response = api_client
            .delete_realm_user_role_mappings_realm(&self.config.realm, user_id, roles)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }
}
