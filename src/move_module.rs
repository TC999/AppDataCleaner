use crate::logger;
use dirs_next as dirs;
use eframe::egui;
use native_dialog::FileDialog;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread; // 如果使用的是 `dirs-next`
use walkdir::WalkDir;

pub struct MoveModule {
    pub show_window: bool,
    pub folder_name: String,            // 源文件夹名（相对路径）
    pub selected_path: Option<PathBuf>, // 目标路径
    pub progress: f32,                  // 复制进度
    pub status_message: Option<String>, // 操作状态
    pub receiver: Option<Receiver<ProgressMessage>>, // 非阻塞消息接收器
}

#[derive(Debug, Clone)]
pub enum ProgressMessage {
    Progress(f32, String),              // 进度百分比和状态消息
    HashVerificationStart,              // 开始哈希校验
    HashVerificationProgress(f32),      // 哈希校验进度
    Success(String),                    // 成功完成
    Error(String),                      // 错误消息
}

impl Default for MoveModule {
    fn default() -> Self {
        Self {
            show_window: false,
            folder_name: String::new(),
            selected_path: None,
            progress: 0.0,
            status_message: None,
            receiver: None,
        }
    }
}

impl MoveModule {
    pub fn show_move_window(&mut self, ctx: &egui::Context) {
        let mut receiver = self.receiver.take();
        // 非阻塞地检查进度消息
        if let Some(rx) = receiver.as_ref() {
            let mut should_clear_receiver = false;
            while let Ok(msg) = rx.try_recv() {
                match msg {
                    ProgressMessage::Progress(progress, status) => {
                        self.progress = progress;
                        self.status_message = Some(status);
                        ctx.request_repaint(); // 请求重绘以更新 UI
                    }
                    ProgressMessage::HashVerificationStart => {
                        self.status_message = Some("开始哈希校验...".to_string());
                        ctx.request_repaint();
                    }
                    ProgressMessage::HashVerificationProgress(progress) => {
                        self.progress = progress;
                        self.status_message = Some(format!("哈希校验进度: {:.1}%", progress * 100.0));
                        ctx.request_repaint();
                    }
                    ProgressMessage::Success(msg) => {
                        self.progress = 1.0;
                        self.status_message = Some(msg);
                        //self.receiver = None; // 完成后清除接收器
                        ctx.request_repaint();
                        logger::log_info("文件夹移动操作成功完成");
                    }
                    ProgressMessage::Error(err) => {
                        self.status_message = Some(err.clone());
                        //self.receiver = None; // 错误后清除接收器
                        ctx.request_repaint();
                        logger::log_error(&err);
                    }
                }
            }
            if !should_clear_receiver {
            // 操作未结束，放回 self.receiver
                self.receiver = receiver;
            }
        }

        if self.show_window {
            egui::Window::new("移动文件夹")
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("需要移动的文件夹: {}", self.folder_name));

                    // 显示目标路径选择
                    ui.horizontal(|ui| {
                        ui.label("目标路径:");
                        if let Some(path) = &self.selected_path {
                            ui.label(path.display().to_string());
                        }
                        if ui.button("选择目标路径").clicked() {
                            // 使用文件对话框选择目标路径
                            if let Ok(Some(path)) = FileDialog::new().show_open_single_dir() {
                                self.selected_path = Some(path);
                                println!(
                                    "目标路径选择: {}",
                                    self.selected_path.as_ref().unwrap().display()
                                );
                            }
                        }
                    });

                    // 显示状态信息
                    if let Some(message) = &self.status_message {
                        ui.label(message);
                    }

                    // 显示进度条
                    ui.add(egui::ProgressBar::new(self.progress).show_percentage());

                    // 操作按钮
                    let can_start = self.receiver.is_none(); // 只有在没有正在进行的操作时才能开始
                    ui.horizontal(|ui| {
                        if ui.add_enabled(can_start, egui::Button::new("确定")).clicked() {
                            if let Some(target_path) = &self.selected_path {
                                self.start_move_folder(target_path.clone());
                            } else {
                                self.status_message = Some("请选择目标路径".to_string());
                            }
                        }

                        if ui.add_enabled(can_start, egui::Button::new("取消")).clicked() {
                            self.show_window = false;
                        }
                    });
                });
        }
    }

    fn start_move_folder(&mut self, target_path: PathBuf) {
        // 获取系统 AppData 路径
        let appdata_path = dirs::data_dir()
            .or_else(|| dirs::config_dir()) // 备用获取 Roaming 路径
            .unwrap_or_else(|| PathBuf::from("%appdata%"));

        let source_path = appdata_path.join(&self.folder_name);

        // 调试日志打印完整路径
        println!("完整源文件夹路径: {}", source_path.display());
        logger::log_info(&format!("开始移动文件夹: {} -> {}", source_path.display(), target_path.display()));

        // 验证源文件夹是否存在
        if !source_path.exists() {
            self.status_message = Some(format!("源文件夹不存在: {}", source_path.display()));
            println!("源文件夹不存在: {}", source_path.display());
            logger::log_error(&format!("源文件夹不存在: {}", source_path.display()));
            return;
        }

        let (tx, rx): (Sender<ProgressMessage>, Receiver<ProgressMessage>) = mpsc::channel();
        self.receiver = Some(rx);
        self.progress = 0.0;
        self.status_message = Some("开始移动文件夹...".to_string());

        let folder_name = self.folder_name.clone();
        let target_folder_path = target_path.join(&folder_name);

        // 启动后台线程执行移动逻辑
        thread::spawn(move || {
            println!(
                "开始复制: 从 {} 到 {}",
                source_path.display(),
                target_folder_path.display()
            );

            // 步骤 1: 创建目标目录
            if let Err(err) = fs::create_dir_all(&target_folder_path) {
                let _ = tx.send(ProgressMessage::Error(format!("无法创建目标目录: {}", err)));
                return;
            }

            // 步骤 2: 复制文件夹，显示进度
            if let Err(err) = copy_dir_with_progress(&source_path, &target_folder_path, &tx) {
                let _ = tx.send(ProgressMessage::Error(format!("复制失败: {}", err)));
                return;
            }

            // 步骤 3: 哈希校验
            let _ = tx.send(ProgressMessage::HashVerificationStart);
            
            match verify_directory_hashes(&source_path, &target_folder_path, &tx) {
                Ok(true) => {
                    logger::log_info("哈希校验通过，所有文件完全一致");
                    let _ = tx.send(ProgressMessage::Progress(0.9, "哈希校验通过，开始删除源目录...".to_string()));
                }
                Ok(false) => {
                    let _ = tx.send(ProgressMessage::Error("哈希校验失败！源文件和目标文件不一致，操作已终止".to_string()));
                    return;
                }
                Err(err) => {
                    let _ = tx.send(ProgressMessage::Error(format!("哈希校验出错: {}", err)));
                    return;
                }
            }

            // 步骤 4: 删除原文件夹
            if let Err(err) = fs::remove_dir_all(&source_path) {
                let _ = tx.send(ProgressMessage::Error(format!("删除源目录失败: {}", err)));
                return;
            }

            let _ = tx.send(ProgressMessage::Progress(0.95, "正在创建符号链接...".to_string()));

            // 步骤 5: 创建符号链接
            if cfg!(target_os = "windows") {
                let output = std::process::Command::new("cmd")
                    .args([
                        //"/C",
                        "mklink",
                        "/D",
                        &format!("\"{}\"", source_path.display()),
                        &format!("\"{}\"", target_folder_path.display()),
                    ])
                    .output();

                match output {
                    Ok(output) if output.status.success() => {
                        let success_msg = format!(
                            "移动文件夹操作成功完成！\n源目录: {}\n目标目录: {}\n符号链接已创建",
                            source_path.display(),
                            target_folder_path.display()
                        );
                        logger::log_info(&success_msg);
                        let _ = tx.send(ProgressMessage::Success(success_msg));
                    }
                    Ok(output) => {
                        let err_msg = format!(
                            "移动文件成功，但创建符号链接失败: {}",
                            String::from_utf8_lossy(&output.stderr)
                        );
                        let _ = tx.send(ProgressMessage::Error(err_msg));
                    }
                    Err(err) => {
                        let _ = tx.send(ProgressMessage::Error(format!("符号链接命令执行失败: {}", err)));
                    }
                }
            } else {
                // 非 Windows 系统，尝试创建软链接
                #[cfg(unix)]
                {
                    use std::os::unix::fs::symlink;
                    if let Err(err) = symlink(&target_folder_path, &source_path) {
                        let _ = tx.send(ProgressMessage::Error(format!("创建符号链接失败: {}", err)));
                    } else {
                        let success_msg = format!(
                            "移动文件夹操作成功完成！\n源目录: {}\n目标目录: {}\n符号链接已创建",
                            source_path.display(),
                            target_folder_path.display()
                        );
                        logger::log_info(&success_msg);
                        let _ = tx.send(ProgressMessage::Success(success_msg));
                    }
                }
                
                #[cfg(not(unix))]
                {
                    let _ = tx.send(ProgressMessage::Error("此平台不支持符号链接创建".to_string()));
                }
            }
        });
    }
}

