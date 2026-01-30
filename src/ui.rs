use crate::logger;
use crate::ai_config::{AIConfig, AIHandler};
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Receiver;
use crate::tabs::about_tab;
use crate::tabs::ai_ui_tab::AIConfigurationUI;
use crate::tabs::clear_tab::ClearTabState;

pub struct AppDataCleaner {
    // 标签页状态
    current_tab: String,             // 当前选中的标签页
    show_about_window: bool,

    // 日志相关字段
    is_logging_enabled: bool,
    previous_logging_state: bool,

    // 主题相关字段
    dark_mode: bool,                 // 深色模式开关

    // 清理标签页状态
    clear_tab: ClearTabState,

    // AI UI标签页
    ai_ui: AIConfigurationUI,
    ai_rx: Option<Receiver<(String, String, String)>>, // 添加 AI 响应接收器

    // 自定义位置相关
    custom_locations: Option<Vec<(String, String)>>, // (name, path)
    show_custom_location_window: bool,
}

impl Default for AppDataCleaner {
    fn default() -> Self {
        let (ai_tx, ai_rx) = std::sync::mpsc::channel();

        // 加载AI配置
        let ai_config = match AIConfig::load_from_file("folders_description.yaml") {
            Ok(config) => {
                logger::log_info("已成功加载AI配置文件");
                config
            }
            Err(_) => {
                logger::log_info("未找到配置文件，使用默认配置");
                AIConfig::default()
            }
        };

        // 创建 AIHandler 并包装在 Arc<Mutex<>> 中
        let ai_handler = Arc::new(Mutex::new(AIHandler::new(
            ai_config.clone(),
            Some(ai_tx.clone()),
        )));

        let ai_ui = AIConfigurationUI::new(ai_config.clone(), ai_handler.clone());

        // 创建清理标签页状态
        let clear_tab = ClearTabState::default();

        Self {
            current_tab: "主页".to_string(),
            show_about_window: false,
            is_logging_enabled: false,
            previous_logging_state: false,
            dark_mode: true,
            clear_tab,
            ai_ui,
            ai_rx: Some(ai_rx),
            custom_locations: Some(vec![]),
            show_custom_location_window: false,
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

    fn show_top_menu(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {  
                // 左侧标签页和选项
                ui.selectable_value(&mut self.current_tab, "主页".to_string(), "主页");
            ui.selectable_value(&mut self.current_tab, "关于".to_string(), "关于");
            ui.selectable_value(&mut self.current_tab, "AI配置".to_string(), "AI配置");
            ui.label("|"); // 添加分隔符
            if ui.button("清理 Temp").clicked() {
                self.clear_tab.clean_temp_directory();
            }
            ui.label("|"); // 添加分隔符
                ui.checkbox(&mut self.is_logging_enabled, "启用日志");

                // 添加一个弹性空间，将后面的内容推到右侧
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // 切换文件夹按钮
                    ui.menu_button("切换文件夹", |ui| {
                        for folder in ["Roaming", "Local", "LocalLow"] {
                            if ui.button(folder).clicked() {
                                self.clear_tab.set_selected_appdata_folder(folder.to_string());
                                ui.close_menu();
                            }
                        }
                    });
                    // 当前目标文件夹显示
                    ui.label(format!("当前目标: {}", self.clear_tab.selected_appdata_folder));
                    
                    ui.separator(); // 分隔符
                    
                    // 动态添加自定义位置
                    if let Some(custom_locations) = &self.custom_locations {
                        for (name, path) in custom_locations {
                            if ui.button(name).clicked() {
                                self.clear_tab.set_selected_appdata_folder(path.clone());
                                ui.close_menu();
                            }
                        }
                    }
                    ui.separator();
                    // 始终在最下方的自定义位置入口
                    if ui.button("自定义位置...").clicked() {
                        self.show_custom_location_window = true;
                        ui.close_menu();
                    }
                });
            });

            ui.separator();
        });
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.setup_custom_fonts(ctx);
        
        // 设置主题
        if self.dark_mode {
            ctx.set_visuals(egui::Visuals::dark());
        } else {
            ctx.set_visuals(egui::Visuals::light());
        }
        
        // 处理日志开关
        if self.is_logging_enabled != self.previous_logging_state {
            logger::init_logger(self.is_logging_enabled); // 初始化日志系统
            if self.is_logging_enabled {
                logger::log_info("日志系统已启用");
            } else {
                logger::log_info("日志系统已禁用");
            }
            self.previous_logging_state = self.is_logging_enabled; // 更新状态
        }
        
        // 处理 AI 响应，忽略不需要的变量
        if let Some(rx) = &self.ai_rx {
            while let Ok((_, folder_name, _)) = rx.try_recv() {
                // 重新加载描述文件以更新显示
                self.clear_tab.update_folder_descriptions();
                // 更新状态
                self.clear_tab.status = Some(format!("已更新 {} 的描述", folder_name));
                // 强制重绘
                ctx.request_repaint();
            }
        }

        // 顶部菜单
        self.show_top_menu(ctx);

        // 主面板 - 根据当前标签页显示不同内容
        egui::CentralPanel::default().show(ctx, |ui| {
            match self.current_tab.as_str() {
                "主页" => self.clear_tab.show(ui),
                "关于" => about_tab::handle_about_tab(ui),
                "AI配置" => self.ai_ui.draw_config_ui(ui),
                _ => self.clear_tab.show(ui),
            }
        });

        // 窗口显示
        self.show_windows(ctx);
    }

    fn show_windows(&mut self, ctx: &egui::Context) {
        // 关于窗口
        if self.show_about_window {
            about_tab::show_about_window(ctx, &mut self.show_about_window);
        }

        // AI配置窗口(使用重构后的AI UI模块)
        self.ai_ui.show(ctx);

        // 移动窗口
        self.clear_tab.move_module.show_move_window(ctx);
    }
}

impl eframe::App for AppDataCleaner {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.update(ctx, frame);
    }
}