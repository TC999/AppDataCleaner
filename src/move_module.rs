use native_dialog::FileDialog;
use std::fs;
use std::io;
use std::path::Path;

pub fn move_folder(appdata_folder: &str) -> Result<(), io::Error> {
    // 弹出文件夹选择对话框
    if let Some(target_folder) = FileDialog::new().show_open_single_dir().ok().flatten() {
        println!("选择的目标文件夹: {:?}", target_folder);

        // 构造完整路径
        let source_path = Path::new(appdata_folder);
        let target_path = Path::new(&target_folder);

        // 弹窗确认
        let message = format!(
            "您正在将 {} 移动至 {}，确定继续？这可能导致 UWP 程序异常！",
            source_path.display(),
            target_path.display()
        );

        // 调用确认对话框 (自行实现 `confirmation` 模块)
        if let Some(confirm) = crate::confirmation::show_confirmation(&message) {
            if confirm {
                // 开始复制文件
                println!("开始移动文件夹...");
                fs::create_dir_all(&target_path)?;

                // 递归复制文件夹内容
                for entry in fs::read_dir(source_path)? {
                    let entry = entry?;
                    let file_name = entry.file_name();
                    let target_file = target_path.join(file_name);

                    if entry.file_type()?.is_dir() {
                        fs::create_dir_all(&target_file)?;
                    } else {
                        fs::copy(entry.path(), &target_file)?;
                    }
                }

                // 校验文件哈希（可选）
                // ...

                // 删除原文件夹
                fs::remove_dir_all(&source_path)?;

                // 创建符号链接
                let output = std::process::Command::new("cmd")
                    .args(["/C", "mklink", "/D", source_path.to_str().unwrap(), target_path.to_str().unwrap()])
                    .output()?;

                if output.status.success() {
                    println!(
                        "为 {} <<===>> {} 创建的符号链接",
                        source_path.display(),
                        target_path.display()
                    );
                } else {
                    eprintln!("符号链接创建失败: {:?}", output.stderr);
                }

                println!("文件夹移动完成！");
            } else {
                println!("用户取消了操作");
            }
        }
    } else {
        println!("未选择目标文件夹");
    }

    Ok(())
}
