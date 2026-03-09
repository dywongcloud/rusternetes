use crate::error::{Error, Result};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation, Algorithm};
use serde::{Deserialize, Serialize};

/// JWT claims for service account tokens
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceAccountClaims {
    /// Subject (service account name)
    pub sub: String,

    /// Namespace
    pub namespace: String,

    /// Service account UID
    pub uid: String,

    /// Issued at timestamp
    pub iat: i64,

    /// Expiration timestamp
    pub exp: i64,

    /// Issuer
    pub iss: String,

    /// Audience
    pub aud: Vec<String>,
}

impl ServiceAccountClaims {
    pub fn new(
        service_account: String,
        namespace: String,
        uid: String,
        expiration_hours: i64,
    ) -> Self {
        let now = Utc::now();
        let exp = now + Duration::hours(expiration_hours);

        Self {
            sub: format!("system:serviceaccount:{}:{}", namespace, service_account),
            namespace,
            uid,
            iat: now.timestamp(),
            exp: exp.timestamp(),
            iss: "rusternetes-api-server".to_string(),
            aud: vec!["rusternetes".to_string()],
        }
    }
}

/// TokenManager handles JWT token generation and validation
pub struct TokenManager {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl TokenManager {
    /// Create a new TokenManager with a secret key
    pub fn new(secret: &[u8]) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret),
            decoding_key: DecodingKey::from_secret(secret),
        }
    }

    /// Generate a JWT token for a service account
    pub fn generate_token(&self, claims: ServiceAccountClaims) -> Result<String> {
        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(|e| Error::Internal(format!("Failed to generate token: {}", e)))
    }

    /// Validate and decode a JWT token
    pub fn validate_token(&self, token: &str) -> Result<ServiceAccountClaims> {
        let mut validation = Validation::new(Algorithm::HS256);
        validation.set_audience(&["rusternetes"]);
        validation.set_issuer(&["rusternetes-api-server"]);

        decode::<ServiceAccountClaims>(token, &self.decoding_key, &validation)
            .map(|data| data.claims)
            .map_err(|e| Error::Authentication(format!("Invalid token: {}", e)))
    }
}

/// User information extracted from authentication
#[derive(Debug, Clone)]
pub struct UserInfo {
    /// Username
    pub username: String,

    /// UID
    pub uid: String,

    /// Groups the user belongs to
    pub groups: Vec<String>,

    /// Extra attributes
    pub extra: std::collections::HashMap<String, Vec<String>>,
}

impl UserInfo {
    pub fn from_service_account_claims(claims: &ServiceAccountClaims) -> Self {
        Self {
            username: claims.sub.clone(),
            uid: claims.uid.clone(),
            groups: vec![
                "system:serviceaccounts".to_string(),
                format!("system:serviceaccounts:{}", claims.namespace),
            ],
            extra: std::collections::HashMap::new(),
        }
    }

    pub fn anonymous() -> Self {
        Self {
            username: "system:anonymous".to_string(),
            uid: String::new(),
            groups: vec!["system:unauthenticated".to_string()],
            extra: std::collections::HashMap::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_generation_and_validation() {
        let secret = b"test-secret-key";
        let manager = TokenManager::new(secret);

        let claims = ServiceAccountClaims::new(
            "default".to_string(),
            "default".to_string(),
            "test-uid".to_string(),
            24,
        );

        let token = manager.generate_token(claims.clone()).unwrap();
        let validated = manager.validate_token(&token).unwrap();

        assert_eq!(validated.sub, claims.sub);
        assert_eq!(validated.namespace, claims.namespace);
        assert_eq!(validated.uid, claims.uid);
    }

    #[test]
    fn test_invalid_token() {
        let secret = b"test-secret-key";
        let manager = TokenManager::new(secret);

        let result = manager.validate_token("invalid.token.here");
        assert!(result.is_err());
    }
}
