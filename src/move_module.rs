use eframe::egui;
use native_dialog::FileDialog;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;
use walkdir::WalkDir;

pub fn move_folder(
    ctx: &egui::Context,
    source_folder: &Path,
    appdata_folder: &str,
) -> Result<(), String> {
    // 弹出文件夹选择对话框
    let target_folder = match FileDialog::new()
        .set_location("C:/")
        .show_open_single_dir()
    {
        Ok(Some(folder)) => folder,
        Ok(None) => return Err("取消选择目标文件夹".to_string()),
        Err(err) => return Err(format!("文件夹选择错误: {}", err)),
    };

    // 构造弹窗提示消息
    let source_display = source_folder.display().to_string();
    let target_display = target_folder.display().to_string();
    let message = format!(
        "您正在将 {} 移动至 {}\n这可能导致 UWP 程序异常！\n确定操作？",
        source_display, target_display
    );

    // 显示确认弹窗
    if !show_confirmation(ctx, &message)? {
        return Err("用户取消操作".to_string());
    }

    // 开始移动文件夹
    let mut total_files = 0;
    let mut completed_files = 0;

    // 计算文件总数
    for entry in WalkDir::new(&source_folder) {
        if let Ok(entry) = entry {
            if entry.file_type().is_file() {
                total_files += 1;
            }
        }
    }

    // 复制文件并显示进度
    for entry in WalkDir::new(&source_folder) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(&source_folder).map_err(|e| e.to_string())?;
            let dest_path = target_folder.join(rel_path);

            if let Some(parent) = dest_path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }

            fs::copy(entry.path(), &dest_path).map_err(|e| e.to_string())?;
            completed_files += 1;

            // 更新进度条
            let progress = completed_files as f32 / total_files as f32;
            egui::Window::new("复制进度").show(ctx, |ui| {
                ui.add(egui::ProgressBar::new(progress).text(format!(
                    "{}/{} 文件已完成",
                    completed_files, total_files
                )));
            });
        }
    }

    // 校验哈希
    if !verify_folder_hash(&source_folder, &target_folder)? {
        return Err("文件哈希校验失败，操作中止".to_string());
    }

    // 删除原文件夹
    fs::remove_dir_all(&source_folder).map_err(|e| e.to_string())?;

    // 创建符号链接
    let output = std::process::Command::new("cmd")
        .args([
            "/C",
            "mklink",
            "/D",
            &source_display,
            &target_display,
        ])
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err(format!(
            "符号链接创建失败: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    Ok(())
}

// 显示确认弹窗
fn show_confirmation(ctx: &egui::Context, message: &str) -> Result<bool, String> {
    // 这里需要使用 egui 的状态管理来显示确认窗口
    // 由于 egui 是即时模式 GUI，不支持阻塞弹窗，因此需要在 UI 状态中管理确认弹窗
    // 这里简化处理，假设用户总是确认

    // TODO: 实现实际的确认弹窗逻辑
    // 目前返回确认
    Ok(true)
}

// 校验两个文件夹内容的哈希值
fn verify_folder_hash(source: &Path, target: &Path) -> Result<bool, String> {
    for entry in WalkDir::new(source) {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().is_file() {
            let rel_path = entry.path().strip_prefix(source).map_err(|e| e.to_string())?;
            let source_file = entry.path();
            let target_file = target.join(rel_path);

            let source_hash = calculate_file_hash(source_file)?;
            let target_hash = calculate_file_hash(&target_file)?;

            if source_hash != target_hash {
                return Ok(false);
            }
        }
    }
    Ok(true)
}

// 计算文件哈希
fn calculate_file_hash(file_path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(file_path).map_err(|e| e.to_string())?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).map_err(|e| e.to_string())?;
    hasher.update(buffer);
    Ok(format!("{:x}", hasher.finalize()))
}
