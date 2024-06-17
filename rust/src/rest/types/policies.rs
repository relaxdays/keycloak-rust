use serde::{Deserialize, Serialize};

use crate::{rest::types::*, Error, ErrorKind};

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AggregatePolicyRepresentation {
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientPolicyRepresentation {
    pub clients: Vec<String>,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientScopeRepresentationClientScopeDefinition {
    pub id: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientScopePolicyRepresentation {
    pub client_scopes: Vec<ClientScopeRepresentationClientScopeDefinition>,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupPolicyRepresentationGroupDefinition {
    pub id: String,
    pub extend_children: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupPolicyRepresentation {
    pub groups: Vec<RolePolicyRepresentationRoleDefinition>,
    pub groups_claim: String,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JsPolicyRepresentation {
    pub code: String,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexPolicyRepresentation {
    pub target_claim: String,
    pub pattern: String,
    pub target_context_attributes: bool,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePolicyRepresentationRoleDefinition {
    pub id: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RolePolicyRepresentation {
    pub roles: Vec<RolePolicyRepresentationRoleDefinition>,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserPolicyRepresentation {
    pub users: Vec<String>,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

impl TryFrom<PolicyRepresentation> for AggregatePolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "aggregate")?;

        check_policy_config(&mut value, "aggregate");

        Ok(Self { policy: value })
    }
}

impl TryFrom<PolicyRepresentation> for ClientPolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "client")?;

        let clients = deserialize_policy_config_field(&mut value, "clients")?;

        check_policy_config(&mut value, "client");

        Ok(Self {
            clients,
            policy: value,
        })
    }
}

impl TryFrom<PolicyRepresentation> for ClientScopePolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "client-scope")?;

        let client_scopes = deserialize_policy_config_field(&mut value, "clientScopes")?;

        check_policy_config(&mut value, "client-scope");

        Ok(Self {
            client_scopes,
            policy: value,
        })
    }
}

impl TryFrom<PolicyRepresentation> for GroupPolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "group")?;

        let groups = deserialize_policy_config_field(&mut value, "groups")?;
        let groups_claim = get_policy_config_field(&mut value, "groupsClaim")?;

        check_policy_config(&mut value, "group");

        Ok(Self {
            groups,
            groups_claim,
            policy: value,
        })
    }
}

impl TryFrom<PolicyRepresentation> for JsPolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "js")?;

        let code = get_policy_config_field(&mut value, "code")?;

        check_policy_config(&mut value, "js");

        Ok(Self {
            code,
            policy: value,
        })
    }
}

impl TryFrom<PolicyRepresentation> for RegexPolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "regex")?;

        let target_claim = get_policy_config_field(&mut value, "targetClaim")?;
        let pattern = get_policy_config_field(&mut value, "pattern")?;
        let target_context_attributes =
            deserialize_policy_config_field(&mut value, "targetContextAttributes")?;

        check_policy_config(&mut value, "regex");

        Ok(Self {
            target_claim,
            pattern,
            target_context_attributes,
            policy: value,
        })
    }
}

impl TryFrom<PolicyRepresentation> for RolePolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "role")?;

        let roles = deserialize_policy_config_field(&mut value, "roles")?;

        check_policy_config(&mut value, "role");

        Ok(Self {
            roles,
            policy: value,
        })
    }
}

impl TryFrom<PolicyRepresentation> for UserPolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        check_policy_type(&value, "user")?;

        let users = deserialize_policy_config_field(&mut value, "users")?;

        check_policy_config(&mut value, "user");

        Ok(Self {
            users,
            policy: value,
        })
    }
}

fn check_policy_type(
    policy: &PolicyRepresentation,
    expected_type: &'static str,
) -> Result<(), Error> {
    let Some(policy_type) = policy.type_.as_ref() else {
        return Err(Error::new_kind(ErrorKind::MissingField("type".into())));
    };

    if policy_type != expected_type {
        return Err(Error::new_kind(ErrorKind::WrongType(
            expected_type.into(),
            policy_type.clone(),
        )));
    }

    Ok(())
}
fn check_policy_config(policy: &mut PolicyRepresentation, expected_type: &'static str) {
    if !policy.config.is_empty() {
        let keys = policy.config.keys().fold(String::new(), |mut a, b| {
            if !a.is_empty() {
                a.push_str(", ")
            }
            a.push_str(&b);
            a
        });
        tracing::warn!(
            "did not deserialize all config fields in {expected_type} policy! remaining fields: {keys}"
        );
    }
    // clear config to make sure we don't serialize config when serializing this policy
    policy.config.clear();
}

fn get_policy_config_field(
    policy: &mut PolicyRepresentation,
    field: &'static str,
) -> Result<String, Error> {
    let Some(value) = policy.config.remove(field) else {
        return Err(Error::new_kind(ErrorKind::MissingField(format!(
            "config.{field}"
        ))));
    };
    Ok(value)
}

fn deserialize_policy_config_field<T>(
    policy: &mut PolicyRepresentation,
    field: &'static str,
) -> Result<T, Error>
where
    T: for<'a> Deserialize<'a>, // maybe Deserialize<'static> would be enough?
{
    let value = get_policy_config_field(policy, field)?;
    serde_json::from_str(&value).map_err(crate::error::deserialize)
}