// 带进度回调的目录复制函数
fn copy_dir_with_progress(
    source: &Path,
    target: &Path,
    tx: &Sender<ProgressMessage>,
) -> Result<(), String> {
    // 首先计算总文件数量
    let total_files = count_files_in_directory(source)?;
    let mut copied_files = 0;

    let _ = tx.send(ProgressMessage::Progress(0.0, format!("开始复制，共 {} 个文件...", total_files)));

    copy_dir_recursive(source, target, tx, &mut copied_files, total_files)?;

    let _ = tx.send(ProgressMessage::Progress(0.8, "文件复制完成，准备哈希校验...".to_string()));
    Ok(())
}

// 递归复制目录
fn copy_dir_recursive(
    source: &Path,
    target: &Path,
    tx: &Sender<ProgressMessage>,
    copied_files: &mut usize,
    total_files: usize,
) -> Result<(), String> {
    let entries: Vec<_> = fs::read_dir(source)
        .map_err(|err| format!("无法读取目录 {}: {}", source.display(), err))?
        .collect();

    for entry in entries {
        let entry = entry.map_err(|err| format!("无法读取条目: {}", err))?;
        let file_type = entry
            .file_type()
            .map_err(|err| format!("无法获取文件类型: {}", err))?;

        let src_path = entry.path();
        let dest_path = target.join(entry.file_name());

        if file_type.is_dir() {
            fs::create_dir_all(&dest_path).map_err(|err| format!("无法创建目录 {}: {}", dest_path.display(), err))?;
            copy_dir_recursive(&src_path, &dest_path, tx, copied_files, total_files)?;
        } else {
            println!(
                "复制文件: 从 {} 到 {}",
                src_path.display(),
                dest_path.display()
            );
            
            fs::copy(&src_path, &dest_path).map_err(|err| {
                format!("无法复制文件 {} 到 {}: {}", src_path.display(), dest_path.display(), err)
            })?;

            *copied_files += 1;
            let progress = (*copied_files as f32 / total_files as f32) * 0.8; // 复制阶段占80%
            let _ = tx.send(ProgressMessage::Progress(
                progress,
                format!("已复制 {}/{} 个文件: {}", *copied_files, total_files, src_path.file_name().unwrap_or_default().to_string_lossy())
            ));
        }
    }

    Ok(())
}

