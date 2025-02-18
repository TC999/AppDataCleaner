use crate::about;
use crate::confirmation;
use crate::delete;
use crate::ignore;
use crate::logger; // 导入 logger 模块
use crate::move_module; // 导入移动模块
use crate::open;
use crate::scanner;
use crate::utils;
use crate::yaml_loader::{load_folder_descriptions, FolderDescriptions};
use eframe::egui::{self, Grid, ScrollArea};
use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};

pub struct AppDataCleaner {
    is_scanning: bool,
    current_folder: Option<String>,
    folder_data: Vec<(String, u64)>,
    show_about_window: bool,                // 确保字段存在
    confirm_delete: Option<(String, bool)>, // 保存要确认删除的文件夹状态
    selected_appdata_folder: String,        // 新增字段
    tx: Option<Sender<(String, u64)>>,
    rx: Option<Receiver<(String, u64)>>,
    is_logging_enabled: bool,             // 控制日志是否启用
    previous_logging_state: bool,         // 记录上一次日志启用状态
    ignored_folders: HashSet<String>,     // 忽略文件夹集合
    move_module: move_module::MoveModule, // 移动模块实例
    folder_descriptions: Option<FolderDescriptions>,
    yaml_error_logged: bool,        // 新增字段，用于标记是否已经记录过错误
    status: Option<String>,         // 添加 status 字段
    sort_criterion: Option<String>, // 新增字段，排序标准 "name" 或 "size"
    sort_order: Option<String>,     // 新增字段，排序顺序 "asc" 或 "desc"
    total_size: u64,                // 新增字段，总大小
}

impl Default for AppDataCleaner {
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            is_scanning: false,
            current_folder: None,
            folder_data: vec![],
            show_about_window: false,                       // 默认值
            confirm_delete: None,                           // 初始化为 None
            selected_appdata_folder: "Roaming".to_string(), // 默认值为 Roaming
            tx: Some(tx),
            rx: Some(rx),
            is_logging_enabled: false,     // 默认禁用日志
            previous_logging_state: false, // 初始时假定日志系统未启用
            ignored_folders: ignore::load_ignored_folders(),
            move_module: Default::default(),
            folder_descriptions: None,
            yaml_error_logged: false,           // 初始时假定未记录过错误
            status: Some("未扫描".to_string()), // 初始化为 "未扫描"
            sort_criterion: None,               // 初始化为 None
            sort_order: None,                   // 初始化为 None
            total_size: 0,                      // 初始化为 0
        }
    }
}

impl AppDataCleaner {
    fn setup_custom_fonts(&self, ctx: &egui::Context) {
        use eframe::egui::{FontData, FontDefinitions};

        let mut fonts = FontDefinitions::default();

        fonts.font_data.insert(
            "custom_font".to_owned(),
            FontData::from_static(include_bytes!("../assets/SourceHanSansCN-Regular.otf")),
        );

        fonts.families.insert(
            egui::FontFamily::Proportional,
            vec!["custom_font".to_owned()],
        );
        fonts
            .families
            .insert(egui::FontFamily::Monospace, vec!["custom_font".to_owned()]);

        ctx.set_fonts(fonts);
    }
}

impl eframe::App for AppDataCleaner {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.setup_custom_fonts(ctx);

        // 加载描述文件
        if self.folder_descriptions.is_none() {
            self.folder_descriptions =
                load_folder_descriptions("folders_description.yaml", &mut self.yaml_error_logged);
        }

        if self.is_logging_enabled != self.previous_logging_state {
            logger::init_logger(self.is_logging_enabled); // 初始化日志系统
            if self.is_logging_enabled {
                logger::log_info("日志系统已启用");
            } else {
                logger::log_info("日志系统已禁用");
            }
            self.previous_logging_state = self.is_logging_enabled; // 更新状态
        }

        // 删除确认弹窗逻辑
        confirmation::handle_delete_confirmation(
            ctx,
            &mut self.confirm_delete,
            &self.selected_appdata_folder,
            &mut self.status,
            &mut self.folder_data,
        ); // 传递 folder_data

        // 顶部菜单
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            if ui.button("关于").clicked() {
                self.show_about_window = true; // 打开关于窗口
                ui.close_menu();
            }

            ui.separator();
            ui.checkbox(&mut self.is_logging_enabled, "启用日志");

