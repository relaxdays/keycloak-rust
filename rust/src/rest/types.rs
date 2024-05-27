use serde::{Deserialize, Serialize};

use crate::{Error, ErrorKind};

// re-export generated all the generated types in addition to our custom defined types
pub use super::generated::types::*;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RolePolicyRepresentationRoleDefinition {
    pub id: String,
    #[serde(default)]
    pub required: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RolePolicyRepresentation {
    pub roles: Vec<RolePolicyRepresentationRoleDefinition>,
    #[serde(flatten)]
    pub policy: PolicyRepresentation,
}

impl TryFrom<PolicyRepresentation> for RolePolicyRepresentation {
    type Error = Error;

    fn try_from(mut value: PolicyRepresentation) -> Result<Self, Self::Error> {
        let Some(policy_type) = value.type_.as_ref() else {
            return Err(Error::new_kind(ErrorKind::MissingField("type".into())));
        };

        if policy_type != "role" {
            return Err(Error::new_kind(ErrorKind::WrongType(
                "role".into(),
                policy_type.clone(),
            )));
        }
        let Some(roles) = value.config.get("roles") else {
            return Err(Error::new_kind(ErrorKind::MissingField(
                "config.roles".into(),
            )));
        };
        let roles = serde_json::from_str(&roles).map_err(crate::error::deserialize)?;

        // clear config to make sure we don't serialize config when serializing this policy
        value.config.clear();

        Ok(Self {
            roles,
            policy: value,
        })
    }
}
