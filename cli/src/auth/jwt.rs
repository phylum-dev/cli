/// JWT token parsing.
use std::collections::HashSet;
use std::string::FromUtf8Error;

use base64::engine::general_purpose::STANDARD;
use base64::prelude::*;
use base64::DecodeError;
use serde::Deserialize;
use serde_json::Error as JsonError;
use thiserror;

/// Get user roles from a bearer token without performing validation.
pub fn user_roles(bearer: &str) -> Result<HashSet<RealmRole>, JwtError> {
    // Extract the base64 payload.
    let (_, payload_base64, _) = parts(bearer)?;

    // Decode the payload.
    let payload_bytes = STANDARD.decode(payload_base64)?;
    let payload_text = String::from_utf8(payload_bytes)?;

    // Parse as JSON.
    let payload: PhylumBearer = serde_json::from_str(&payload_text)?;

    Ok(payload.realm_access.roles)
}

/// Split a bearer token into header/payload/signature.
fn parts(bearer: &str) -> Result<(&str, &str, &str), JwtError> {
    let mut parts = bearer.split('.');

    let header = parts.next().unwrap();
    let payload = parts.next().ok_or(JwtError::MissingPayload)?;
    let signature = parts.next().ok_or(JwtError::MissingSignature)?;

    match parts.next() {
        Some(_) => Err(JwtError::UnexpectedComponent),
        None => Ok((header, payload, signature)),
    }
}

/// Available Phylum JWT realm roles.
#[derive(Deserialize, Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum RealmRole {
    #[serde(rename = "pro-account")]
    Pro,
    #[serde(rename = "vulnreach")]
    Vulnreach,
    #[serde(other)]
    Unknown,
}

/// JWT parsing error.
#[derive(thiserror::Error, Debug)]
pub enum JwtError {
    #[error("JWT missing payload part")]
    MissingPayload,
    #[error("JWT missing signature part")]
    MissingSignature,
    #[error("JWT cannot have more than 3 parts")]
    UnexpectedComponent,
    #[error("invalid base64")]
    InvalidBase64(#[from] DecodeError),
    #[error("invalid UTF-8")]
    InvalidUtf8(#[from] FromUtf8Error),
    #[error("invalid JSON")]
    InvalidJson(#[from] JsonError),
}

/// Partial Phylum JWT bearer payload.
#[derive(Deserialize, Debug)]
struct PhylumBearer {
    realm_access: RealmAccess,
}

/// Partial Phylum JWT realm access.
#[derive(Deserialize, Debug)]
struct RealmAccess {
    roles: HashSet<RealmRole>,
}
