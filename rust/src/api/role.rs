use std::future::Future;

use super::KeycloakGroupExt;
use crate::{
    rest::types::{GroupRepresentation, RoleRepresentation, UserRepresentation},
    Error,
};

type Result<T, E = Error> = std::result::Result<T, E>;

/// role-related methods of the keycloak api
pub trait KeycloakRoleExt {
    /// get a single role matching the given name
    fn role_by_name(
        &self,
        role_name: &str,
    ) -> impl Future<Output = Result<RoleRepresentation>> + Send;

    /// get direct member groups of a role
    fn groups_in_role(
        &self,
        role_name: &str,
    ) -> impl Future<Output = Result<Vec<GroupRepresentation>>> + Send;

    /// get all client scopes configured in the realm
    fn users_in_role(
        &self,
        role_name: &str,
        include_indirect: bool,
    ) -> impl Future<Output = Result<Vec<UserRepresentation>>> + Send;
}

impl<A: crate::AuthenticationProvider + Send + Sync> KeycloakRoleExt for crate::Keycloak<A> {
    #[tracing::instrument(skip(self))]
    async fn role_by_name(&self, role_name: &str) -> Result<RoleRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying role by name");
        let response = api_client
            .get_realm_role_by_name(&self.config.realm, role_name)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn groups_in_role(&self, role_name: &str) -> Result<Vec<GroupRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying groups with role");
        let groups = api_client
            .get_realm_role_by_name_groups(&self.config.realm, role_name, Some(true), None, None)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(groups)
    }

    #[tracing::instrument(skip(self))]
    async fn users_in_role(
        &self,
        role_name: &str,
        include_indirect: bool,
    ) -> Result<Vec<UserRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying users with role");
        let mut users = api_client
            .get_realm_role_by_name_users(&self.config.realm, role_name, None, None)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();

        if include_indirect {
            let groups = self.groups_in_role(role_name).await?;
            for group in &groups {
                let mut group_users = self
                    .group_users(group.id.as_ref().unwrap(), Some(true))
                    .await?;
                // deduplicate users who are members of multiple roles
                group_users.retain(|group_user| !users.iter().any(|u| u.id == group_user.id));
                users.extend(group_users);
            }
        }

        Ok(users)
    }
}
