use crate::logger;
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, Result as SqliteResult};
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FolderRecord {
    pub id: Option<i64>,
    pub folder_type: String,    // Roaming, Local, LocalLow
    pub folder_name: String,
    pub folder_size: u64,
    pub last_modified: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    /// 创建或打开数据库连接
    pub fn new(db_path: &str) -> SqliteResult<Self> {
        let conn = Connection::open(db_path)?;
        let db = Database { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// 初始化数据库架构
    fn init_schema(&self) -> SqliteResult<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS folder_scans (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                folder_type TEXT NOT NULL,
                folder_name TEXT NOT NULL,
                folder_size INTEGER NOT NULL,
                last_modified TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                UNIQUE(folder_type, folder_name)
            )",
            [],
        )?;

        // 创建索引提高查询性能
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_folder_type_name 
             ON folder_scans(folder_type, folder_name)",
            [],
        )?;

        logger::log_info("数据库架构初始化完成");
        Ok(())
    }

    /// 获取指定文件夹类型的所有记录
    pub fn get_folders_by_type(&self, folder_type: &str) -> SqliteResult<Vec<FolderRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, folder_type, folder_name, folder_size, last_modified, created_at, updated_at 
             FROM folder_scans WHERE folder_type = ?1 ORDER BY folder_name",
        )?;

        let rows = stmt.query_map([folder_type], |row| {
            Ok(FolderRecord {
                id: Some(row.get(0)?),
                folder_type: row.get(1)?,
                folder_name: row.get(2)?,
                folder_size: row.get(3)?,
                last_modified: DateTime::parse_from_rfc3339(&row.get::<_, String>(4)?)
                    .unwrap()
                    .with_timezone(&Utc),
                created_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(5)?)
                    .unwrap()
                    .with_timezone(&Utc),
                updated_at: DateTime::parse_from_rfc3339(&row.get::<_, String>(6)?)
                    .unwrap()
                    .with_timezone(&Utc),
            })
        })?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row?);
        }
        Ok(records)
    }

    /// 插入或更新文件夹记录
    pub fn upsert_folder(&self, record: &FolderRecord) -> SqliteResult<()> {
        let now = Utc::now().to_rfc3339();
        
        self.conn.execute(
            "INSERT OR REPLACE INTO folder_scans 
             (folder_type, folder_name, folder_size, last_modified, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 
                     COALESCE((SELECT created_at FROM folder_scans 
                              WHERE folder_type = ?1 AND folder_name = ?2), ?5), ?6)",
            params![
                record.folder_type,
                record.folder_name,
                record.folder_size as i64,
                record.last_modified.to_rfc3339(),
                now,  // created_at (only used if record doesn't exist)
                now   // updated_at (always updated)
            ],
        )?;
        Ok(())
    }

    /// 批量更新文件夹记录
    pub fn batch_upsert_folders(&self, records: &[FolderRecord]) -> SqliteResult<()> {
        let tx = self.conn.unchecked_transaction()?;
        
        for record in records {
            let now = Utc::now().to_rfc3339();
            tx.execute(
                "INSERT OR REPLACE INTO folder_scans 
                 (folder_type, folder_name, folder_size, last_modified, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, 
                         COALESCE((SELECT created_at FROM folder_scans 
                                  WHERE folder_type = ?1 AND folder_name = ?2), ?5), ?6)",
                params![
                    record.folder_type,
                    record.folder_name,
                    record.folder_size as i64,
                    record.last_modified.to_rfc3339(),
                    now,
                    now
                ],
            )?;
        }
        
        tx.commit()?;
        logger::log_info(&format!("批量更新了 {} 条文件夹记录", records.len()));
        Ok(())
    }

    /// 删除指定文件夹类型中不存在的文件夹记录
    pub fn remove_missing_folders(&self, folder_type: &str, existing_folders: &[String]) -> SqliteResult<()> {
        if existing_folders.is_empty() {
            return Ok(());
        }

        // 构建 NOT IN 子句的占位符
        let placeholders: Vec<&str> = existing_folders.iter().map(|_| "?").collect();
        let query = format!(
            "DELETE FROM folder_scans WHERE folder_type = ? AND folder_name NOT IN ({})",
            placeholders.join(",")
        );

        let mut params = vec![folder_type.to_string()];
        params.extend(existing_folders.iter().cloned());

        let deleted = self.conn.execute(&query, rusqlite::params_from_iter(params))?;
        
        if deleted > 0 {
            logger::log_info(&format!("从数据库中删除了 {} 个不存在的文件夹记录", deleted));
        }
        
        Ok(())
    }

    /// 检查数据库中是否有指定类型的数据
    pub fn has_data_for_type(&self, folder_type: &str) -> SqliteResult<bool> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM folder_scans WHERE folder_type = ?1",
            [folder_type],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// 获取数据库统计信息
    pub fn get_stats(&self) -> SqliteResult<(i64, String)> {
        let total_records: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM folder_scans",
            [],
            |row| row.get(0),
        )?;

        let last_updated: Option<String> = self.conn.query_row(
            "SELECT MAX(updated_at) FROM folder_scans",
            [],
            |row| row.get(0),
        ).ok();

        Ok((total_records, last_updated.unwrap_or_else(|| "无数据".to_string())))
    }
}

