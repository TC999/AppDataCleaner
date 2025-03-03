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
    match folder_type {
        "Roaming" => dirs::data_dir(),
        "Local" => dirs::cache_dir(),
        "LocalLow" => Some(PathBuf::from("C:/Users/Default/AppData/LocalLow")), 
        _ => None,
    }
}

use std::fs;
use std::path::Path;
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

