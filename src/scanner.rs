// src/scanner.rs
use std::env;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::thread;
use std::{fs, path::PathBuf};

use crate::logger;
use dirs_next as dirs; // 引入日志模块
use windows_sys::Win32::Foundation::BOOL;
use windows_sys::Win32::Storage::FileSystem::{
    Everything_GetResultFileName, Everything_GetResultSize, Everything_QueryW, Everything_SetSearchW,
    Everything_SetSort, EVERYTHING_SORT_SIZE_DESCENDING,
};

pub fn scan_appdata(tx: Sender<(String, u64)>, folder_type: &str) {
    println!("开始扫描 {} 类型的文件夹", folder_type);
    // 记录日志
    logger::log_info(&format!("开始扫描 {} 类型的文件夹", folder_type));

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
        thread::spawn(move || {
            // 使用 Everything 进行扫描
            if let Some(results) = scan_with_everything(&appdata_dir) {
                for (folder_name, size) in results {
                    tx.send((folder_name, size)).unwrap();
                }
            }
            // 发送一个特殊标志，表示扫描完成
            tx.send(("__SCAN_COMPLETE__".to_string(), 0)).unwrap();
        });
    }
}

// 使用 Everything 进行搜索
fn scan_with_everything(appdata_dir: &Path) -> Option<Vec<(String, u64)>> {
    let search_path = format!("{}\\*", appdata_dir.to_string_lossy());
    let search_path_wide: Vec<u16> = search_path.encode_utf16().chain(Some(0)).collect();

    unsafe {
        // 设置搜索字符串
        Everything_SetSearchW(search_path_wide.as_ptr());
        // 设置排序方式，按大小降序
        Everything_SetSort(EVERYTHING_SORT_SIZE_DESCENDING);
        // 执行搜索
        if Everything_QueryW(BOOL(1)) == 0 {
            eprintln!("Everything query failed");
            return None;
        }

        let mut results = Vec::new();
        let mut index = 0;
        loop {
            let file_name_ptr = Everything_GetResultFileName(index);
            if file_name_ptr.is_null() {
                break;
            }
            let file_name = std::ffi::CStr::from_ptr(file_name_ptr as *const i8)
                .to_string_lossy()
                .to_string();
            let file_size = Everything_GetResultSize(index);

            results.push((file_name, file_size));
            index += 1;
        }

        Some(results)
    }
}

// 计算文件夹的总大小（递归），此函数暂时保留，以备非 NTFS 分区使用
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
