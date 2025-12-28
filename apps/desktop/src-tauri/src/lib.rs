mod api;
mod app;

use tauri::Manager;

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            let state = app::startup::initialize(app)?;
            app.manage(state);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            api::handlers::analytics::summary,
            api::handlers::analytics::context_latest,
            api::handlers::analytics::context_sessions,
            api::handlers::analytics::context_stats,
            api::handlers::analytics::timeseries,
            api::handlers::analytics::breakdown,
            api::handlers::analytics::breakdown_tokens,
            api::handlers::analytics::breakdown_costs,
            api::handlers::analytics::breakdown_effort_tokens,
            api::handlers::analytics::breakdown_effort_costs,
            api::handlers::analytics::events,
            api::handlers::limits::limits_latest,
            api::handlers::limits::limits_current,
            api::handlers::limits::limits_7d_windows,
            api::handlers::ingest::ingest,
            api::handlers::logs::open_logs_dir,
            api::handlers::pricing::pricing_list,
            api::handlers::pricing::pricing_replace,
            api::handlers::pricing::pricing_recompute,
            api::handlers::settings::settings_get,
            api::handlers::settings::settings_put,
            api::handlers::homes::homes_list,
            api::handlers::homes::homes_create,
            api::handlers::homes::homes_set_active,
            api::handlers::homes::homes_delete,
            api::handlers::homes::homes_clear_data
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
