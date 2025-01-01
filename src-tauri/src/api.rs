use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 发送 GET 请求获取数据
/// 
/// # 参数
/// * `url` - 请求的目标 URL
/// 
/// # 返回
/// * `Result<String, reqwest::Error>` - 成功返回响应文本，失败返回错误
/// 
/// # 示例
/// ```rust
/// let response = get_data("https://api.example.com/data").await?;
/// println!("Response: {}", response);
/// ```
pub async fn get_data(url: &str) -> Result<String, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let body = response.text().await?;
    Ok(body)
}

/// 发送 POST 请求提交数据
/// 
/// # 参数
/// * `url` - 请求的目标 URL
/// * `data` - 要发送的 JSON 数据
/// 
/// # 返回
/// * `Result<String, reqwest::Error>` - 成功返回响应文本，失败返回错误
/// 
/// # 示例
/// ```rust
/// let data = serde_json::json!({"name": "test"});
/// let response = post_data("https://api.example.com/post", data).await?;
/// ```
pub async fn post_data(url: &str, data: Value) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .post(url)
        .header("Content-Type", "application/json")
        .json(&data)
        .send()
        .await?;
    
    let body = response.text().await?;
    Ok(body)
}

/// 发送带认证令牌的请求
/// 
/// # 参数
/// * `url` - 请求的目标 URL
/// * `token` - 认证令牌
/// 
/// # 返回
/// * `Result<String, reqwest::Error>` - 成功返回响应文本，失败返回错误
/// 
/// # 示例
/// ```rust
/// let response = authenticated_request("https://api.example.com/secure", "your-token").await?;
/// ```
pub async fn authenticated_request(url: &str, token: &str) -> Result<String, reqwest::Error> {
    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;
    
    let body = response.text().await?;
    Ok(body)
}

/// API 响应的数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    /// 响应状态
    pub status: String,
    /// 响应数据
    pub data: Value,
}

/// 获取并解析 JSON 响应数据
/// 
/// # 参数
/// * `url` - 请求的目标 URL
/// 
/// # 返回
/// * `Result<ApiResponse, reqwest::Error>` - 成功返回解析后的响应对象，失败返回错误
/// 
/// # 示例
/// ```rust
/// let response = fetch_json_data("https://api.example.com/json").await?;
/// println!("Status: {}", response.status);
/// ```
pub async fn fetch_json_data(url: &str) -> Result<ApiResponse, reqwest::Error> {
    let response = reqwest::get(url).await?;
    let data: ApiResponse = response.json().await?;
    Ok(data)
} 