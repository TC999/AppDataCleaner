use crate::about;
use crate::confirmation;
use crate::delete;
use crate::scanner;
use crate::utils;
use crate::logger; // 导入 logger 模块
use crate::ignore;
use eframe::egui::{self, Grid, ScrollArea};
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashSet;

pub struct AppDataCleaner { // 定义数据类型
    is_scanning: bool,
    current_folder: Option<String>,
    folder_data: Vec<(String, u64)>,
    show_about_window: bool,                // 确保字段存在
    confirm_delete: Option<(String, bool)>, // 保存要确认删除的文件夹状态
    selected_appdata_folder: String, // 新增字段
    tx: Option<Sender<(String, u64)>>,
    rx: Option<Receiver<(String, u64)>>,
    is_logging_enabled: bool,  // 控制日志是否启用
    //current_folder_type: String, // 新增字段
    previous_logging_state: bool, // 记录上一次日志启用状态
    ignored_folders: HashSet<String>,  // 忽略文件夹集合
}

impl Default for AppDataCleaner { // 定义变量默认值
    fn default() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            is_scanning: false,
            current_folder: None,
            folder_data: vec![],
            show_about_window: false, // 默认值
            confirm_delete: None,     // 初始化为 None
            selected_appdata_folder: "Roaming".to_string(), // 默认值为 Roaming
            tx: Some(tx),
            rx: Some(rx),
            is_logging_enabled: false,  // 默认禁用日志
            previous_logging_state: false, // 初始时假定日志系统未启用
            ignored_folders: ignore::load_ignored_folders(),
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
        //if let Some((folder_name, _)) = &self.confirm_delete {
        //    let message = format!("确定要彻底删除文件夹 {} 吗？", folder_name);
        //    logger::log_info(&message);
        //    if let Some(confirm) = confirmation::show_confirmation(ctx, &message) {
        //        if confirm {
        //            if let Some(base_path) = utils::get_appdata_dir(&self.selected_appdata_folder) {
        //                let full_path = base_path.join(folder_name);
        //                if let Err(err) = delete::delete_folder(&full_path) {
        //                    eprintln!("Error: {}", err);
        //                    logger::log_error(&format!("Error: {}", err));
        //                }
        //            } else {
        //                eprintln!("无法获取 {} 文件夹路径", self.selected_appdata_folder);
        //                logger::log_error(&format!("无法获取 {} 文件夹路径", self.selected_appdata_folder));
        //            }
        //        }
        //        self.confirm_delete = None; // 清除状态
        //    }
        //}

        // 顶部菜单
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            if ui.button("关于").clicked() {
                self.show_about_window = true; // 打开关于窗口
                ui.close_menu();
            }

            ui.separator();
            ui.checkbox(&mut self.is_logging_enabled, "启用日志");

            ui.menu_button("切换文件夹", |ui| {
                if ui.button("Roaming").clicked() {
                    self.selected_appdata_folder = "Roaming".to_string();
                    self.folder_data.clear(); // 清空扫描结果
                    self.is_scanning = false; // 重置扫描状态
                    ui.close_menu();
                }
                if ui.button("Local").clicked() {
                    self.selected_appdata_folder = "Local".to_string();
                    self.folder_data.clear(); // 清空扫描结果
                    self.is_scanning = false; // 重置扫描状态
                    ui.close_menu();
                }
                if ui.button("LocalLow").clicked() {
                    self.selected_appdata_folder = "LocalLow".to_string();
                    self.folder_data.clear(); // 清空扫描结果
                    self.is_scanning = false; // 重置扫描状态
                    ui.close_menu();
                }
            });
            ui.label(format!("当前目标: {}", self.selected_appdata_folder));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            if ui.button("立即扫描").clicked() && !self.is_scanning {
                self.is_scanning = true;
                self.folder_data.clear();

                let tx = self.tx.clone().unwrap();
                let folder_type = self.selected_appdata_folder.clone();

                scanner::scan_appdata(tx, &folder_type);
            }

            if let Some(rx) = &self.rx {
                while let Ok((folder, size)) = rx.try_recv() {
                    self.folder_data.push((folder, size));
                }
            }

            if self.is_scanning {
                ui.label("扫描中...");
            } else {
                ui.label("扫描完成");
            }

            ScrollArea::vertical().show(ui, |ui| {
                Grid::new("folders_table").striped(true).show(ui, |ui| {
                    ui.label("文件夹");
                    ui.label("大小");
                    ui.label("父级百分比"); // 后续内容，死机暂时不处理
                    ui.label("使用软件");
                    ui.label("操作");
                    ui.end_row();

                    for (folder, size) in &self.folder_data {
                        let is_ignored = self.ignored_folders.contains(folder);
                        if is_ignored {
                            ui.add_enabled(false, egui::Label::new(egui::RichText::new(folder).color(egui::Color32::GRAY)));
                        } else {
                            ui.label(folder);
                        }
                        ui.label(utils::format_size(*size));
                        ui.label("敬请期待"); // 百分比计算，一直死机没解决，代码在dev分支
                        ui.label("敬请期待");

                        if !self.ignored_folders.contains(folder) {
                            if ui.button("彻底删除").clicked() {
                                if let Some((folder_name, _)) = self.confirm_delete.clone() {
                                    confirmation::handle_delete_confirmation(
                                        ctx,
                                        &folder_name,
                                        &self.selected_appdata_folder,
                                        &mut self.confirm_delete,
                                    );
                                
                                    if self.confirm_delete.is_none() {
                                        logger::log_info("确认窗口关闭，操作完成或取消。");
                                    }
                                }
                            }
                            if ui.button("移动").clicked() {
                                // 移动逻辑
                            }
                        } else {
                            ui.add_enabled(false, |ui: &mut egui::Ui| {
                                let response1 = ui.button("彻底删除");
                                let response2 = ui.button("移动");
                                response1 | response2 // 返回合并的 Response
                            });
                        }
                        
                        if ui.button("忽略").clicked() {
                            self.ignored_folders.insert(folder.clone());
                            ignore::save_ignored_folders(&self.ignored_folders);
                            println!("文件夹 '{}' 已被忽略", folder);
                            log::info!("文件夹 '{}' 已被忽略", folder);
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

        // 根据日志开关决定是否记录日志
        //    log::info!("日志系统已启用");
        //if self.is_logging_enabled {
        //} else {
        //    log::info!("日志系统已禁用");
        //}
    }
}
