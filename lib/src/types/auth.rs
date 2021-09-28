use serde::{Deserialize, Serialize};

/// Typed wrapper for AuthorizationCode
#[derive(Debug, Deserialize, Serialize)]
pub struct AuthorizationCode(String);

impl AuthorizationCode {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl Into<String> for &AuthorizationCode {
    fn into(self) -> String {
        self.0.to_owned()
    }
}

/// Typed wrapper for RefreshToken
#[derive(Debug, Deserialize, Serialize)]
pub struct RefreshToken(String);

impl RefreshToken {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl Into<String> for &RefreshToken {
    fn into(self) -> String {
        self.0.to_owned()
    }
}

/// Typed wrapper for AccessToken
#[derive(Debug, Deserialize, Serialize)]
pub struct AccessToken(String);

impl AccessToken {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl Into<String> for &AccessToken {
    fn into(self) -> String {
        self.0.to_owned()
    }
}

/// Typed wrapper for IdToken
#[derive(Debug, Deserialize, Serialize)]
pub struct IdToken(String);

impl IdToken {
    pub fn new(string: impl AsRef<str>) -> Self {
        Self(string.as_ref().to_owned())
    }
}

impl Into<String> for &IdToken {
    fn into(self) -> String {
        self.0.to_owned()
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TokenResponse {
    access_token: AccessToken,
    refresh_token: RefreshToken,
    id_token: IdToken,
    #[serde(rename = "expires_in")]
    expires_in_seconds: u32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccessTokenResponse {
    access_token: AccessToken,
    #[serde(rename = "expires_in")]
    expires_in_seconds: u32,
}
