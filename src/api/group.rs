use std::future::Future;

use crate::{
    rest::types::{GroupRepresentation, RoleRepresentation},
    Error,
};

type Result<T, E = Error> = std::result::Result<T, E>;

/// group-related methods of the keycloak api
pub trait KeycloakGroupExt {
    /// get a single group matching the given name
    fn group_by_name(
        &self,
        group_name: &str,
    ) -> impl Future<Output = Result<GroupRepresentation>> + Send;

    /// get a single group using its uuid
    fn group_by_id(
        &self,
        group_id: &str,
    ) -> impl Future<Output = Result<GroupRepresentation>> + Send;

    /// get the realm roles associated with a group
    fn group_realm_roles(
        &self,
        group_id: &str,
    ) -> impl Future<Output = Result<Vec<RoleRepresentation>>> + Send;

    /// add new realm roles to a group
    #[allow(clippy::ptr_arg)] // generated api client requires &Vec<T>
    fn group_add_realm_roles(
        &self,
        group_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> impl Future<Output = Result<()>> + Send;

    /// remove existing realm roles from a group
    #[allow(clippy::ptr_arg)] // generated api client requires &Vec<T>
    fn group_remove_realm_roles(
        &self,
        group_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> impl Future<Output = Result<()>> + Send;
}

impl<A: crate::AuthenticationProvider + Send + Sync> KeycloakGroupExt for crate::Keycloak<A> {
    #[tracing::instrument(skip(self))]
    async fn group_by_name(&self, group_name: &str) -> Result<GroupRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying group by name");
        // TODO: paginate this api?
        let mut groups = api_client
            .get_realm_groups(
                &self.config.realm,
                Some(false),
                Some(true),
                None,
                None,
                Some(false),
                None,
                Some(group_name),
            )
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        while !groups.is_empty() {
            let mut sub_groups = Vec::new();
            for group in groups {
                if group.name.as_ref().map_or("", String::as_ref) == group_name {
                    return Ok(group);
                };
                sub_groups.extend(group.sub_groups);
            }
            groups = sub_groups;
        }
        Err(Error::new_kind(crate::ErrorKind::NotFound(
            crate::error::ResourceType::Group,
        )))
    }

    #[tracing::instrument(skip(self))]
    async fn group_by_id(&self, group_id: &str) -> Result<GroupRepresentation> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying group by id");
        let response = api_client
            .get_realm_group(&self.config.realm, group_id)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn group_realm_roles(&self, group_id: &str) -> Result<Vec<RoleRepresentation>> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("querying group realm roles");
        let response = api_client
            .get_realm_group_role_mappings_realm(&self.config.realm, group_id)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(response)
    }

    #[tracing::instrument(skip(self))]
    async fn group_add_realm_roles(
        &self,
        group_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> Result<()> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("adding realm roles to group");
        api_client
            .post_realm_group_role_mappings_realm(&self.config.realm, group_id, roles)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn group_remove_realm_roles(
        &self,
        group_id: &str,
        roles: &Vec<RoleRepresentation>,
    ) -> Result<()> {
        self.refresh_if_necessary().await?;
        let api_client = self.api_client.read().await;

        tracing::debug!("removing realm roles from group");
        api_client
            .delete_realm_group_role_mappings_realm(&self.config.realm, group_id, roles)
            .await
            .map_err(crate::error::progenitor)?
            .into_inner();
        Ok(())
    }
}
