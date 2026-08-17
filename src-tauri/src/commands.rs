//! Tauri invoke 命令层。

use serde::Serialize;
use tauri::Window;

use crate::ccswitch;

#[derive(Serialize)]
pub struct UsageStats {
    pub today: ccswitch::WindowStats,
    pub week: ccswitch::WindowStats,
    pub month: ccswitch::WindowStats,
}

#[tauri::command]
pub async fn get_balance() -> Result<crate::deepseek::Balance, String> {
    crate::deepseek::fetch_balance().await
}

#[tauri::command]
pub async fn get_opencode_usage() -> Result<crate::opencode::OpenCodeUsage, String> {
    crate::opencode::fetch_usage().await
}

#[tauri::command]
pub fn get_usage_stats() -> Result<UsageStats, String> {
    let cc = ccswitch::CcSwitch::open()?;
    let now = chrono::Local::now().timestamp();
    let (today_start, week_start, month_start) = ccswitch::window_bounds();
    Ok(UsageStats {
        today: cc.window_stats(today_start, now)?,
        week: cc.window_stats(week_start, now)?,
        month: cc.window_stats(month_start, now)?,
    })
}

#[tauri::command]
pub fn get_trend(days: i64) -> Result<Vec<ccswitch::TrendPoint>, String> {
    ccswitch::CcSwitch::open()?.trend(days)
}

/// 按服务分类的今日统计 (按 cc-switch provider 名称): OpenCode Go (走 opencode.ai) + DeepSeek (直接)。
#[tauri::command]
pub fn get_provider_stats() -> Result<ProviderStats, String> {
    let cc = ccswitch::CcSwitch::open()?;
    let now = chrono::Local::now().timestamp();
    let (today_start, _, _) = ccswitch::window_bounds();
    let (oc_req, oc_in) = cc.provider_by_name(today_start, now, "%OpenCode%")?;
    let (ds_req, ds_in) = cc.provider_by_name_ex(today_start, now, "%DeepSeek%", "%OpenCode%")?;
    Ok(ProviderStats {
        deepseek: ProviderStat { requests: ds_req, input_tokens: ds_in },
        opencode: ProviderStat { requests: oc_req, input_tokens: oc_in },
    })
}

#[derive(Serialize)]
pub struct ProviderStat {
    pub requests: i64,
    pub input_tokens: i64,
}

#[derive(Serialize)]
pub struct ProviderStats {
    pub deepseek: ProviderStat,
    pub opencode: ProviderStat,
}

#[tauri::command]
pub async fn open_topup(platform: String, app: tauri::AppHandle) -> Result<(), String> {
    let url = match platform.as_str() {
        "deepseek" => "https://platform.deepseek.com/top_up",
        "opencode" => "https://opencode.ai/",
        _ => return Err(format!("unknown platform: {platform}")),
    };
    tauri_plugin_opener::OpenerExt::opener(&app)
        .open_url(url, None::<String>)
        .map_err(|e| format!("open url: {e}"))?;
    Ok(())
}

/// 吸顶开关, 返回新的状态。
#[tauri::command]
pub fn toggle_pin(window: Window) -> Result<bool, String> {
    let on = window.is_always_on_top().map_err(|e| e.to_string())?;
    let new = !on;
    window.set_always_on_top(new).map_err(|e| e.to_string())?;
    println!("[pin] was={on} now={new}"); // 诊断日志
    Ok(new)
}

/// 折叠窗口: 展开(360x480) ↔ 折叠(360x44)。返回是否处于折叠态。
/// 用 LogicalSize (逻辑像素, 与 tauri.conf 一致), 避免 Retina 下 PhysicalSize 缩半。
#[tauri::command]
pub fn toggle_collapse(window: Window) -> Result<bool, String> {
    let size = window.outer_size().map_err(|e| e.to_string())?;
    let collapsed = size.height <= 100;
    let (w, h) = if collapsed { (360.0, 480.0) } else { (360.0, 44.0) };
    window
        .set_size(tauri::LogicalSize::new(w, h))
        .map_err(|e| e.to_string())?;
    Ok(!collapsed)
}
