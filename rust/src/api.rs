//! this module (or rather, its submodules) implements the keycloak api using extension traits so
//! we have less clutter

pub mod client;
pub mod group;
pub mod realm;
pub mod role;

pub use self::{
    client::KeycloakClientExt, group::KeycloakGroupExt, realm::KeycloakRealmExt,
    role::KeycloakRoleExt,
};
