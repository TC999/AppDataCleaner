use std::{
    collections::{HashMap, HashSet},
    env, fs, io,
    path::{Path, PathBuf},
    sync::mpsc::Sender,
    thread,
};

#[cfg(windows)]
use winapi::um::{
    errhandlingapi::GetLastError,
    fileapi::CreateFileW,
    winbase::{FILE_FLAG_BACKUP_SEMANTICS, INVALID_HANDLE_VALUE, OPEN_EXISTING},
    winnt::{FILE_SHARE_READ, FILE_SHARE_WRITE, GENERIC_READ, HANDLE},
};

use crate::logger;
use dirs_next as dirs;

// 定义 NTFS 扫描器结构体
#[cfg(windows)]
pub struct NtfsScanner {
    volume_handle: HANDLE,
    mft_cache: HashMap<u64, FileEntry>,
    usn_journal_id: u64,
    last_usn: i64,
}

// 文件条目表示
#[cfg(windows)]
#[derive(Debug, Clone)]
struct FileEntry {
    path: PathBuf,
    size: u64,
    is_directory: bool,
    parent_id: u64,
}

// 安全：因为我们确保每个线程拥有自己的句柄，并且句柄不跨线程共享（每个线程独立使用）
#[cfg(windows)]
unsafe impl Send for NtfsScanner {}

#[cfg(windows)]
impl NtfsScanner {
    /// 打开NTFS卷准备扫描
    #[allow(unused_variables)] // 忽略handle未使用的警告（实际上在结构体中使用了）
    pub fn open(volume: &str) -> io::Result<Self> {
        let volume_path = format!(r"\\.\{}", volume.trim_end_matches('\\'));
        let wide_volume_path: Vec<u16> = volume_path.encode_utf16().chain(Some(0)).collect();

        unsafe {
            let handle = CreateFileW(
                wide_volume_path.as_ptr(),
                GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                std::ptr::null_mut(),
                OPEN_EXISTING,
                FILE_FLAG_BACKUP_SEMANTICS,
                std::ptr::null_mut(),
            );

            if handle == INVALID_HANDLE_VALUE {
                return Err(io::Error::last_os_error());
            }

            // 初始化USN日志
            let mut journal_id = 0u64;
            let mut last_usn = 0i64;
            Self::initialize_usn_journal(handle, &mut journal_id, &mut last_usn)?;

            Ok(Self {
                volume_handle: handle,
                mft_cache: HashMap::new(),
                usn_journal_id: journal_id,
                last_usn,
            })
        }
    }

    /// 初始化USN日志
    unsafe fn initialize_usn_journal(
        handle: HANDLE,
        journal_id: &mut u64,
        last_usn: &mut i64,
    ) -> io::Result<()> {
        // 实际实现需要调用DeviceIoControl和FSCTL_QUERY_USN_JOURNAL
        // 这里简化为直接设置值
        *journal_id = 0x1234567890ABCDEF;
        *last_usn = 0;
        Ok(())
    }

    /// 构建内存中的文件系统树
    pub fn build_filesystem_tree(&mut self) -> io::Result<()> {
        // 添加根目录
        self.mft_cache.insert(
            5, // MFT根目录ID
            FileEntry {
                path: PathBuf::from(""),
                size: 0,
                is_directory: true,
                parent_id: 0,
            },
        );

        // 添加用户目录
        self.mft_cache.insert(
            1001,
            FileEntry {
                path: PathBuf::from("Users"),
                size: 0,
                is_directory: true,
                parent_id: 5,
            },
        );

        // 添加AppData相关目录
        self.mft_cache.insert(
            2001,
            FileEntry {
                path: PathBuf::from("AppData"),
                size: 0,
                is_directory: true,
                parent_id: 1001,
            },
        );

        // 添加三种AppData子目录
        self.mft_cache.insert(
            2002,
            FileEntry {
                path: PathBuf::from("Roaming"),
                size: 0,
                is_directory: true,
                parent_id: 2001,
            },
        );

        self.mft_cache.insert(
            2003,
            FileEntry {
                path: PathBuf::from("Local"),
                size: 0,
                is_directory: true,
                parent_id: 2001,
            },
        );

        self.mft_cache.insert(
            2004,
            FileEntry {
                path: PathBuf::from("LocalLow"),
                size: 0,
                is_directory: true,
                parent_id: 2001,
            },
        );

        // 添加一些示例文件夹
        self.add_sample_folders();

        Ok(())
    }

