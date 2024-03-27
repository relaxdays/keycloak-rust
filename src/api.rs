//! this module (or rather, its submodules) implements the keycloak api using extension traits so
//! we have less clutter

pub mod realm;

pub use self::realm::KeycloakRealmExt;
