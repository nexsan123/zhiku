mod commands;
mod errors;
mod models;
mod services;

use tauri::Manager;

pub fn run() {
    env_logger::init();

    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![
            commands::news::get_news,
            commands::news::get_news_count,
            commands::news::fetch_rss,
            commands::macro_data::get_macro_data,
            commands::macro_data::fetch_fred,
            commands::api_status::get_api_status,
            commands::api_status::update_api_status,
            commands::market_data::get_market_data,
            commands::market_data::fetch_market,
            commands::market_data::get_market_radar,
            commands::ai::summarize_pending_news,
            commands::ai::get_ai_brief,
            commands::ai::get_cycle_indicators,
            commands::ai::get_cycle_reasoning,
            commands::ai::trigger_cycle_reasoning,
            commands::shell::open_url,
        ])
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|e| format!("Failed to resolve app data dir: {}", e))?;

            // Ensure the data directory exists
            std::fs::create_dir_all(&app_data_dir)
                .map_err(|e| format!("Failed to create app data dir: {}", e))?;

            let db_path = app_data_dir.join("zhiku.db");

            // Initialize database synchronously in setup (setup closure is sync)
            let pool = tauri::async_runtime::block_on(async {
                services::db::init_database(db_path).await
            })
            .map_err(|e| format!("Database initialization failed: {}", e))?;

            // Make the pool available to all commands via State
            app.manage(pool.clone());

            // Phase 5: Initialize market_context.db (shared with QuantTerminal)
            let mc_db_path = app_data_dir.join("market_context.db");
            let mc_pool = tauri::async_runtime::block_on(async {
                services::market_context::init_market_context_db(&mc_db_path).await
            })
            .map_err(|e| format!("market_context.db init failed: {}", e))?;
            app.manage(mc_pool.clone());

            // Phase 5: QuantTerminal integration servers
            services::qt_rest::start_rest_server(pool.clone());
            let ws_broadcaster = services::qt_ws::start_ws_server();
            app.manage(ws_broadcaster);

            // Start SmartPollLoop — background data fetching (RT-001: tauri::async_runtime::spawn)
            let app_handle = app.handle().clone();
            services::poll_loop::start_poll_loop(
                app_handle,
                pool,
                mc_pool,
                services::poll_loop::PollConfig::default(),
            );

            log::info!("Phase 2+5 data engine initialized + SmartPollLoop + REST :9601 + WS :9600 started");
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| {
            eprintln!("Fatal: Tauri application error: {}", e);
            std::process::exit(1);
        });
}
