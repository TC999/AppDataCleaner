// mod about; // 关于界面
pub mod ai_config; // 使用 pub 使其可以被其他模块访问
mod confirmation; // 确认删除模块
mod database; // 数据库模块
mod delete; // 引入删除模块
mod ignore; // 引入忽略模块
mod logger; // 引入日志模块
mod move_module; // 移动文件夹，使用 mklink 指令
mod open; // 调用资源管理器打开文件夹
mod scanner; // 引入扫盘模块
mod stats; // 引入统计模块
mod stats_logger; // 引入统计日志模块
pub mod tabs;
mod ui; // 引入 ui 模块
mod utils; // 文件夹大小计算模块
mod yaml_loader; // 文件描述 // 添加tabs模块，使其可以被其他模块访问

use ui::AppDataCleaner;

fn main() -> Result<(), eframe::Error> {
    // 初始化日志
    logger::init_logger(true); // true 表示日志记录到文件，false 表示只输出到控制台

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