// 计算目录中的文件总数
fn count_files_in_directory(dir: &Path) -> Result<usize, String> {
    let mut count = 0;
    for entry in WalkDir::new(dir) {
        let entry = entry.map_err(|err| format!("无法访问文件: {}", err))?;
        if entry.file_type().is_file() {
            count += 1;
        }
    }
    Ok(count)
}

// SHA-256 哈希校验函数
fn verify_directory_hashes(
    source_dir: &Path,
    target_dir: &Path,
    tx: &Sender<ProgressMessage>,
) -> Result<bool, String> {
    // 获取源目录和目标目录的所有文件
    let source_files = collect_all_files(source_dir)?;
    let target_files = collect_all_files(target_dir)?;

    if source_files.len() != target_files.len() {
        return Ok(false);
    }

    let total_files = source_files.len();
    let mut verified_files = 0;

    for (source_file, target_file) in source_files.iter().zip(target_files.iter()) {
        // 计算相对路径以确保对应关系正确
        let source_rel = source_file.strip_prefix(source_dir)
            .map_err(|_| "无法获取源文件相对路径".to_string())?;
        let target_rel = target_file.strip_prefix(target_dir)
            .map_err(|_| "无法获取目标文件相对路径".to_string())?;

        if source_rel != target_rel {
            return Ok(false);
        }

        // 计算文件哈希
        let source_hash = calculate_file_hash(source_file)?;
        let target_hash = calculate_file_hash(target_file)?;

        if source_hash != target_hash {
            logger::log_error(&format!(
                "文件哈希不匹配: {} != {}",
                source_file.display(),
                target_file.display()
            ));
            return Ok(false);
        }

        verified_files += 1;
        let progress = 0.8 + (verified_files as f32 / total_files as f32) * 0.1; // 哈希校验占10%
        let _ = tx.send(ProgressMessage::HashVerificationProgress(progress));
    }

    Ok(true)
}

