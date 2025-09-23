use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

#[derive(Clone)]
pub struct StatsLogger {
    file_path: PathBuf,
}

impl StatsLogger {
    pub fn new(file_path: PathBuf) -> Self {
        Self { file_path }
    }

    pub fn log_stats(&self, cleaned_folders_count: u64, total_cleaned_size: u64) -> Option<()> {
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(true)
            .open(&self.file_path)
            .map_err(|err| {
                eprintln!("无法打开统计日志文件: {}", err);
                err
            })
            .ok()?;

        // 添加时间戳
        let now = chrono::Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S");

        writeln!(
            file,
            "[{}] 已清理文件夹数量: {}, 总清理大小: {} 字节",
            timestamp, cleaned_folders_count, total_cleaned_size
        )
        .map_err(|err| {
            eprintln!("无法写入统计日志文件: {}", err);
            err
        })
        .ok()?;

        Some(())
    }
}
