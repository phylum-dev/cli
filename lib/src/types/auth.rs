use serde::{Deserialize, Serialize};

/// Typed wrapper for AuthorizationCode
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AuthorizationCode(String);

impl AuthorizationCode {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl From<&AuthorizationCode> for String {
    fn from(val: &AuthorizationCode) -> Self {
        val.0.to_owned()
    }
}

/// Typed wrapper for RefreshToken
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RefreshToken(String);

impl RefreshToken {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl From<&RefreshToken> for String {
    fn from(val: &RefreshToken) -> Self {
        val.0.to_owned()
    }
}

/// Typed wrapper for AccessToken
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl From<&AccessToken> for String {
    fn from(val: &AccessToken) -> Self {
        val.0.to_owned()
    }
}

impl<'a> From<&'a AccessToken> for &'a str {
    fn from(val: &'a AccessToken) -> Self {
        &val.0
    }
}

/// Typed wrapper for IdToken
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct IdToken(String);

impl IdToken {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl From<&IdToken> for String {
    fn from(val: &IdToken) -> Self {
        val.0.to_owned()
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TokenResponse {
    pub access_token: AccessToken,
    pub refresh_token: RefreshToken,
    pub id_token: IdToken,
    #[serde(rename = "expires_in")]
    pub expires_in_seconds: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct AccessTokenResponse {
    pub access_token: AccessToken,
    #[serde(rename = "expires_in")]
    pub expires_in_seconds: u32,
}