            ui.menu_button("切换文件夹", |ui| {
                for folder in ["Roaming", "Local", "LocalLow"] {
                    if ui.button(folder).clicked() {
                        self.selected_appdata_folder = folder.to_string();
                        self.folder_data.clear();
                        self.is_scanning = false;
                        self.status = Some("未扫描".to_string()); // 更新状态为 "未扫描"
                        ui.close_menu();
                    }
                }
            });
            ui.label(format!("当前目标: {}", self.selected_appdata_folder));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("立即扫描").clicked() && !self.is_scanning {
                self.is_scanning = true;
                self.folder_data.clear();
                self.status = Some("扫描中...".to_string()); // 更新状态为 "扫描中..."

                let tx = self.tx.clone().unwrap();
                let folder_type = self.selected_appdata_folder.clone();

                scanner::scan_appdata(tx, &folder_type);
            }

            if let Some(rx) = &self.rx {
                while let Ok((folder, size)) = rx.try_recv() {
                    // 检查是否接收到扫描完成标志
                    if folder == "__SCAN_COMPLETE__" {
                        self.is_scanning = false;
                        self.status = Some("扫描完成".to_string()); // 更新状态为 "扫描完成"
                    } else {
                        self.folder_data.push((folder, size));
                    }
                }
            }

            // 显示状态
            if let Some(status) = &self.status {
                ui.label(status);
            }

            // 添加排序按钮
            ui.menu_button("排序", |ui| {
                if ui.button("名称正序").clicked() {
                    self.sort_criterion = Some("name".to_string());
                    self.sort_order = Some("asc".to_string());
                }
                if ui.button("大小正序").clicked() {
                    self.sort_criterion = Some("size".to_string());
                    self.sort_order = Some("asc".to_string());
                }
                if ui.button("名称倒序").clicked() {
                    self.sort_criterion = Some("name".to_string());
                    self.sort_order = Some("desc".to_string());
                }
                if ui.button("大小倒序").clicked() {
                    self.sort_criterion = Some("size".to_string());
                    self.sort_order = Some("desc".to_string());
                }
            });

            // 计算总大小
            self.total_size = self.folder_data.iter().map(|(_, size)| size).sum();

            // 显示总大小
            ui.label(format!("总大小: {}", utils::format_size(self.total_size)));

            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("folders_table").striped(true).show(ui, |ui| {
                    ui.label("文件夹");
                    ui.label("大小");
                    ui.label("描述");
                    ui.label("操作");
                    ui.end_row();

                    if let Some(criterion) = &self.sort_criterion {
                        self.folder_data.sort_by(|a, b| {
                            if *criterion == "name" {
                                if self.sort_order == Some("asc".to_string()) {
                                    a.0.cmp(&b.0)
                                } else {
                                    b.0.cmp(&a.0)
                                }
                            } else {
                                if self.sort_order == Some("asc".to_string()) {
                                    a.1.cmp(&b.1)
                                } else {
                                    b.1.cmp(&a.1)
                                }
                            }
                        });
                    }

                    for (folder, size) in &self.folder_data {
                        if self.ignored_folders.contains(folder) {
                            ui.add_enabled(
                                false,
                                egui::Label::new(
                                    egui::RichText::new(folder).color(egui::Color32::GRAY),
                                ),
                            );
                        } else {
                            ui.label(folder);
                        }
                        ui.label(utils::format_size(*size));

                        // 读取描述信息并显示
                        let description = self.folder_descriptions.as_ref().and_then(|desc| {
                            desc.get_description(folder, &self.selected_appdata_folder)
                        });
                        if let Some(desc) = description {
                            ui.label(desc);
                        } else {
                            ui.label("无描述");
                        }

                        if !self.ignored_folders.contains(folder) {
                            if ui.button("彻底删除").clicked() {
                                self.confirm_delete = Some((folder.clone(), false));
                                self.status = None; // 每次点击"彻底删除"时清除状态
                            }
                            if ui.button("移动").clicked() {
                                self.move_module.show_window = true;
                                self.move_module.folder_name = folder.clone();
                            }
                            if ui.button("忽略").clicked() {
                                self.ignored_folders.insert(folder.clone());
                                ignore::save_ignored_folders(&self.ignored_folders);
                                logger::log_info(&format!("文件夹 '{}' 已被忽略", folder));
                            }
                        } else {
                            ui.add_enabled(false, |ui: &mut egui::Ui| {
                                let response1 = ui.button("彻底删除");
                                let response2 = ui.button("移动");
                                let response3 = ui.button("忽略");
                                response1 | response2 | response3 // 返回合并的 Response
                            });
                        }
                        if ui.button("打开").clicked() {
                            if let Some(base_path) =
                                utils::get_appdata_dir(&self.selected_appdata_folder)
                            {
                                let full_path = base_path.join(folder);
                                if let Err(err) = open::open_folder(&full_path) {
                                    logger::log_error(&format!("无法打开文件夹: {}", err));
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
            });
        });

        // 关于窗口
        if self.show_about_window {
            about::show_about_window(ctx, &mut self.show_about_window);
        }

        // 显示移动窗口
        self.move_module.show_move_window(ctx);
    }
}
