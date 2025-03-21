use crate::logger;
use crate::yaml_loader::{create_default_descriptions, FolderDescriptions};
use eframe::egui;

pub struct YamlEditor {
    // 编辑描述相关状态
    pub edit_description: Option<(String, String)>, // (文件夹名, 当前描述)
    pub folder_descriptions: Option<FolderDescriptions>,
    pub yaml_error_logged: bool,
}

impl Default for YamlEditor {
    fn default() -> Self {
        Self {
            edit_description: None,
            folder_descriptions: None,
            yaml_error_logged: false,
        }
    }
}

impl YamlEditor {
    pub fn new() -> Self {
        Self::default()
    }

    // 初始化编辑器，打开编辑窗口
    pub fn open_description_editor(&mut self, folder: &str, current_description: &str) {
        self.edit_description = Some((folder.to_string(), current_description.to_string()));
    }

    // 处理编辑描述的弹窗
    pub fn handle_edit_description_window(
        &mut self,
        ctx: &egui::Context,
        folder_type: &str,
        status_callback: impl FnOnce(String),
    ) {
        // 判断当前是否有编辑窗口
        if self.edit_description.is_none() {
            return;
        }

        // 获取当前正在编辑的文件夹名和描述
        let (folder_name, current_description) = self.edit_description.clone().unwrap();
        let mut edited_description = current_description.clone(); // 创建一个可编辑的副本
        let mut is_open = true;

        // 创建编辑窗口
        egui::Window::new(format!("编辑 {} 的描述", folder_name))
            .open(&mut is_open)
            .resizable(true)
            .default_width(400.0)
            .show(ctx, |ui| {
                ui.label("描述:");

                // 编辑本地变量
                ui.text_edit_multiline(&mut edited_description);

                ui.horizontal(|ui| {
                    if ui.button("保存").clicked() {
                        is_open = false; // 关闭窗口
                    }

                    if ui.button("取消").clicked() {
                        is_open = false; // 关闭窗口
                        edited_description = current_description.clone(); // 恢复原始描述
                    }
                });
            });

        // 窗口关闭且描述有变更时保存
        if !is_open {
            if edited_description != current_description {
                // 窗口已关闭且有修改，保存描述
                self.save_description(
                    &folder_name,
                    folder_type,
                    edited_description,
                    status_callback,
                );
            }
            self.edit_description = None; // 关闭编辑状态
        } else {
            // 窗口仍然打开，更新编辑中的描述
            self.edit_description = Some((folder_name, edited_description));
        }
    }

    // 保存描述到YAML文件
    pub fn save_description(
        &mut self,
        folder: &str,
        folder_type: &str,
        description: String,
        status_callback: impl FnOnce(String),
    ) {
        // 如果描述为空，则不进行保存
        if description.trim().is_empty() {
            return;
        }

        // 确保folder_descriptions已初始化
        if self.folder_descriptions.is_none() {
            self.folder_descriptions = Some(create_default_descriptions());
        }

        // 更新描述
        if let Some(descriptions) = &mut self.folder_descriptions {
            if let Err(e) = descriptions.update_description(folder, folder_type, description) {
                logger::log_error(&format!("更新描述失败: {}", e));
                status_callback(format!("更新描述失败: {}", e));
                return;
            }

            // 保存到YAML文件
            if let Err(e) = descriptions.save_to_yaml("folders_description.yaml") {
                logger::log_error(&format!("保存描述文件失败: {}", e));
                status_callback(format!("保存描述文件失败: {}", e));
                return;
            }

            logger::log_info(&format!("已更新 {} 的描述", folder));
            status_callback(format!("已更新 {} 的描述", folder));
        }
    }

    // 加载描述文件
    pub fn load_descriptions(&mut self) -> Option<FolderDescriptions> {
        self.folder_descriptions = crate::yaml_loader::load_folder_descriptions(
            "folders_description.yaml",
            &mut self.yaml_error_logged,
        );
        self.folder_descriptions.clone()
    }
}
