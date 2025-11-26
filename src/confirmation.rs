use crate::delete;
use crate::logger;
use crate::stats::Stats;
use crate::stats_logger::StatsLogger;
use crate::utils;
use eframe::egui;
use std::collections::HashSet;

pub fn show_confirmation(
    ctx: &egui::Context,
    message: &str,
    status: &Option<String>,
) -> Option<bool> {
    let mut result = None;

    egui::Window::new("确认操作")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(message);

            // 显示状态信息
            if let Some(status_message) = status {
                ui.label(status_message);
            }

            ui.horizontal(|ui| {
                if ui.button("确认").clicked() {
                    result = Some(true);
                }
                if ui.button("取消").clicked() {
                    result = Some(false);
                    println!("用户取消操作");
                }
            });
        });

    result
}

pub fn handle_delete_confirmation(
    ctx: &egui::Context,
    confirm_delete: &mut Option<(String, bool)> ,
    selected_appdata_folder: &str,
    status: &mut Option<String>,
    folder_data: &mut Vec<(String, u64)>,   // 新增参数
    selected_folders: &mut HashSet<String>, // 传入 selected_folders
    stats: &mut Stats,                      // 新增参数
    stats_logger: &StatsLogger,             // 新增参数
    db: &crate::database::Database,         // 新增参数
) {
    if let Some((folder_name, is_bulk)) = confirm_delete.clone() {
        let message = if is_bulk && folder_name == "BULK_DELETE" {
            "确定要批量删除选中的文件夹吗？".to_string()
        } else {
            format!("确定要彻底删除文件夹 {} 吗？", folder_name)
        };

        if let Some(confirm) = show_confirmation(ctx, &message, status) {
            if confirm {
                if is_bulk && folder_name == "BULK_DELETE" {
                    for folder in selected_folders.iter() {
                        if let Some(base_path) = utils::get_appdata_dir(selected_appdata_folder) {
                            let full_path = base_path.join(folder);
                            if let Err(err) = delete::delete_folder(&full_path, stats, stats_logger, db, selected_appdata_folder)
                            {
                                logger::log_error(&format!("批量删除失败: {}", err));
                            } else {
                                logger::log_info(&format!("已删除文件夹: {}", folder));
                            }
                        }
                    }
                    folder_data.retain(|(folder, _)| !selected_folders.contains(folder)); // 从数据中移除已删除的文件夹
                    selected_folders.clear(); // 清空选定文件夹列表
                    *status = Some("批量删除完成".to_string());
                } else {
                    if let Some(base_path) = utils::get_appdata_dir(selected_appdata_folder) {
                        let full_path = base_path.join(&folder_name);
                        if let Err(err) = delete::delete_folder(&full_path, stats, stats_logger, db, selected_appdata_folder) {
                            logger::log_error(&format!("删除失败: {}", err));
                        } else {
                            logger::log_info(&format!("已删除文件夹: {}", folder_name));
                            folder_data.retain(|(folder, _)| folder != &folder_name);
                        }
                        *status = Some(format!("文件夹 {} 已成功删除", folder_name));
                    }
                }
            }
            *confirm_delete = None; // 重置确认状态
        }
    }
}
