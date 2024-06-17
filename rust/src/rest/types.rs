// don't look below, this is hacky shit

// first re-export all the generated types
pub use self::generated::*;

// re-export custom types
// explicit re-export for `self::policies::ClientPolicyRepresentation` as that's also part of the generated types
pub use self::policies::{ClientPolicyRepresentation, *};

/// types generated from the keycloak openapi spec
pub mod generated {
    // make `rest::generated::types::*` available as `rest::types::generated::*` because lol lmao what the fuck
    pub use crate::rest::generated::types::*;
}

/// concrete subtypes of [`PolicyRepresentation`]
pub mod policies;
