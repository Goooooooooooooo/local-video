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

/// API 响应的数据结构
#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse {
    /// 响应状态
    pub status: String,
    /// 响应数据
    pub data: Value,
}