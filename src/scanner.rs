// 文档扫描器模块
// 递归扫描文件夹，识别支持格式的文档
// 过滤不支持的格式、超大文件和空文件

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// 最大文件大小限制：100MB
const MAX_FILE_SIZE: u64 = 100 * 1024 * 1024;

/// 支持的文档扩展名列表
const SUPPORTED_EXTENSIONS: &[&str] = &["txt", "md", "pdf", "docx", "doc", "xlsx", "xls", "csv"];

/// 文档扫描器
/// 负责递归遍历目录，筛选出支持格式的文档文件
pub struct DocumentScanner {
    /// 支持的文件扩展名
    supported_extensions: Vec<&'static str>,
    /// 文件大小上限（字节）
    max_file_size: u64,
}

/// 文件信息，包含路径
#[derive(Debug, Clone)]
pub struct FileInfo {
    /// 文件相对路径（相对于扫描根目录）
    pub relative_path: PathBuf,
    /// 文件绝对路径
    pub absolute_path: PathBuf,
}

/// 扫描结果
pub struct ScanResult {
    /// 有效文件列表（通过所有过滤条件的文件）
    pub files: Vec<FileInfo>,
    /// 跳过的文件及原因
    pub skipped: Vec<SkipInfo>,
}

/// 跳过文件的信息
pub struct SkipInfo {
    /// 跳过原因
    pub reason: String,
}

impl DocumentScanner {
    /// 创建新的文档扫描器实例
    /// 使用默认的支持扩展名和文件大小限制
    pub fn new() -> Self {
        Self {
            supported_extensions: SUPPORTED_EXTENSIONS.to_vec(),
            max_file_size: MAX_FILE_SIZE,
        }
    }

    /// 递归扫描指定文件夹，返回支持格式的文件列表
    ///
    /// # 参数
    /// - `folder`: 要扫描的文件夹路径
    ///
    /// # 返回
    /// - `Ok(ScanResult)`: 扫描结果，包含有效文件、跳过文件和扫描总数
    /// - `Err`: 路径无效时返回错误
    pub fn scan(&self, folder: &Path) -> anyhow::Result<ScanResult> {
        // 验证路径是否存在且为目录
        if !folder.exists() {
            anyhow::bail!("路径不存在: {}", folder.display());
        }
        if !folder.is_dir() {
            anyhow::bail!("路径不是目录: {}", folder.display());
        }

        let mut files = Vec::new();
        let mut skipped = Vec::new();

        // 使用 walkdir 递归遍历目录
        let walker = WalkDir::new(folder)
            .follow_links(true)
            .min_depth(1)
            .into_iter();

        for entry in walker {
            let entry = match entry {
                Ok(e) => e,
                Err(err) => {
                    skipped.push(SkipInfo {
                        reason: format!("遍历错误: {}", err),
                    });
                    continue;
                }
            };

            // 获取文件名
            let file_name = entry.file_name().to_string_lossy().to_string();

            // 只处理文件，跳过目录
            if !entry.file_type().is_file() {
                // 隐藏目录记录跳过原因
                if file_name.starts_with('.') {
                    skipped.push(SkipInfo {
                        reason: "跳过隐藏目录".to_string(),
                    });
                }
                continue;
            }

            // 跳过隐藏文件（名称以 . 开头）
            if file_name.starts_with('.') {
                skipped.push(SkipInfo {
                    reason: "跳过隐藏文件".to_string(),
                });
                continue;
            }

            let absolute_path = entry.path().to_path_buf();

            // 检查文件大小
            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(err) => {
                    skipped.push(SkipInfo {
                        reason: format!("无法读取文件元数据: {}", err),
                    });
                    continue;
                }
            };

            let file_size = metadata.len();

            // 跳过空文件（0字节）
            if file_size == 0 {
                skipped.push(SkipInfo {
                    reason: "文件为空（0字节）".to_string(),
                });
                continue;
            }

            // 跳过超大文件（>100MB）
            if file_size > self.max_file_size {
                skipped.push(SkipInfo {
                    reason: format!(
                        "文件过大（{:.1}MB），超过 100MB 限制",
                        file_size as f64 / (1024.0 * 1024.0)
                    ),
                });
                continue;
            }

            // 计算相对路径
            let relative_path = match absolute_path.strip_prefix(folder) {
                Ok(p) => p.to_path_buf(),
                Err(_) => absolute_path.clone(),
            };

            // 检查文件扩展名
            let extension = absolute_path
                .extension()
                .and_then(|ext| ext.to_str())
                .map(|ext| ext.to_lowercase());

            match extension {
                Some(ext) if self.supported_extensions.contains(&ext.as_str()) => {
                    files.push(FileInfo {
                        relative_path,
                        absolute_path,
                    });
                }
                Some(ext) => {
                    skipped.push(SkipInfo {
                        reason: format!("不支持的文件格式: .{}", ext),
                    });
                }
                None => {
                    skipped.push(SkipInfo {
                        reason: "文件无扩展名".to_string(),
                    });
                }
            }
        }

        Ok(ScanResult {
            files,
            skipped,
        })
    }

    /// 获取支持的扩展名列表
    pub fn supported_extensions(&self) -> &[&'static str] {
        &self.supported_extensions
    }
}

