// mod about; // 关于界面
mod confirmation; // 确认删除模块
mod delete; // 引入删除模块
mod ignore; // 引入忽略模块
mod logger; // 引入日志模块
mod move_module; // 移动文件夹，使用 mklink 指令
mod open; // 调用资源管理器打开文件夹
mod scanner; // 引入扫盘模块
mod ui; // 引入 ui 模块
mod utils; // 文件夹大小计算模块
mod yaml_loader; // 文件描述
pub mod ai_config;  // 使用 pub 使其可以被其他模块访问
pub mod tabs;  // 添加tabs模块，使其可以被其他模块访问

use eframe::egui::IconData;
use std::sync::Arc;
use image;
use ui::AppDataCleaner;

fn main() -> Result<(), eframe::Error> {
    // 初始化日志
    logger::init_logger(true); // true 表示日志记录到文件，false 表示只输出到控制台

    let mut native_options = eframe::NativeOptions::default();
    let icon_data = include_bytes!("../assets/icon.png");
    let img = image::load_from_memory_with_format(icon_data, image::ImageFormat::Png).unwrap();
    let rgba_data = img.into_rgba8();
    let (w,h)=(rgba_data.width(),rgba_data.height());
    let raw_data: Vec<u8> = rgba_data.into_raw();
    native_options.viewport.icon=Some(Arc::<IconData>::new(IconData { rgba:  raw_data, width: w, height: h }));

    let options = eframe::NativeOptions::default();
    eframe::run_native(
        "AppData 清理器",
        options,
        Box::new(|_| {
            logger::log_info("应用程序启动");
            Ok(Box::new(AppDataCleaner::default()))
        }),
    )
}
