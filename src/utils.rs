pub fn format_size(size: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.2} {}", size, UNITS[unit])
}

//use std::env;

use dirs_next as dirs;
use std::path::PathBuf;

pub fn get_appdata_dir(folder_type: &str) -> Option<PathBuf> {
    // 如果是自定义路径（以Custom:开头），直接返回路径
    if folder_type.starts_with("Custom:") {
        let path_str = folder_type.strip_prefix("Custom:").unwrap_or("");
        return Some(PathBuf::from(path_str));
    }
    
    match folder_type {
        "Roaming" => dirs::data_dir(),
        "Local" => dirs::cache_dir(),
        "LocalLow" => Some(PathBuf::from("C:/Users/Default/AppData/LocalLow")), 
        _ => None,
    }
}

use std::fs;
use std::path::Path;
use std::env;
use sha2::{Digest, Sha256};

#[allow(dead_code)]
pub fn hash_file(path: &Path) -> Result<String, std::io::Error> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher)?;
    Ok(format!("{:x}", hasher.finalize()))
}

#[allow(dead_code)]
pub fn compare_dirs_hash(source: &Path, target: &Path) -> Result<bool, std::io::Error> {
    let source_hashes: Vec<_> = fs::read_dir(source)?
        .map(|entry| hash_file(&entry?.path()))
        .collect::<Result<_, _>>()?;
    let target_hashes: Vec<_> = fs::read_dir(target)?
        .map(|entry| hash_file(&entry?.path()))
        .collect::<Result<_, _>>()?;

    Ok(source_hashes == target_hashes)
}

// 获取系统Temp目录
pub fn get_temp_dir() -> Option<PathBuf> {
    env::temp_dir().exists().then(|| env::temp_dir())
}

