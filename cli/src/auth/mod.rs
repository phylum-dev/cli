pub use ip_addr_ext::*;
pub use oidc::*;
pub use server::*;

mod ip_addr_ext;
pub mod jwt;
mod oidc;
mod server;

pub fn is_locksmith_token(token: impl AsRef<str>) -> bool {
    token.as_ref().starts_with("ph0_")
}
