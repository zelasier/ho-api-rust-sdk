## 禾禾奇趣屋-API-RUST版本SDK

### 引入
```toml
[dependencies]
ho-api-rust-sdk = { git = "https://github.com/Zelaaser/rust-sdk.git" }
```


### 使用示例
```rust
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

        match client.send(Method::GET, "/v1/lol/champion/mate?region=cn", Some(body)).await {
            Ok(response) => {
                println!("Response Body: {}", response);
            }
            Err(e) => eprintln!("Error: {:?}", e),
        }
    }
}
```