//! DeepSeek 开放平台余额查询: GET https://api.deepseek.com/user/balance

use serde::{Deserialize, Serialize};

#[derive(Serialize, Clone)]
pub struct Balance {
    pub currency: String,
    pub total: f64,
    pub granted: f64,
    pub topped_up: f64,
}

#[derive(Deserialize)]
struct BalanceInfo {
    currency: String,
    total_balance: String,
    granted_balance: String,
    topped_up_balance: String,
}

#[derive(Deserialize)]
struct BalanceResponse {
    is_available: bool,
    balance_infos: Vec<BalanceInfo>,
}

pub async fn fetch_balance() -> Result<Balance, String> {
    let key = crate::keys::deepseek_api_key()?;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| format!("http client: {e}"))?;

    let resp = client
        .get("https://api.deepseek.com/user/balance")
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .await
        .map_err(|e| format!("balance request: {e}"))?
        .error_for_status()
        .map_err(|e| format!("balance http: {e}"))?
        .json::<BalanceResponse>()
        .await
        .map_err(|e| format!("balance parse: {e}"))?;

    let info = resp
        .balance_infos
        .into_iter()
        .next()
        .ok_or("no balance info")?;

    Ok(Balance {
        currency: info.currency,
        total: info.total_balance.parse().unwrap_or(0.0),
        granted: info.granted_balance.parse().unwrap_or(0.0),
        topped_up: info.topped_up_balance.parse().unwrap_or(0.0),
    })
}
