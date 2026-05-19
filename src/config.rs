// 配置管理模块
// 负责 tell_me_data/config.json 的读写

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// 配置数据结构
/// 存储文档文件夹路径和最后初始化时间
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    /// 文档文件夹的绝对路径
    pub doc_folder_path: String,
    /// 最后一次初始化的时间戳
    pub last_init_time: String,
}

/// 配置存储管理器
/// 负责 config.json 文件的读写操作
pub struct ConfigStore {
    /// 配置文件的完整路径（如 tell_me_data/config.json）
    config_path: PathBuf,
}

impl ConfigStore {
    /// 创建 ConfigStore 实例
    ///
    /// # 参数
    /// - `config_path`: 配置文件的路径
    pub fn new(config_path: &Path) -> Self {
        Self {
            config_path: config_path.to_path_buf(),
        }
    }

    /// 保存配置到文件
    /// 如果 tell_me_data 目录不存在，会自动创建
    ///
    /// # 参数
    /// - `config`: 要保存的配置数据
    pub fn save(&self, config: &Config) -> Result<()> {
        // 自动创建父目录（tell_me_data/）
        if let Some(parent) = self.config_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let json = serde_json::to_string_pretty(config)?;
        std::fs::write(&self.config_path, json)?;
        Ok(())
    }

    /// 从文件加载配置
    /// 如果配置文件不存在，返回 Ok(None)
    pub fn load(&self) -> Result<Option<Config>> {
        if !self.config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&self.config_path)?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(Some(config))
    }

    /// 判断配置文件是否存在
    pub fn exists(&self) -> bool {
        self.config_path.exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 测试配置保存和加载的往返一致性
    #[test]
    fn test_save_and_load_roundtrip() {
        let temp_dir = std::env::temp_dir().join("tell_me_test_config_roundtrip");
        let config_path = temp_dir.join("config.json");

        // 清理可能存在的旧测试数据
        let _ = fs::remove_dir_all(&temp_dir);

        let store = ConfigStore::new(&config_path);
        let config = Config {
            doc_folder_path: "/home/user/documents".to_string(),
            last_init_time: "2024-01-01T12:00:00".to_string(),
        };

        // 保存配置
        store.save(&config).unwrap();

        // 加载配置并验证
        let loaded = store.load().unwrap().unwrap();
        assert_eq!(loaded.doc_folder_path, config.doc_folder_path);
        assert_eq!(loaded.last_init_time, config.last_init_time);

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    /// 测试加载不存在的配置文件返回 None
    #[test]
    fn test_load_nonexistent_returns_none() {
        let config_path = std::env::temp_dir().join("tell_me_test_nonexistent/config.json");

        // 确保文件不存在
        let _ = fs::remove_file(&config_path);

        let store = ConfigStore::new(&config_path);
        let result = store.load().unwrap();
        assert!(result.is_none());
    }

    /// 测试 exists 方法
    #[test]
    fn test_exists() {
        let temp_dir = std::env::temp_dir().join("tell_me_test_config_exists");
        let config_path = temp_dir.join("config.json");

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);

        let store = ConfigStore::new(&config_path);

        // 文件不存在时
        assert!(!store.exists());

        // 保存后文件应存在
        let config = Config {
            doc_folder_path: "/tmp/docs".to_string(),
            last_init_time: "2024-06-01T10:00:00".to_string(),
        };
        store.save(&config).unwrap();
        assert!(store.exists());

        // 清理
        let _ = fs::remove_dir_all(&temp_dir);
    }

    /// 测试 save 自动创建目录
    #[test]
    fn test_save_creates_parent_directory() {
        let temp_dir = std::env::temp_dir().join("tell_me_test_auto_mkdir/nested/dir");
        let config_path = temp_dir.join("config.json");

        // 确保目录不存在
        let _ = fs::remove_dir_all(std::env::temp_dir().join("tell_me_test_auto_mkdir"));

        let store = ConfigStore::new(&config_path);
        let config = Config {
            doc_folder_path: "/data/docs".to_string(),
            last_init_time: "2024-03-15T08:30:00".to_string(),
        };

        // save 应自动创建目录
        store.save(&config).unwrap();
        assert!(config_path.exists());

        // 清理
        let _ = fs::remove_dir_all(std::env::temp_dir().join("tell_me_test_auto_mkdir"));
    }
}
