use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct FolderDescriptions {
    pub Roaming: HashMap<String, String>,
    pub Local: HashMap<String, String>,
    pub LocalLow: HashMap<String, String>,
}

impl FolderDescriptions {
    pub fn load_from_yaml(file_path: &str) -> Result<Self, String> {
        let path = Path::new(file_path);
        if !path.exists() {
            return Err("YAML 文件未找到".to_string());
        }

        let content = fs::read_to_string(path).map_err(|e| format!("读取 YAML 文件失败: {}", e))?;

        let descriptions: FolderDescriptions =
            serde_yaml::from_str(&content).map_err(|e| format!("解析 YAML 文件失败: {}", e))?;

        Ok(descriptions)
    }

    pub fn get_description(&self, folder_name: &str, folder_type: &str) -> Option<String> {
        match folder_type {
            "Roaming" => self.Roaming.get(folder_name).cloned(),
            "Local" => self.Local.get(folder_name).cloned(),
            "LocalLow" => self.LocalLow.get(folder_name).cloned(),
            _ => None,
        }
    }

    pub fn update_description(
        &mut self,
        folder_name: &str,
        folder_type: &str,
        description: String,
    ) -> Result<(), String> {
        match folder_type {
            "Roaming" => {
                self.Roaming.insert(folder_name.to_string(), description);
            }
            "Local" => {
                self.Local.insert(folder_name.to_string(), description);
            }
            "LocalLow" => {
                self.LocalLow.insert(folder_name.to_string(), description);
            }
            _ => return Err("无效的文件夹类型".to_string()),
        }
        Ok(())
    }

    pub fn save_to_yaml(&self, file_path: &str) -> Result<(), String> {
        let yaml_string =
            serde_yaml::to_string(self).map_err(|e| format!("序列化 YAML 失败: {}", e))?;

        fs::write(file_path, yaml_string).map_err(|e| format!("写入 YAML 文件失败: {}", e))?;

        Ok(())
    }
}

// 新增函数，用于加载文件夹描述
pub fn load_folder_descriptions(
    file_path: &str,
    yaml_error_logged: &mut bool,
) -> Option<FolderDescriptions> {
    match FolderDescriptions::load_from_yaml(file_path) {
        Ok(descriptions) => Some(descriptions),
        Err(e) => {
            if !*yaml_error_logged {
                eprintln!("加载 YAML 文件失败: {}", e);
                crate::logger::log_error(&format!("加载 YAML 文件失败: {}", e));
                *yaml_error_logged = true; // 记录错误，避免重复输出
            }
            None
        }
    }
}

// 创建默认的描述文件
pub fn create_default_descriptions() -> FolderDescriptions {
    FolderDescriptions {
        Roaming: HashMap::new(),
        Local: HashMap::new(),
        LocalLow: HashMap::new(),
    }
}
