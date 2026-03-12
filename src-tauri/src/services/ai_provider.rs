use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// AI 模型完成请求的结果
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: Option<u32>,
    pub latency_ms: u64,
}

/// AI 模型提供商信息
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelInfo {
    pub provider: String,
    pub model_name: String,
    pub display_name: String,
    pub max_context: usize,
    pub cost_per_1k_input: f64,
    pub cost_per_1k_output: f64,
}

/// 统一的 AI 提供商接口 — Phase B 将让各 client 实现此 trait
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// 提供商名称 (e.g., "ollama", "groq", "claude")
    fn name(&self) -> &str;

    /// 模型信息
    fn model_info(&self) -> AiModelInfo;

    /// 健康检查
    async fn health_check(&self) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    /// 发送补全请求
    async fn complete(
        &self,
        prompt: &str,
        system: Option<&str>,
    ) -> Result<AiResponse, Box<dyn std::error::Error + Send + Sync>>;
}
