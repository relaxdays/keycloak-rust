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

    /// get a single role given its id
    fn role_by_id(&self, role_id: &str) -> impl Future<Output = Result<RoleRepresentation>> + Send;

    /// get direct member groups of a role
    ///
    /// if a `client_id` is specified, this queries a client role
    fn groups_in_role(
        &self,
        client_id: Option<&str>,
        role_name: &str,
    ) -> impl Future<Output = Result<Vec<GroupRepresentation>>> + Send;

    /// get all members of a role
    ///
    /// if `include_indirect` is `true`, members of groups with this role are returned as well
    fn users_in_role_by_id(
        &self,
        role_id: &str,
        include_indirect: bool,
    ) -> impl Future<Output = Result<Vec<UserRepresentation>>> + Send;

    /// get all members of a role
    ///
    /// if `include_indirect` is `true`, members of groups with this role are returned as well
    /// if a `client_id` is specified, this queries a client role
    fn users_in_role(
        &self,
        client_id: Option<&str>,
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
    async fn role_by_id(&self, role_id: &str) -> Result<RoleRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying role by id");
        let response = api_client
            .get_realm_role_by_id(&self.config.realm, role_id)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn groups_in_role(
        &self,
        client_id: Option<&str>,
        role_name: &str,
    ) -> Result<Vec<GroupRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying groups with role");
        let groups = if let Some(client_id) = client_id {
            paginate_api!(|first, max| {
                api_client
                    .get_realm_client_role_by_name_groups(
                        &self.config.realm,
                        client_id,
                        role_name,
                        Some(true),
                        Some(first),
                        Some(max),
                    )
                    .await
                    .map_err(crate::error::progenitor)?
                    .into_inner()
            })
        } else {
            paginate_api!(|first, max| {
                api_client
                    .get_realm_role_by_name_groups(
                        &self.config.realm,
                        role_name,
                        Some(true),
                        Some(first),
                        Some(max),
                    )
                    .await
                    .map_err(crate::error::progenitor)?
                    .into_inner()
            })
        };
        Ok(groups)
    }

    #[tracing::instrument(skip(self))]
    async fn users_in_role_by_id(
        &self,
        role_id: &str,
        include_indirect: bool,
    ) -> Result<Vec<UserRepresentation>> {
        let role = self.role_by_id(role_id).await?;

        if role.client_role == Some(true) {
            self.users_in_role(
                Some(&role.container_id.unwrap()),
                &role.name.unwrap(),
                include_indirect,
            )
            .await
        } else {
            self.users_in_role(None, &role.name.unwrap(), include_indirect)
                .await
        }
    }

    #[tracing::instrument(skip(self))]
    async fn users_in_role(
        &self,
        client_id: Option<&str>,
        role_name: &str,
        include_indirect: bool,
    ) -> Result<Vec<UserRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying users with role");
        let mut users = if let Some(client_id) = client_id {
            paginate_api!(|first, max| {
                api_client
                    .get_realm_client_role_by_name_users(
                        &self.config.realm,
                        client_id,
                        role_name,
                        None,
                        Some(first),
                        Some(max),
                    )
                    .await
                    .map_err(crate::error::progenitor)?
                    .into_inner()
            })
        } else {
            paginate_api!(|first, max| {
                api_client
                    .get_realm_role_by_name_users(
                        &self.config.realm,
                        role_name,
                        None,
                        Some(first),
                        Some(max),
                    )
                    .await
                    .map_err(crate::error::progenitor)?
                    .into_inner()
            })
        };

        if include_indirect {
            let groups = self.groups_in_role(client_id, role_name).await?;
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
