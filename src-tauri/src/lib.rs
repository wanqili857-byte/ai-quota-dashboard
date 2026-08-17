mod ccswitch;
mod commands;
mod deepseek;
mod keys;
mod opencode;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .setup(|app| {
            // 开机自启动 (打包后生效; dev 模式无 .app 不注册)
            #[cfg(not(debug_assertions))]
            {
                use tauri_plugin_autostart::ManagerExt;
                if let Err(e) = app.autolaunch().enable() {
                    eprintln!("[autostart] enable failed: {e}");
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_balance,
            commands::get_opencode_usage,
            commands::get_usage_stats,
            commands::get_trend,
            commands::get_provider_stats,
            commands::open_topup,
            commands::toggle_pin,
            commands::toggle_collapse,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
