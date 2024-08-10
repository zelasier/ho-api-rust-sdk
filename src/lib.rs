use std::time::Duration;

use aes::Aes256;
use block_modes::{block_padding::Pkcs7, BlockMode, Cbc};
use chrono::Utc;
use chrono_tz::Asia::Shanghai;
use reqwest::{Client, Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, to_string, Value};
use sha1::{Digest, Sha1};
use uuid::Uuid;

type Aes256Cbc = Cbc<Aes256, Pkcs7>;

#[derive(Debug, Deserialize)]
struct ApiResult {
    data: String,
}

#[derive(Debug)]
pub enum ApiClientError {
    ReqwestError(reqwest::Error),
    SerdeJsonError(serde_json::Error),
    AesError(block_modes::BlockModeError),
    Utf8Error(std::string::FromUtf8Error),
    HexError(hex::FromHexError),
    InvalidConfig(String),
}

impl From<reqwest::Error> for ApiClientError {
    fn from(err: reqwest::Error) -> Self {
        ApiClientError::ReqwestError(err)
    }
}

impl From<serde_json::Error> for ApiClientError {
    fn from(err: serde_json::Error) -> Self {
        ApiClientError::SerdeJsonError(err)
    }
}

impl From<block_modes::BlockModeError> for ApiClientError {
    fn from(err: block_modes::BlockModeError) -> Self {
        ApiClientError::AesError(err)
    }
}

impl From<std::string::FromUtf8Error> for ApiClientError {
    fn from(err: std::string::FromUtf8Error) -> Self {
        ApiClientError::Utf8Error(err)
    }
}

impl From<hex::FromHexError> for ApiClientError {
    fn from(err: hex::FromHexError) -> Self {
        ApiClientError::HexError(err)
    }
}

pub struct ApiClient {
    config: ApiClientConfig,
    cipher: Aes256Cbc,
}

#[derive(Clone)]
pub struct ApiClientConfig {
    pub app_id: String,
    pub app_secret: String,
    pub iv: String,
    pub base_url: String,
    pub content: String,
}

impl ApiClient {
    pub fn new(config: ApiClientConfig) -> Result<Self, ApiClientError> {
        let cipher = Aes256Cbc::new_from_slices(config.app_secret.as_bytes(), config.iv.as_bytes())
            .map_err(|_| ApiClientError::InvalidConfig("AES config error".to_string()))?;
        Ok(Self { config, cipher })
    }

    fn generate_nonce(&self) -> String {
        Uuid::new_v4().to_string()
    }

    fn generate_signature(&self, nonce: &str, timestamp: i64, uri: &str, body: &str) -> String {
        let sign_str = format!(
            "{}{}{}{}{}{}",
            self.config.app_id, nonce, timestamp, uri, body, self.config.app_secret
        );

        let mut hasher = Sha1::default();
        hasher.update(sign_str.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }

    pub async fn send(&self, method: Method, uri: &str, body_option: Option<Value>) -> Result<String, ApiClientError> {
        let nonce = self.generate_nonce();
        let now = Utc::now().with_timezone(&Shanghai).timestamp_millis();
        let body_str = match body_option.clone() {
            Some(body) => to_string(&body)?,
            None => "".to_string(),
        };
        let signature = self.generate_signature(&nonce, now, uri, &body_str);

        let url = format!("{}{}{}", self.config.base_url, self.config.content, uri);

        let client = Client::builder()
            .timeout(Duration::from_secs(100))
            .connect_timeout(Duration::from_secs(100))
            .build()?;

        let mut request = client
            .request(method, &url)
            .header("User-Agent", "H-RUST-SDK-1.0.0")
            .header("HO-APP-ID", &self.config.app_id)
            .header("HO-NONCE", &nonce)
            .header("HO-TIMESTAMP", now.to_string())
            .header("HO-SIGNATURE", &signature);

        if let Some(body) = body_option {
            request = request.json(&json!({ "data": to_string(&body)? }));
        } else {
            request = request.json(&json!({}));
        }

        let response = request.send().await?;
        if response.status() != StatusCode::OK {
            return Err(ApiClientError::ReqwestError(response.error_for_status().unwrap_err()));
        }

        let api_result: ApiResult = response.json().await?;
        let hex_ciphertext = hex::decode(&api_result.data)?;
        let decrypted_data = self.cipher.clone().decrypt_vec(&hex_ciphertext)?;
        let decrypted_str = String::from_utf8(decrypted_data)?;
        Ok(decrypted_str)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;
    use tokio::test;

    use super::*;

    #[test]
    async fn test_send_request() {
        let config = ApiClientConfig {
            app_id: "your app id".to_string(),
            app_secret: "your app secret".to_string(),
            iv: "you app iv".to_string(),
            base_url: "https://server.zelaser.com".to_string(),
            content: "/server/common/api".to_string(),
        };

        let client = ApiClient::new(config).expect("Failed to create API client");

        let body = json!({
            "key": "value",
        });

        match client.send(Method::GET, "/v1/lol/champion/skin?region=cn", Some(body)).await {
            Ok(response) => {
                println!("Response Body: {}", response);
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }
}