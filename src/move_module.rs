use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use egui::Context;
use crate::utils;
use crate::logger;
use eframe::egui;

pub fn show_move_dialog(
    ctx: &Context,
    folder_name: &str,
    source_path: &Path,
    on_confirm: impl FnOnce(PathBuf),
) {
    let mut selected_path = None;

    egui::Window::new("选择目标文件夹").show(ctx, |ui| {
        println!("Window rendered");  // 确保窗口被渲染
        if ui.button("选择目标文件夹").clicked() {
            println!("选择目标文件夹按钮被点击");  // 确保按钮点击事件被捕捉
            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                println!("Selected path: {:?}", path);
                logger::log_info(&format!("Selected path: {:?}", path));
                println!("选择的目标路径: {:?}", path);
                selected_path = Some(path);
            }
        }

        if let Some(target_path) = &selected_path {
            let message = format!(
                "您正在将 {} 移动至 {}，确定操作？\n这可能导致 UWP 程序异常！",
                source_path.display(),
                target_path.display()
            );
            if ui.button("确认").clicked() {
                println!("确认按钮被点击");  // 确保确认按钮点击事件
                on_confirm(target_path.clone());
                ui.close_menu();
                println!("Move confirmed to {:?}", target_path); // 确认动作
            }
            if ui.button("取消").clicked() {
                println!("取消按钮被点击");  // 确保取消按钮点击事件
                ui.close_menu();
            }
            ui.label(&message);
        }
    });
}

pub fn move_folder(
    source: &Path,
    target: &Path,
    on_progress: &dyn Fn(f64), // 使用引用的动态函数类型
) -> io::Result<()> {
    println!("Starting folder move from {:?} to {:?}", source, target);
    logger::log_info(&format!("Starting folder move from {:?} to {:?}", source, target));
    let entries: Vec<_> = fs::read_dir(source)?.collect::<Result<_, _>>()?;
    let total_files = entries.len();
    let mut copied_files = 0;

    fs::create_dir_all(target).expect("Failed to create target directory");

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        println!("Processing entry: {:?}", entry.path());

        if file_type.is_dir() {
            move_folder(&source_path, &target_path, on_progress)?; // 递归移动子目录
        } else {
            fs::copy(&source_path, &target_path)?; // 复制文件
        }

        copied_files += 1;
        on_progress((copied_files as f64) / (total_files as f64));
    }

    Ok(())
}

pub fn verify_and_create_symlink(source: &Path, target: &Path) -> io::Result<()> {
    if !utils::compare_dirs_hash(source, target)? {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "源文件夹与目标文件夹哈希值不匹配",
        ));
    }

    fs::remove_dir_all(source)?;

    let output = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/D", &source.display().to_string(), &target.display().to_string()])
        .output()?;

    let output_str = String::from_utf8_lossy(&output.stdout);
    if output_str.contains("<<===>>") {
        Ok(())
    } else {
        Err(io::Error::new(
            io::ErrorKind::Other,
            "创建符号链接失败",
        ))
    }
}
