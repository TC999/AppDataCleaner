use eframe::egui;
use crate::{delete, logger, utils};

pub fn handle_delete_confirmation(
    ctx: &egui::Context,
    folder_name: &str,
    selected_appdata_folder: &str,
    confirm_delete: &mut Option<(String, bool)>,
) {
    let message = format!("确定要彻底删除文件夹 {} 吗？", folder_name);
    logger::log_info(&message);

    egui::Window::new("确认操作")
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label(&message);

            ui.horizontal(|ui| {
                if ui.button("确认").clicked() {
                    print!("确认按钮被点击");
                    logger::log_info("确认按钮被点击");
                    // 删除逻辑
                    if let Some(base_path) = utils::get_appdata_dir(selected_appdata_folder) {
                        let full_path = base_path.join(folder_name);
                        if let Err(err) = delete::delete_folder(&full_path) {
                            eprintln!("Error: {}", err);
                            logger::log_error(&format!("Error: {}", err));
                        } else {
                            logger::log_info(&format!("成功删除文件夹 {}", folder_name));
                        }
                    } else {
                        eprintln!("无法获取 {} 文件夹路径", selected_appdata_folder);
                        logger::log_error(&format!("无法获取 {} 文件夹路径", selected_appdata_folder));
                    }
                    *confirm_delete = None; // 清除确认状态
                }
                if ui.button("取消").clicked() {
                    *confirm_delete = None; // 清除状态
                    logger::log_info("用户取消删除操作");
                }
            });
        });
}
