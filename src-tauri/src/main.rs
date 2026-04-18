// Animation Asset Manager - Tauri Main
// 入口点：初始化所有Deep Modules并构建命令路由

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Mutex;
use tauri::Manager;

mod models;
mod services;
mod dcc;
mod commands;

use services::{StorageService, DecoderService, EncoderService};

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // 初始化应用数据目录
            let app_data_dir = app.path().app_data_dir()
                .expect("Failed to get app data directory");
            
            std::fs::create_dir_all(&app_data_dir)?;
            
            // 初始化服务
            let storage = StorageService::new(&app_data_dir)
                .expect("Failed to initialize storage");
            let decoder = DecoderService::new(app_data_dir.join("frames"));
            let encoder = EncoderService::new(app_data_dir.join("exports"));
            
            // 管理应用状态
            app.manage(commands::AppState {
                storage: Mutex::new(storage),
                decoder,
                encoder,
            });
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // 素材管理
            commands::create_asset,
            commands::get_asset,
            commands::get_all_assets,
            commands::delete_asset,
            
            // 帧管理
            commands::get_frame_path,
            commands::get_frame_data,
            
            // 标注管理
            commands::create_annotation,
            commands::get_annotations,
            commands::get_annotations_for_frame,
            commands::delete_annotation,
            
            // 导出
            commands::export_asset,
            commands::generate_import_script,
            
            // Sakugabooru
            commands::search_sakugabooru,
            commands::download_sakuga_post,
            commands::get_sakuga_post,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}