// 收集目录中的所有文件
fn collect_all_files(dir: &Path) -> Result<Vec<PathBuf>, String> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir).sort_by_file_name() {
        let entry = entry.map_err(|err| format!("无法访问文件: {}", err))?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }
    Ok(files)
}

// 计算单个文件的 SHA-256 哈希
fn calculate_file_hash(file_path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(file_path)
        .map_err(|err| format!("无法打开文件 {}: {}", file_path.display(), err))?;
    
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192]; // 8KB buffer
    
    loop {
        let bytes_read = file.read(&mut buffer)
            .map_err(|err| format!("读取文件 {} 失败: {}", file_path.display(), err))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;

    #[test]
    fn test_calculate_file_hash() {
        // 创建临时文件进行测试
        let test_content = b"Hello, World!";
        let temp_dir = std::env::temp_dir();
        let test_file = temp_dir.join("test_hash_file.txt");
        
        // 写入测试内容
        fs::write(&test_file, test_content).unwrap();
        
        // 计算哈希
        let hash = calculate_file_hash(&test_file).unwrap();
        
        // 验证哈希值（SHA-256 of "Hello, World!")
        let expected_hash = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert_eq!(hash, expected_hash);
        
        // 清理
        fs::remove_file(&test_file).unwrap();
    }

    #[test]
    fn test_count_files_in_directory() {
        // 创建临时目录和文件进行测试
        let temp_dir = std::env::temp_dir().join("test_count_files");
        fs::create_dir_all(&temp_dir).unwrap();
        
        // 创建一些测试文件
        fs::write(temp_dir.join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.join("file2.txt"), "content2").unwrap();
        
        // 创建子目录
        let sub_dir = temp_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("file3.txt"), "content3").unwrap();
        
        // 测试文件计数
        let count = count_files_in_directory(&temp_dir).unwrap();
        assert_eq!(count, 3); // 应该有3个文件
        
        // 清理
        fs::remove_dir_all(&temp_dir).unwrap();
    }

    #[test]
    fn test_collect_all_files() {
        // 创建临时目录和文件进行测试
        let temp_dir = std::env::temp_dir().join("test_collect_files");
        fs::create_dir_all(&temp_dir).unwrap();
        
        // 创建一些测试文件
        fs::write(temp_dir.join("file1.txt"), "content1").unwrap();
        fs::write(temp_dir.join("file2.txt"), "content2").unwrap();
        
        // 创建子目录
        let sub_dir = temp_dir.join("subdir");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::write(sub_dir.join("file3.txt"), "content3").unwrap();
        
        // 测试文件收集
        let files = collect_all_files(&temp_dir).unwrap();
        assert_eq!(files.len(), 3);
        
        // 验证文件路径
        let file_names: Vec<String> = files.iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        assert!(file_names.contains(&"file1.txt".to_string()));
        assert!(file_names.contains(&"file2.txt".to_string()));
        assert!(file_names.contains(&"file3.txt".to_string()));
        
        // 清理
        fs::remove_dir_all(&temp_dir).unwrap();
    }
}
