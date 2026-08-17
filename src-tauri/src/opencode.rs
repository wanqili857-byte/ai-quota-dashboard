//! OpenCode Go (Zen) 额度查询: GET https://opencode.ai/zen/go/v1/usage
//! 返回 rolling(5h)/weekly/monthly 三个窗口的剩余百分比 + 重置时间。

use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct QuotaWindow {
    pub percent: u32,
    pub resets_at: String,
    /// ok | rate-limited | ...
    pub status: String,
}

#[derive(Serialize, Clone)]
pub struct OpenCodeUsage {
    pub rolling: QuotaWindow,
    pub weekly: QuotaWindow,
    pub monthly: QuotaWindow,
}

#[derive(Deserialize)]
struct UsageWindow {
    status: String,
    percent: u32,
    #[serde(rename = "resetsAt")]
    resets_at: String,
}

#[derive(Deserialize)]
struct Usage {
    rolling: UsageWindow,
    weekly: UsageWindow,
    monthly: UsageWindow,
}

#[derive(Deserialize)]
struct UsageResponse {
    usage: Usage,
}

pub async fn fetch_usage() -> Result<OpenCodeUsage, String> {
    let key = crate::keys::opencode_api_key()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .get("https://opencode.ai/zen/go/v1/usage")
        .header("Authorization", format!("Bearer {key}"))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("opencode usage request: {e}"))?
        .error_for_status()
        .map_err(|e| format!("opencode usage http: {e}"))?
        .json::<UsageResponse>()
        .await
        .map_err(|e| format!("opencode usage parse: {e}"))?;

    let u = resp.usage;

    Ok(OpenCodeUsage {
        rolling: QuotaWindow { percent: u.rolling.percent, resets_at: u.rolling.resets_at, status: u.rolling.status },
        weekly: QuotaWindow { percent: u.weekly.percent, resets_at: u.weekly.resets_at, status: u.weekly.status },
        monthly: QuotaWindow { percent: u.monthly.percent, resets_at: u.monthly.resets_at, status: u.monthly.status },
    })
}
