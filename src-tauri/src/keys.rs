//! 密钥读取: DeepSeek key 在 ~/.env, OpenCode key 在 ~/.local/share/opencode/auth.json。
//! 绝不硬编码进代码。

use std::path::PathBuf;

fn home() -> Result<PathBuf, String> {
    std::env::var("HOME")
        .map(PathBuf::from)
        .map_err(|_| "HOME not set".to_string())
}

pub fn deepseek_api_key() -> Result<String, String> {
    let path = home()?.join(".env");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("read ~/.env: {e}"))?;
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            if k.trim() == "DEEPSEEK_API_KEY" {
                let v = v.trim().trim_matches('"').trim_matches('\'');
                if !v.is_empty() {
                    return Ok(v.to_string());
                }
            }
        }
    }
    Err("DEEPSEEK_API_KEY not found in ~/.env".into())
}

pub fn opencode_api_key() -> Result<String, String> {
    let path = home()?.join(".local/share/opencode/auth.json");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("read opencode auth.json: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("parse opencode auth.json: {e}"))?;

    // 首选结构: { "opencode": { "type": "api", "key": "sk-..." } }
    let obj = v.as_object().ok_or("auth.json 不是 JSON 对象")?;
    for (_, provider) in obj {
        if let Some(key) = provider.get("key").and_then(|k| k.as_str()) {
            if !key.is_empty() {
                return Ok(key.to_string());
            }
        }
    }
    Err("opencode api key not found in auth.json".into())
}
