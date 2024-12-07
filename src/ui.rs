use crate::about;
use crate::confirmation;
use crate::delete;
use crate::scanner;
use crate::utils;
use crate::logger; // 导入 logger 模块
use crate::ignore;
use crate::move_module;
use crate::open;
use eframe::egui::{self, Grid, ScrollArea};
use std::sync::mpsc::{Sender, Receiver};
use std::collections::HashSet;
use std::path::PathBuf; // 确保引入 PathBuf

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
    move_state: Option<MoveState>,
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
            move_state: None,
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
    fn handle_move_operation(&mut self, ctx: &egui::Context, folder: &String) {
        if let Some(base_path) = utils::get_appdata_dir(&self.selected_appdata_folder) {
            let full_path = base_path.join(folder);
            self.move_state = Some(MoveState::Initiate {
                source: full_path.clone(),
                folder: folder.clone(),
            });
            logger::log_info(&format!("准备移动文件夹: {}", folder));
        }
    }
}

enum MoveState {
    Initiate {
        source: PathBuf,
        folder: String,
    },
    Confirm {
        source: PathBuf,
        target: PathBuf,
        folder: String,
    },
    Moving {
        source: PathBuf,
        target: PathBuf,
        folder: String,
        progress: f32,
    },
    Completed {
        source: PathBuf,
        target: PathBuf,
        folder: String,
    },
    Error {
        message: String,
    },
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
        if let Some((folder_name, _)) = &self.confirm_delete {
            let message = format!("确定要彻底删除文件夹 {} 吗？", folder_name);
            logger::log_info(&message);
            if let Some(confirm) = confirmation::show_confirmation(ctx, &message) {
                if confirm {
                    if let Some(base_path) = utils::get_appdata_dir(&self.selected_appdata_folder) {
                        let full_path = base_path.join(folder_name);
                        if let Err(err) = delete::delete_folder(&full_path) {
                            eprintln!("Error: {}", err);
                            logger::log_error(&format!("Error: {}", err));
                        }
                    } else {
                        eprintln!("无法获取 {} 文件夹路径", self.selected_appdata_folder);
                        logger::log_error(&format!("无法获取 {} 文件夹路径", self.selected_appdata_folder));
                    }
                }
                self.confirm_delete = None; // 清除状态
            }
        }

        // 处理移动状态
        if let Some(move_state) = &mut self.move_state {
            match move_state {
                MoveState::Initiate { source, folder } => {
                    // 弹出文件夹选择对话框
                    if let Some(target_folder) = FileDialog::new()
                        .set_location("C:/")
                        .show_open_single_dir()
                        .ok()
                        .flatten()
                    {
                        let message = format!(
                            "您正在将 {} 移动至 {}\n这可能导致 UWP 程序异常！\n确定操作？",
                            source.display(),
                            target_folder.display()
                        );

                        // 设置状态为确认
                        self.move_state = Some(MoveState::Confirm {
                            source: source.clone(),
                            target: target_folder.clone(),
                            folder: folder.clone(),
                        });
                    } else {
                        // 用户取消选择
                        self.move_state = None;
                        logger::log_info("用户取消了移动操作");
                    }
                }
                MoveState::Confirm { source, target, folder } => {
                    // 显示确认弹窗
                    egui::Window::new("确认移动")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label(format!(
                                "您正在将 {} 移动至 {}\n这可能导致 UWP 程序异常！\n确定操作？",
                                source.display(),
                                target.display()
                            ));
                            ui.horizontal(|ui| {
                                if ui.button("确定").clicked() {
                                    // 开始移动
                                    self.move_state = Some(MoveState::Moving {
                                        source: source.clone(),
                                        target: target.clone(),
                                        folder: folder.clone(),
                                        progress: 0.0,
                                    });
                                }
                                if ui.button("取消").clicked() {
                                    self.move_state = None;
                                    logger::log_info("用户取消了移动操作");
                                }
                            });
                        });
                }
                MoveState::Moving { source, target, folder, progress } => {
                    // 执行移动操作（应在后台线程中执行）
                    ui.ctx().request_repaint(); // 请求重绘以显示进度

                    // 此处仅作为示例，实际应使用多线程处理并更新进度
                    // 假设移动操作完成
                    *progress = 1.0;
                    self.move_state = Some(MoveState::Completed {
                        source: source.clone(),
                        target: target.clone(),
                        folder: folder.clone(),
                    });

                    // 删除原文件夹并创建符号链接
                    match move_module::move_folder(ctx, &source, &self.selected_appdata_folder) {
                        Ok(_) => {
                            logger::log_info(&format!("成功移动文件夹: {}", folder));
                            self.move_state = Some(MoveState::Completed {
                                source: source.clone(),
                                target: target.clone(),
                                folder: folder.clone(),
                            });
                        }
                        Err(err) => {
                            logger::log_error(&format!("移动失败: {}", err));
                            self.move_state = Some(MoveState::Error { message: err });
                        }
                    }
                }
                MoveState::Completed { source, target, folder } => {
                    egui::Window::new("移动完成")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label(format!(
                                "为 {} <<===>> {} 创建的符号链接",
                                source.display(),
                                target.display()
                            ));
                            if ui.button("确定").clicked() {
                                self.move_state = None;
                            }
                        });
                }
                MoveState::Error { message } => {
                    egui::Window::new("移动错误")
                        .collapsible(false)
                        .resizable(false)
                        .show(ctx, |ui| {
                            ui.label(format!("移动过程中发生错误: {}", message));
                            if ui.button("确定").clicked() {
                                self.move_state = None;
                            }
                        });
                }
            }
        }

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
                                self.confirm_delete = Some((folder.clone(), false));
                            }
                            if ui.button("移动").clicked() {
                                self.handle_move_operation(ctx, folder);
                            }
                            if ui.button("忽略").clicked() {
                                self.ignored_folders.insert(folder.clone());
                                ignore::save_ignored_folders(&self.ignored_folders);
                                println!("文件夹 '{}' 已被忽略", folder);
                                log::info!("文件夹 '{}' 已被忽略", folder);
                            }
                        } else {
                            ui.add_enabled(false, |ui: &mut egui::Ui| {
                                let response1 = ui.button("彻底删除");
                                let response2 = ui.button("移动");
                                let response3 = ui.button("忽略");
                                response1 | response2 | response3 // 返回合并的 Response
                            });
                        }
                        // 操作区内逻辑，新增 "打开" 按钮
                        if ui.button("打开").clicked() {
                            if let Some(base_path) = utils::get_appdata_dir(&self.selected_appdata_folder) {
                                let full_path = base_path.join(folder);
                                match open::open_folder(&full_path) {
                                    Ok(_) => {
                                        println!("成功打开文件夹: {}", full_path.display());
                                        logger::log_info(&format!("成功打开文件夹: {}", full_path.display()));
                                    }
                                    Err(err) => {
                                        eprintln!("无法打开文件夹: {}", err);
                                        logger::log_error(&format!("无法打开文件夹: {}", err));
                                    }
                                }
                            } else {
                                eprintln!("无法获取 {} 文件夹路径", self.selected_appdata_folder);
                                logger::log_error(&format!("无法获取 {} 文件夹路径", self.selected_appdata_folder));
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

        // 根据日志开关决定是否记录日志
        //    log::info!("日志系统已启用");
        //if self.is_logging_enabled {
        //} else {
        //    log::info!("日志系统已禁用");
        //}
    }
}
