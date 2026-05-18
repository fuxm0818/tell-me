// clear 命令处理器
// 一键清空 coi_data 目录及其所有内容

use std::path::Path;

use crate::error::CoiError;

/// 处理 clear 命令
///
/// 检查 coi_data 目录是否存在：
/// - 不存在时输出"当前无数据需要清除"提示
/// - 存在时删除整个 coi_data 目录及其内容
/// - 删除失败时返回 ClearError（权限不足/文件占用）
/// - 仅执行删除，不执行扫描或构建操作
pub fn handle_clear(data_dir: &Path) -> Result<(), CoiError> {
    // 检查 coi_data 目录是否存在
    if !data_dir.exists() {
        println!("当前无数据需要清除");
        return Ok(());
    }

    // 删除整个 coi_data 目录及其内容
    std::fs::remove_dir_all(data_dir).map_err(|e| CoiError::ClearError {
        reason: e.to_string(),
    })?;

    println!("所有数据已成功清除");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_clear_nonexistent_dir() {
        // 不存在的目录应返回 Ok 并提示无数据
        let tmp = TempDir::new().unwrap();
        let non_existent = tmp.path().join("coi_data");
        let result = handle_clear(&non_existent);
        assert!(result.is_ok());
        assert!(!non_existent.exists());
    }

    #[test]
    fn test_clear_existing_dir() {
        // 存在的目录应被成功删除
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("coi_data");
        fs::create_dir_all(data_dir.join("vector_db")).unwrap();
        fs::write(data_dir.join("config.json"), "{}").unwrap();
        fs::write(data_dir.join("fqa.json"), "[]").unwrap();

        let result = handle_clear(&data_dir);
        assert!(result.is_ok());
        assert!(!data_dir.exists());
    }

    #[test]
    fn test_clear_dir_with_nested_content() {
        // 包含嵌套内容的目录应被完整删除
        let tmp = TempDir::new().unwrap();
        let data_dir = tmp.path().join("coi_data");
        let nested = data_dir.join("vector_db").join("sub");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("embeddings.bin"), vec![0u8; 100]).unwrap();
        fs::write(data_dir.join("config.json"), "{}").unwrap();

        let result = handle_clear(&data_dir);
        assert!(result.is_ok());
        assert!(!data_dir.exists());
    }
}
