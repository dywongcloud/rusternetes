use anyhow::{Context, Result};
use reqwest::{Client, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;

pub struct ApiClient {
    base_url: String,
    client: Client,
    token: Option<String>,
}

#[derive(Debug)]
pub enum GetError {
    NotFound,
    Other(anyhow::Error),
}

impl std::fmt::Display for GetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GetError::NotFound => write!(f, "Resource not found"),
            GetError::Other(e) => write!(f, "{}", e),
        }
    }
}

impl std::error::Error for GetError {}

impl ApiClient {
    pub fn new(base_url: &str, insecure_skip_tls_verify: bool, token: Option<String>) -> Result<Self> {
        let client = if insecure_skip_tls_verify {
            Client::builder()
                .danger_accept_invalid_certs(true)
                .build()
                .context("Failed to build HTTP client")?
        } else {
            Client::new()
        };

        Ok(Self {
            base_url: base_url.to_string(),
            client,
            token,
        })
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, GetError> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.get(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .map_err(|e| GetError::Other(anyhow::anyhow!("Failed to send GET request: {}", e)))?;

        let status = response.status();

        if status == StatusCode::NOT_FOUND {
            return Err(GetError::NotFound);
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(GetError::Other(anyhow::anyhow!("Request failed with status {}: {}", status, body)));
        }

        response.json().await.map_err(|e| GetError::Other(anyhow::anyhow!("Failed to parse response: {}", e)))
    }

    pub async fn post<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.post(&url).json(body);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to send POST request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, body);
        }

        response.json().await.context("Failed to parse response")
    }

    pub async fn put<T: Serialize, R: DeserializeOwned>(&self, path: &str, body: &T) -> Result<R> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.put(&url).json(body);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to send PUT request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, body);
        }

        response.json().await.context("Failed to parse response")
    }

    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = format!("{}{}", self.base_url, path);
        let mut request = self.client.delete(&url);

        if let Some(ref token) = self.token {
            request = request.header("Authorization", format!("Bearer {}", token));
        }

        let response = request
            .send()
            .await
            .context("Failed to send DELETE request")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Request failed with status {}: {}", status, body);
        }

        Ok(())
    }
}