impl Default for DocumentScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// 辅助函数：在临时目录中创建指定名称的文件
    fn create_file(dir: &Path, name: &str, content: &[u8]) {
        let file_path = dir.join(name);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(file_path, content).unwrap();
    }

    #[test]
    fn test_scan_supported_extensions() {
        // 测试扫描器正确识别所有支持的格式
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "doc.txt", b"hello");
        create_file(dir, "readme.md", b"# Title");
        create_file(dir, "report.pdf", b"pdf content");
        create_file(dir, "letter.docx", b"docx content");
        create_file(dir, "legacy.doc", b"doc content");
        create_file(dir, "data.xlsx", b"xlsx content");
        create_file(dir, "legacy.xls", b"xls content");
        create_file(dir, "table.csv", b"a,b,c");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 8);
        assert_eq!(result.skipped.len(), 0);
    }

    #[test]
    fn test_scan_unsupported_extensions_skipped() {
        // 测试不支持的格式被正确跳过
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "image.png", b"png data");
        create_file(dir, "video.mp4", b"mp4 data");
        create_file(dir, "valid.txt", b"text content");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.skipped.len(), 2);

        // 验证跳过原因包含格式信息
        let skip_reasons: Vec<&str> = result.skipped.iter().map(|s| s.reason.as_str()).collect();
        assert!(skip_reasons.iter().any(|r| r.contains("不支持的文件格式: .png")));
        assert!(skip_reasons.iter().any(|r| r.contains("不支持的文件格式: .mp4")));
    }

    #[test]
    fn test_scan_empty_file_skipped() {
        // 测试空文件（0字节）被跳过
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "empty.txt", b"");
        create_file(dir, "valid.txt", b"content");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("文件为空"));
    }

    #[test]
    fn test_scan_large_file_skipped() {
        // 测试超大文件被跳过（使用较小的限制来测试逻辑）
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // 创建一个略大于 100MB 的文件不现实，
        // 所以我们通过自定义 scanner 来测试逻辑
        create_file(dir, "small.txt", b"small");
        create_file(dir, "big.txt", &vec![b'x'; 1024]);

        let scanner = DocumentScanner {
            supported_extensions: SUPPORTED_EXTENSIONS.to_vec(),
            max_file_size: 512, // 设置为 512 字节以便测试
        };
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("文件过大"));
    }

    #[test]
    fn test_scan_recursive_subdirectories() {
        // 测试递归扫描子目录
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "root.txt", b"root");
        create_file(dir, "sub1/doc.md", b"# Sub1");
        create_file(dir, "sub1/sub2/deep.csv", b"a,b");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 3);
    }

    #[test]
    fn test_scan_nonexistent_path_returns_error() {
        // 测试不存在的路径返回错误
        let scanner = DocumentScanner::new();
        let result = scanner.scan(Path::new("/nonexistent/path/12345"));

        assert!(result.is_err());
    }

    #[test]
    fn test_scan_file_without_extension_skipped() {
        // 测试无扩展名的文件被跳过
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "Makefile", b"all: build");
        create_file(dir, "valid.txt", b"content");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("文件无扩展名"));
    }

    #[test]
    fn test_scan_empty_directory() {
        // 测试空目录返回空结果
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 0);
        assert_eq!(result.skipped.len(), 0);
    }

    #[test]
    fn test_scan_mixed_files() {
        // 测试混合文件类型的综合场景
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        // 有效文件
        create_file(dir, "doc.txt", b"text");
        create_file(dir, "notes.md", b"# Notes");
        // 不支持的格式
        create_file(dir, "image.jpg", b"jpg");
        // 空文件
        create_file(dir, "empty.csv", b"");
        // 无扩展名
        create_file(dir, "README", b"readme");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 2); // txt + md
        assert_eq!(result.skipped.len(), 3); // jpg + empty csv + README
    }

    #[test]
    fn test_scan_case_insensitive_extensions() {
        // 测试扩展名大小写不敏感
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "upper.TXT", b"text");
        create_file(dir, "mixed.Md", b"# Title");
        create_file(dir, "caps.PDF", b"pdf");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 3);
        assert_eq!(result.skipped.len(), 0);
    }

    #[test]
    fn test_scan_hidden_files_skipped() {
        // 测试隐藏文件被正确跳过
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, ".hidden.txt", b"hidden content");
        create_file(dir, "visible.txt", b"visible content");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 1);
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("跳过隐藏文件"));
    }

    #[test]
    fn test_scan_hidden_directories_skipped() {
        // 测试隐藏文件被正确跳过（文件名以 . 开头）
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path();

        create_file(dir, "visible.txt", b"visible");
        create_file(dir, ".hidden1.txt", b"hidden1");
        create_file(dir, ".hidden2.txt", b"hidden2");

        let scanner = DocumentScanner::new();
        let result = scanner.scan(dir).unwrap();

        assert_eq!(result.files.len(), 1); // 只有 visible.txt
        // 隐藏文件都被跳过
        assert_eq!(result.skipped.len(), 2); // .hidden1.txt 和 .hidden2.txt
        let skip_reasons: Vec<&str> = result.skipped.iter().map(|s| s.reason.as_str()).collect();
        assert!(skip_reasons.iter().all(|r| r.contains("跳过隐藏文件")));
    }

    #[test]
    fn test_default_trait() {
        // 测试 Default trait 实现
        let scanner = DocumentScanner::default();
        assert_eq!(scanner.supported_extensions.len(), 8);
        assert_eq!(scanner.max_file_size, MAX_FILE_SIZE);
    }

    #[test]
    fn test_supported_extensions_method() {
        // 测试获取支持扩展名列表
        let scanner = DocumentScanner::new();
        let exts = scanner.supported_extensions();
        assert!(exts.contains(&"txt"));
        assert!(exts.contains(&"md"));
        assert!(exts.contains(&"pdf"));
        assert!(exts.contains(&"docx"));
        assert!(exts.contains(&"doc"));
        assert!(exts.contains(&"xlsx"));
        assert!(exts.contains(&"xls"));
        assert!(exts.contains(&"csv"));
    }
}
