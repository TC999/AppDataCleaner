#[derive(Clone)]
pub struct Stats {
    pub cleaned_folders_count: u64,
    pub total_cleaned_size: u64,
}

impl Stats {
    pub fn new() -> Self {
        Self {
            cleaned_folders_count: 0,
            total_cleaned_size: 0,
        }
    }

    pub fn update_stats(&mut self, folder_size: u64) {
        self.cleaned_folders_count += 1;
        self.total_cleaned_size += folder_size;
    }

    pub fn report(&self) -> String {
        format!(
            "已清理文件夹数量: {}\n总清理大小: {} 字节",
            self.cleaned_folders_count, self.total_cleaned_size
        )
    }
}