/// 获取默认数据库路径
pub fn get_default_db_path() -> String {
    "appdata_cleaner.db".to_string()
}

/// 检查数据库文件是否存在
pub fn database_exists(db_path: &str) -> bool {
    Path::new(db_path).exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    
    #[test]
    fn test_database_creation_and_basic_operations() {
        let test_db_path = "test_db.db";
        
        // 清理之前的测试数据库
        if database_exists(test_db_path) {
            fs::remove_file(test_db_path).unwrap();
        }
        
        // 创建数据库
        let db = Database::new(test_db_path).unwrap();
        
        // 测试初始状态
        assert!(!db.has_data_for_type("Roaming").unwrap());
        
        // 创建测试记录
        let test_record = FolderRecord {
            id: None,
            folder_type: "Roaming".to_string(),
            folder_name: "TestApp".to_string(),
            folder_size: 1024,
            last_modified: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        // 插入记录
        db.batch_upsert_folders(&[test_record]).unwrap();
        
        // 验证数据存在
        assert!(db.has_data_for_type("Roaming").unwrap());
        
        // 获取记录
        let folders = db.get_folders_by_type("Roaming").unwrap();
        assert_eq!(folders.len(), 1);
        assert_eq!(folders[0].folder_name, "TestApp");
        assert_eq!(folders[0].folder_size, 1024);
        
        // 清理测试数据库
        fs::remove_file(test_db_path).unwrap();
    }
    
    #[test]
    fn test_database_update_and_cleanup() {
        let test_db_path = "test_update_db.db";
        
        // 清理之前的测试数据库
        if database_exists(test_db_path) {
            fs::remove_file(test_db_path).unwrap();
        }
        
        let db = Database::new(test_db_path).unwrap();
        
        // 插入初始记录
        let initial_records = vec![
            FolderRecord {
                id: None,
                folder_type: "Roaming".to_string(),
                folder_name: "App1".to_string(),
                folder_size: 1024,
                last_modified: Utc::now(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            FolderRecord {
                id: None,
                folder_type: "Roaming".to_string(),
                folder_name: "App2".to_string(),
                folder_size: 2048,
                last_modified: Utc::now(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        
        db.batch_upsert_folders(&initial_records).unwrap();
        
        // 验证记录存在
        let folders = db.get_folders_by_type("Roaming").unwrap();
        assert_eq!(folders.len(), 2);
        
        // 更新一个记录
        let updated_record = FolderRecord {
            id: None,
            folder_type: "Roaming".to_string(),
            folder_name: "App1".to_string(),
            folder_size: 4096, // 更新大小
            last_modified: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        
        db.batch_upsert_folders(&[updated_record]).unwrap();
        
        // 验证更新
        let updated_folders = db.get_folders_by_type("Roaming").unwrap();
        assert_eq!(updated_folders.len(), 2);
        
        let app1 = updated_folders.iter().find(|f| f.folder_name == "App1").unwrap();
        assert_eq!(app1.folder_size, 4096);
        
        // 测试清理不存在的文件夹
        let existing_folders = vec!["App1".to_string()]; // 只保留 App1
        db.remove_missing_folders("Roaming", &existing_folders).unwrap();
        
        let remaining_folders = db.get_folders_by_type("Roaming").unwrap();
        assert_eq!(remaining_folders.len(), 1);
        assert_eq!(remaining_folders[0].folder_name, "App1");
        
        // 清理测试数据库
        fs::remove_file(test_db_path).unwrap();
    }
}