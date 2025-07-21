use std::env;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;
use std::{fs, path::PathBuf};

use crate::database::{Database, FolderRecord, get_default_db_path, database_exists};
use crate::logger;
use chrono::Utc;
use dirs_next as dirs; // 引入日志模块

pub fn scan_appdata(tx: Sender<(String, u64)>, folder_type: &str) {
    println!("开始扫描 {} 类型的文件夹", folder_type);
    // 记录日志
    logger::log_info(&format!("开始扫描 {} 类型的文件夹", folder_type));

    let folder_type = folder_type.to_string();
    
    thread::spawn(move || {
        if let Err(e) = scan_with_database(tx.clone(), &folder_type) {
            logger::log_error(&format!("扫描过程中发生错误: {}", e));
            // 发送扫描完成信号
            let _ = tx.send(("__SCAN_COMPLETE__".to_string(), 0));
        }
    });
}

fn scan_with_database(tx: Sender<(String, u64)>, folder_type: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db_path = get_default_db_path();
    let db_exists = database_exists(&db_path);
    
    // 打开或创建数据库
    let db = Database::new(&db_path)?;
    
    // 如果数据库存在且有该类型的数据，先从数据库加载
    if db_exists && db.has_data_for_type(folder_type)? {
        logger::log_info(&format!("从数据库加载 {} 类型的文件夹数据", folder_type));
        
        // 发送状态消息（使用特殊前缀识别）
        tx.send(("__STATUS__从缓存加载数据...".to_string(), 0))?;
        
        let cached_records = db.get_folders_by_type(folder_type)?;
        for record in &cached_records {
            tx.send((record.folder_name.clone(), record.folder_size))?;
        }
        
        logger::log_info(&format!("从数据库加载了 {} 个文件夹记录", cached_records.len()));
        
        // 发送状态消息
        tx.send(("__STATUS__正在检查文件系统变化...".to_string(), 0))?;
    }
    
    // 执行实际的文件系统扫描
    let fs_scan_results = perform_filesystem_scan(folder_type)?;
    
    if !fs_scan_results.is_empty() {
        // 创建文件夹记录
        let folder_records: Vec<FolderRecord> = fs_scan_results
            .iter()
            .map(|(name, size)| FolderRecord {
                id: None,
                folder_type: folder_type.to_string(),
                folder_name: name.clone(),
                folder_size: *size,
                last_modified: Utc::now(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            })
            .collect();
        
        // 如果数据库中已有数据，比较并更新；否则直接插入
        if db_exists && db.has_data_for_type(folder_type)? {
            // 获取现有记录进行比较
            let existing_records = db.get_folders_by_type(folder_type)?;
            let mut changes_detected = false;
            
            // 检查是否有变化
            for new_record in &folder_records {
                if let Some(existing) = existing_records.iter().find(|r| r.folder_name == new_record.folder_name) {
                    // 如果大小发生变化，发送更新的数据
                    if existing.folder_size != new_record.folder_size {
                        tx.send((new_record.folder_name.clone(), new_record.folder_size))?;
                        changes_detected = true;
                        logger::log_info(&format!(
                            "检测到文件夹 '{}' 大小变化: {} -> {}", 
                            new_record.folder_name,
                            existing.folder_size,
                            new_record.folder_size
                        ));
                    }
                } else {
                    // 新文件夹
                    tx.send((new_record.folder_name.clone(), new_record.folder_size))?;
                    changes_detected = true;
                    logger::log_info(&format!("发现新文件夹: {}", new_record.folder_name));
                }
            }
            
            // 检查是否有文件夹被删除
            let new_folder_names: Vec<String> = folder_records.iter().map(|r| r.folder_name.clone()).collect();
            for existing in &existing_records {
                if !new_folder_names.contains(&existing.folder_name) {
                    changes_detected = true;
                    logger::log_info(&format!("文件夹已被删除: {}", existing.folder_name));
                }
            }
            
            if changes_detected {
                logger::log_info("检测到变化，更新数据库");
            } else {
                logger::log_info("未检测到变化，使用缓存数据");
            }
        } else {
            // 第一次扫描，发送所有结果
            logger::log_info("第一次扫描，创建数据库记录");
            for (name, size) in &fs_scan_results {
                tx.send((name.clone(), *size))?;
            }
        }
        
        // 更新数据库
        db.batch_upsert_folders(&folder_records)?;
        
        // 清理不存在的文件夹记录
        let existing_folder_names: Vec<String> = folder_records.iter().map(|r| r.folder_name.clone()).collect();
        db.remove_missing_folders(folder_type, &existing_folder_names)?;
        
        logger::log_info(&format!("数据库更新完成，共处理 {} 个文件夹", folder_records.len()));
    } else {
        logger::log_info("未找到任何文件夹");
    }
    
    // 发送扫描完成标志
    tx.send(("__SCAN_COMPLETE__".to_string(), 0))?;
    logger::log_info(&format!("{} 类型文件夹扫描完成", folder_type));
    
    Ok(())
}

fn perform_filesystem_scan(folder_type: &str) -> Result<Vec<(String, u64)>, Box<dyn std::error::Error>> {
    let mut results = Vec::new();
    
    // 根据 folder_type 确定要扫描的目录
    let appdata_dir = match folder_type {
        "Roaming" => dirs::data_dir(), // Roaming 目录（跨设备同步的配置）
        "Local" => dirs::cache_dir(),  // Local 目录（本机应用数据）
        "LocalLow" => {
            // 通过 APPDATA 环境变量推导路径
            env::var("APPDATA").ok().and_then(|apdata| {
                let appdata_path = PathBuf::from(apdata);
                // 获取上级目录（即 AppData 文件夹）
                appdata_path
                    .parent()
                    .map(|appdata_dir| appdata_dir.join("LocalLow"))
            })
        }
        // 未知类型返回 None
        _ => None,
    };

    // 如果找到有效的目录，开始扫描
    if let Some(appdata_dir) = appdata_dir {
        if let Ok(entries) = fs::read_dir(&appdata_dir) {
            for entry in entries.flatten() {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_dir() {
                        let folder_name = entry.file_name().to_string_lossy().to_string();
                        let size = calculate_folder_size(&entry.path());
                        results.push((folder_name, size));
                    }
                }
            }
        }
    }
    
    Ok(results)
}

// 计算文件夹的总大小（递归）
fn calculate_folder_size(folder: &Path) -> u64 {
    let mut size = 0;

    // 遍历文件夹中的所有条目
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // 递归计算子文件夹的大小
                size += calculate_folder_size(&path);
            } else if path.is_file() {
                // 计算文件大小
                if let Ok(metadata) = entry.metadata() {
                    size += metadata.len();
                }
            }
        }
    }

    size
}
