//! CC Switch 本地用量查询: ~/.cc-switch/cc-switch.db 的 proxy_request_logs。
//! created_at 是 epoch 秒; 窗口按本地时区 (Asia/Shanghai) 计算。

use chrono::{Datelike, Duration, Local, TimeZone};
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct WindowStats {
    pub requests: i64,
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cache_read_tokens: i64,
    pub cache_creation_tokens: i64,
    pub cost_usd: f64,
    /// 缓存命中率 (%) — 口径: cache_read / (cache_read + cache_creation + input)
    pub cache_hit_rate: f64,
}

#[derive(Serialize, Clone)]
pub struct TrendPoint {
    pub date: String,
    pub requests: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
}

pub struct CcSwitch {
    db: Connection,
}

impl CcSwitch {
    pub fn open() -> Result<Self, String> {
        let home = std::env::var("HOME").map_err(|_| "HOME not set".to_string())?;
        let path = std::path::Path::new(&home).join(".cc-switch/cc-switch.db");
        let db = Connection::open(&path)
            .map_err(|e| format!("open cc-switch.db: {e}"))?;
        Ok(Self { db })
    }

    /// 查询 [since, until) 秒区间内的请求汇总。
    pub fn window_stats(&self, since: i64, until: i64) -> Result<WindowStats, String> {
        let (requests, input, output, cache_read, cache_creation, cost) = self.db
            .query_row(
                "SELECT COUNT(*),
                        COALESCE(SUM(input_tokens),0),
                        COALESCE(SUM(output_tokens),0),
                        COALESCE(SUM(cache_read_tokens),0),
                        COALESCE(SUM(cache_creation_tokens),0),
                        COALESCE(SUM(total_cost_usd),0)
                 FROM proxy_request_logs
                 WHERE created_at >= ?1 AND created_at < ?2",
                rusqlite::params![since, until],
                |r| Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, i64>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, f64>(5)?,
                )),
            )
            .map_err(|e| format!("window query: {e}"))?;

        let hit_rate = if cache_read + cache_creation + input > 0 {
            cache_read as f64 * 100.0 / (cache_read + cache_creation + input) as f64
        } else {
            0.0
        };

        Ok(WindowStats {
            requests,
            input_tokens: input,
            output_tokens: output,
            cache_read_tokens: cache_read,
            cache_creation_tokens: cache_creation,
            cost_usd: cost,
            cache_hit_rate: hit_rate,
        })
    }

    /// 查询 [since, until) 秒区间内指定模型前缀的请求数 + 输入 token (用于按服务分类行)。
    pub fn provider_window(
        &self,
        since: i64,
        until: i64,
        model_like: &str,
    ) -> Result<(i64, i64), String> {
        self.db
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(input_tokens),0)
                 FROM proxy_request_logs
                 WHERE created_at >= ?1 AND created_at < ?2 AND model LIKE ?3",
                rusqlite::params![since, until, model_like],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| format!("provider query: {e}"))
    }

    /// 按 app_type 过滤 (如 'claude').
    pub fn provider_window_app(
        &self,
        since: i64,
        until: i64,
        app_type: &str,
    ) -> Result<(i64, i64), String> {
        self.db
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(input_tokens),0)
                 FROM proxy_request_logs
                 WHERE created_at >= ?1 AND created_at < ?2 AND app_type = ?3",
                rusqlite::params![since, until, app_type],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| format!("provider app query: {e}"))
    }

    /// 按 providers 表里的 provider 名称模糊匹配, 可排除同时匹配另一个名称的 provider
    /// (如 DeepSeek 需排除 "OpenCode Go DeepSeekv4flash").
    pub fn provider_by_name_ex(
        &self,
        since: i64,
        until: i64,
        name_like: &str,
        name_not_like: &str,
    ) -> Result<(i64, i64), String> {
        self.db
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(l.input_tokens),0)
                 FROM proxy_request_logs l
                 JOIN providers p ON l.provider_id = p.id
                 WHERE l.created_at >= ?1 AND l.created_at < ?2
                   AND p.name LIKE ?3 AND p.name NOT LIKE ?4",
                rusqlite::params![since, until, name_like, name_not_like],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| format!("provider name ex query: {e}"))
    }

    /// 按 providers 表里的 provider 名称模糊匹配 (如 '%OpenCode%' / '%DeepSeek%')。
    pub fn provider_by_name(
        &self,
        since: i64,
        until: i64,
        name_like: &str,
    ) -> Result<(i64, i64), String> {
        self.db
            .query_row(
                "SELECT COUNT(*), COALESCE(SUM(l.input_tokens),0)
                 FROM proxy_request_logs l
                 JOIN providers p ON l.provider_id = p.id
                 WHERE l.created_at >= ?1 AND l.created_at < ?2 AND p.name LIKE ?3",
                rusqlite::params![since, until, name_like],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .map_err(|e| format!("provider name query: {e}"))
    }

    /// 近 days 天按天汇总 (本地日期)。
    pub fn trend(&self, days: i64) -> Result<Vec<TrendPoint>, String> {
        let since = (Local::now() - Duration::days(days)).timestamp();
        let mut stmt = self
            .db
            .prepare(
                "SELECT date(created_at,'unixepoch','localtime') d,
                        COUNT(*),
                        SUM(input_tokens+output_tokens+cache_read_tokens+cache_creation_tokens),
                        SUM(total_cost_usd)
                 FROM proxy_request_logs
                 WHERE created_at >= ?1
                 GROUP BY d ORDER BY d",
            )
            .map_err(|e| format!("trend prepare: {e}"))?;

        let rows = stmt
            .query_map([since], |r| {
                Ok(TrendPoint {
                    date: r.get(0)?,
                    requests: r.get(1)?,
                    total_tokens: r.get::<_, Option<i64>>(2)?.unwrap_or(0),
                    cost_usd: r.get::<_, Option<f64>>(3)?.unwrap_or(0.0),
                })
            })
            .map_err(|e| format!("trend query: {e}"))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| format!("trend collect: {e}"))
    }
}

/// 本地时区窗口边界 (epoch 秒)。
pub fn window_bounds() -> (i64, i64, i64) {
    let now = Local::now();
    let today_start = Local
        .with_ymd_and_hms(now.year(), now.month(), now.day(), 0, 0, 0)
        .single()
        .unwrap_or(now);
    let week_start = today_start - Duration::days(6); // 含今天共 7 天
    let month_start = Local
        .with_ymd_and_hms(now.year(), now.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(today_start);
    (
        today_start.timestamp(),
        week_start.timestamp(),
        month_start.timestamp(),
    )
}
