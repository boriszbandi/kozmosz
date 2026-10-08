//! Google service-account sign-in: a JWT signed with the account's private key (RS256) is
//! exchanged for an access token, which is reused until shortly before it expires.

use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use aws_lc_rs::{
    rand::SystemRandom,
    signature::{RsaKeyPair, RSA_PKCS1_SHA256},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, engine::general_purpose::STANDARD, Engine};
use reqwest::Client;
use serde::Deserialize;

const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const SCOPE: &str = "https://www.googleapis.com/auth/drive.readonly";

pub struct ServiceAccount {
    email: String,
    key: RsaKeyPair,
    token: Option<(String, Instant)>,
}

/// The fields we need from the JSON key file Google Cloud gives for a service account.
#[derive(Deserialize)]
struct KeyFile {
    client_email: String,
    private_key: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: u64,
}

impl ServiceAccount {
    /// Reads a service-account JSON key file.
    pub fn from_json(json: &str) -> Result<Self, String> {
        let file: KeyFile = serde_json::from_str(json).map_err(|e| format!("not a service-account key file: {e}"))?;
        let der = pem_body(&file.private_key).ok_or("the key file holds no PKCS#8 private key")?;
        let key = RsaKeyPair::from_pkcs8(&der).map_err(|e| format!("unusable private key: {e}"))?;
        Ok(Self { email: file.client_email, key, token: None })
    }

    pub fn email(&self) -> &str {
        &self.email
    }

    /// A valid access token, from the cache or freshly requested.
    pub async fn token(&mut self, client: &Client) -> Result<String, String> {
        if let Some((token, until)) = &self.token {
            if Instant::now() < *until {
                return Ok(token.clone());
            }
        }
        let assertion = self.assertion()?;
        let body = format!("grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Ajwt-bearer&assertion={assertion}");
        let response = client
            .post(TOKEN_URL)
            .header(reqwest::header::CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(body)
            .timeout(Duration::from_secs(30))
            .send()
            .await
            .map_err(|e| format!("token request failed: {e}"))?;
        if !response.status().is_success() {
            return Err(format!("token request refused: {}", response.status()));
        }
        let token: TokenResponse = response.json().await.map_err(|e| format!("bad token response: {e}"))?;
        // Renew a minute early so a token never expires in the middle of a sync.
        let until = Instant::now() + Duration::from_secs(token.expires_in.saturating_sub(60));
        self.token = Some((token.access_token.clone(), until));
        Ok(token.access_token)
    }

    /// The signed JWT: header.claims.signature, each part base64url without padding.
    fn assertion(&self) -> Result<String, String> {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_err(|e| e.to_string())?.as_secs();
        let header = URL_SAFE_NO_PAD.encode(br#"{"alg":"RS256","typ":"JWT"}"#);
        let claims = serde_json::json!({
            "iss": self.email,
            "scope": SCOPE,
            "aud": TOKEN_URL,
            "iat": now,
            "exp": now + 3600,
        });
        let claims = URL_SAFE_NO_PAD.encode(claims.to_string());
        let message = format!("{header}.{claims}");
        let mut signature = vec![0; self.key.public_modulus_len()];
        self.key
            .sign(&RSA_PKCS1_SHA256, &SystemRandom::new(), message.as_bytes(), &mut signature)
            .map_err(|e| format!("signing failed: {e}"))?;
        Ok(format!("{message}.{}", URL_SAFE_NO_PAD.encode(signature)))
    }
}

/// DER bytes of a "-----BEGIN PRIVATE KEY-----" PEM block.
fn pem_body(pem: &str) -> Option<Vec<u8>> {
    let start = pem.find("-----BEGIN PRIVATE KEY-----")? + "-----BEGIN PRIVATE KEY-----".len();
    let end = pem[start..].find("-----END PRIVATE KEY-----")? + start;
    let b64: String = pem[start..end].chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD.decode(b64).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_bad_key_files() {
        assert!(ServiceAccount::from_json("{}").is_err());
        assert!(ServiceAccount::from_json(r#"{"client_email":"a@b","private_key":"nope"}"#).is_err());
        assert_eq!(pem_body("-----BEGIN PRIVATE KEY-----\nAAEC\n-----END PRIVATE KEY-----\n"), Some(vec![0, 1, 2]));
    }
}