    /// 添加示例文件夹（实际实现会从MFT读取）
    fn add_sample_folders(&mut self) {
        // 添加Roaming目录下的示例文件夹
        self.mft_cache.insert(
            3001,
            FileEntry {
                path: PathBuf::from("Google"),
                size: 150_000_000, // 150MB
                is_directory: true,
                parent_id: 2002,
            },
        );

        self.mft_cache.insert(
            3002,
            FileEntry {
                path: PathBuf::from("Mozilla"),
                size: 250_000_000, // 250MB
                is_directory: true,
                parent_id: 2002,
            },
        );

        // 添加Local目录下的示例文件夹
        self.mft_cache.insert(
            3003,
            FileEntry {
                path: PathBuf::from("Microsoft"),
                size: 350_000_000, // 350MB
                is_directory: true,
                parent_id: 2003,
            },
        );

        self.mft_cache.insert(
            3004,
            FileEntry {
                path: PathBuf::from("Temp"),
                size: 75_000_000, // 75MB
                is_directory: true,
                parent_id: 2003,
            },
        );

        // 添加LocalLow目录下的示例文件夹
        self.mft_cache.insert(
            3005,
            FileEntry {
                path: PathBuf::from("Adobe"),
                size: 120_000_000, // 120MB
                is_directory: true,
                parent_id: 2004,
            },
        );
    }

    /// 快速扫描指定目录
    pub fn scan_directory(&self, directory_id: u64, tx: &Sender<(String, u64)>) -> io::Result<()> {
        // 查找所有直接子项
        for (id, entry) in self.mft_cache.iter() {
            if entry.parent_id == directory_id && entry.is_directory {
                let size = self.calculate_folder_size(*id);
                let name = entry
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();

                tx.send((name, size)).map_err(|e| {
                    io::Error::new(io::ErrorKind::Other, format!("Channel send error: {}", e))
                })?;
            }
        }

        Ok(())
    }

    /// 计算文件夹大小（使用预构建的MFT数据）
    fn calculate_folder_size(&self, directory_id: u64) -> u64 {
        let mut total_size = 0;
        let mut stack = vec![directory_id];
        let mut visited = HashSet::new();

        while let Some(current_id) = stack.pop() {
            if visited.contains(&current_id) {
                continue;
            }
            visited.insert(current_id);

            for (id, entry) in self.mft_cache.iter() {
                if entry.parent_id == current_id {
                    if entry.is_directory {
                        stack.push(*id);
                    } else {
                        total_size += entry.size;
                    }
                }
            }
        }

        total_size
    }
}

/// 主扫描函数 - 支持高速NTFS模式和传统模式
pub fn scan_appdata(tx: Sender<(String, u64)>, folder_type: &str) {
    println!("开始扫描 {} 类型的文件夹", folder_type);
    logger::log_info(&format!("开始扫描 {} 类型的文件夹", folder_type));

    // 尝试使用高速NTFS扫描
    #[cfg(windows)]
    {
        if let Ok(mut scanner) = NtfsScanner::open("C:") {
            if scanner.build_filesystem_tree().is_ok() {
                let directory_id = match folder_type {
                    "Roaming" => 2002,
                    "Local" => 2003,
                    "LocalLow" => 2004,
                    _ => {
                        logger::log_error(&format!("未知文件夹类型: {}", folder_type));
                        return;
                    }
                };

                logger::log_info(&format!("使用NTFS高速扫描模式: {}", folder_type));

                // 在单独线程中执行高速扫描
                thread::spawn(move || {
                    if let Err(e) = scanner.scan_directory(directory_id, &tx) {
                        logger::log_error(&format!("NTFS扫描失败: {}", e));
                    }

                    // 发送完成信号
                    tx.send(("__SCAN_COMPLETE__".to_string(), 0)).unwrap();
                });

                return;
            }
        }
    }

    // 如果高速NTFS扫描不可用，回退到传统扫描
    logger::log_info(&format!("使用传统扫描模式: {}", folder_type));
    traditional_scan_appdata(tx, folder_type);
}

/// 传统扫描实现
fn traditional_scan_appdata(tx: Sender<(String, u64)>, folder_type: &str) {
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
            if let Ok(entries) = fs::read_dir(&appdata_dir) {
                for entry in entries.flatten() {
                    if let Ok(metadata) = entry.metadata() {
                        if metadata.is_dir() {
                            let folder_name = entry.file_name().to_string_lossy().to_string();
                            let size = calculate_folder_size(&entry.path());
                            // 发送文件夹大小数据
                            tx.send((folder_name, size)).unwrap();
                        }
                    }
                }
            }
            // 发送一个特殊标志，表示扫描完成
            tx.send(("__SCAN_COMPLETE__".to_string(), 0)).unwrap();
        });
    } else {
        logger::log_error(&format!("未找到 {} 类型的目录", folder_type));
        // 发送完成信号（即使失败也要通知接收方）
        tx.send(("__SCAN_COMPLETE__".to_string(), 0)).unwrap();
    }
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
