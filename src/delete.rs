use crate::logger;
use crate::stats::Stats;
use crate::stats_logger::StatsLogger; // 引入 StatsLogger 模块
use std::fs;
use std::path::PathBuf;
use crate::database::Database;

pub fn delete_folder(
    folder_path: &PathBuf,
    stats: &mut Stats,
    stats_logger: &StatsLogger,
    db: &Database,
    folder_type: &str,
) -> Result<(), String> {
    let folder_path_str = folder_path.to_string_lossy();
    println!("尝试删除文件夹: {}", folder_path_str);
    logger::log_info(&format!("尝试删除文件夹: {}", folder_path_str));

    if !folder_path.exists() {
        let error_msg = format!("文件夹不存在: {}", folder_path_str);
        println!("{}", error_msg);
        logger::log_error(&error_msg);
        return Err(error_msg);
    }

    if folder_path.is_dir() {
        let folder_size = calculate_folder_size(folder_path); // 计算文件夹大小
        fs::remove_dir_all(folder_path).map_err(|e| {
            let error_msg = format!("删除失败: {} - 错误: {}", folder_path_str, e);
            println!("{}", error_msg);
            logger::log_error(&error_msg);
            error_msg
        })?;
        // 删除数据库记录
        let folder_name = folder_path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if let Err(e) = db.delete_folder_record(folder_type, folder_name) {
            logger::log_error(&format!("数据库记录删除失败: {}", e));
        }
        stats.update_stats(folder_size); // 更新统计数据
        stats_logger.log_stats(stats.cleaned_folders_count, stats.total_cleaned_size); // 记录统计数据到文件
        Ok(())
    } else {
        let error_msg = format!("路径不是目录: {}", folder_path_str);
        println!("{}", error_msg);
        logger::log_error(&error_msg);
        Err(error_msg)
    }
}

// 计算文件夹大小的函数
fn calculate_folder_size(folder: &PathBuf) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                size += calculate_folder_size(&path);
            } else if path.is_file() {
                if let Ok(metadata) = entry.metadata() {
                    size += metadata.len();
                }
            }
        }
    }
    size
}
