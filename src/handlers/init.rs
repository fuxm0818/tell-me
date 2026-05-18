// init 命令处理器
// 初始化文档知识库：验证路径、扫描文档、解析文本、分块、向量化、存储

use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

use crate::config::{Config, ConfigStore};
use crate::embedding::EmbeddingService;
use crate::error::CoiError;
use crate::parser::DocumentParser;
use crate::scanner::{DocumentScanner, SkipInfo};
use crate::splitter::{ChunkSplitter, TextChunk};
use crate::vector_store::VectorStore;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

/// 打印带前缀的信息
fn print_info(message: &str) {
    println!("[COI] {}", message);
}

/// 处理 init 命令
///
/// 完整流程：
/// 1. 验证文档路径是否存在且为目录
/// 2. 转换为绝对路径
/// 3. 保存配置到 config.json
/// 4. 扫描文档文件夹（记录文件哈希）
/// 5. 逐文件解析提取文本（部分失败时继续）
/// 6. 文本分块（智能策略）
/// 7. 向量化
/// 8. 重建向量库（记录文件状态）
///
/// # 参数
/// - `doc_path`: 用户传入的文档文件夹路径
/// - `data_dir`: coi_data 目录路径
pub fn handle_init(doc_path: &str, data_dir: &Path) -> Result<(), CoiError> {
    // 1. 验证路径是否存在且为目录
    let path = Path::new(doc_path);
    if !path.exists() || !path.is_dir() {
        return Err(CoiError::InvalidPath {
            path: doc_path.to_string(),
        });
    }

    // 2. 转换为绝对路径
    let abs_path = std::fs::canonicalize(path).map_err(|_| CoiError::InvalidPath {
        path: doc_path.to_string(),
    })?;
    let abs_path_str = abs_path.to_string_lossy().to_string();

    // 3. 创建 coi_data 目录（如不存在）并保存配置
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            CoiError::Other(anyhow::anyhow!("创建 coi_data 目录失败: {}", e))
        })?;
    }

    let config_path = data_dir.join("config.json");
    let config_store = ConfigStore::new(&config_path);
    let config = Config {
        doc_folder_path: abs_path_str.clone(),
        last_init_time: chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
    };
    config_store.save(&config).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("保存配置失败: {}", e))
    })?;

    print_info(&format!("文档路径: {}", abs_path_str));

    // 4. 扫描文档
    let scanner = DocumentScanner::new();
    let scan_result = scanner.scan(&abs_path).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("扫描文档失败: {}", e))
    })?;

    // 统计跳过的文件和目录
    let skipped_dirs: Vec<&SkipInfo> = scan_result
        .skipped
        .iter()
        .filter(|s| s.reason == "跳过隐藏目录")
        .collect();
    let skipped_files: Vec<&SkipInfo> = scan_result
        .skipped
        .iter()
        .filter(|s| s.reason == "跳过隐藏文件")
        .collect();

    // 按扩展名统计文件
    let mut ext_stats: HashMap<String, usize> = HashMap::new();
    for file in &scan_result.files {
        if let Some(ext) = file.absolute_path.extension() {
            *ext_stats.entry(ext.to_string_lossy().to_string()).or_insert(0) += 1;
        }
    }

    print_info(&format!(
        "扫描完成: 发现 {} 个文档, 跳过 {} 个隐藏文件, {} 个隐藏目录",
        scan_result.files.len(),
        skipped_files.len(),
        skipped_dirs.len()
    ));

    // 如果没有支持格式的文档，输出提示并列出支持格式
    if scan_result.files.is_empty() {
        print_info("未找到可处理的文档");
        print_info(&format!("支持的文件格式: {}", scanner.supported_extensions().join(", ")));
        return Ok(());
    }

    // 输出扩展名统计
    print_info("文件类型分布:");
    for (ext, count) in ext_stats {
        println!("  .{}: {} 个", ext, count);
    }

    // 5. 并行解析文件，收集失败信息，继续处理
    let parser = DocumentParser::new();
    let splitter = ChunkSplitter::default();
    
    // 用于并行处理的共享状态
    let progress_counter = AtomicUsize::new(0);
    let success_count = AtomicUsize::new(0);
    let total_chars = AtomicUsize::new(0);
    let failed_files = Mutex::new(Vec::new());
    let total_files = scan_result.files.len();

    print_info(&format!("开始解析 {} 个文件...", total_files));

    // 使用 rayon 并行处理文件
    let all_chunks: Vec<TextChunk> = scan_result
        .files
        .par_iter()
        .flat_map(|file_info| {
            let file_name = file_info
                .relative_path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();

            // 计算进度
            let current = progress_counter.fetch_add(1, Ordering::Relaxed) + 1;

            // 解析文件
            match parser.parse(&file_info.absolute_path) {
                Ok(parse_result) => {
                    // 跳过解析后内容为空的文件
                    if parse_result.content.trim().is_empty() {
                        let mut failed = failed_files.lock().unwrap();
                        failed.push((file_name.clone(), "解析后内容为空".to_string()));
                        println!("  [{}/{}] {} - 跳过（内容为空）", current, total_files, file_name);
                        return Vec::new();
                    }

                    // 6. 文本分块
                    let source = file_info.relative_path.to_string_lossy().to_string();
                    let chunks = splitter.split(&parse_result.content, &source);
                    let chunk_count = chunks.len();
                    
                    success_count.fetch_add(1, Ordering::Relaxed);
                    total_chars.fetch_add(parse_result.content.chars().count(), Ordering::Relaxed);

                    println!("  [{}/{}] {} - {} 个文本块", current, total_files, file_name, chunk_count);
                    chunks
                }
                Err(e) => {
                    let reason = match &e {
                        CoiError::ParseError { reason, .. } => reason.clone(),
                        _ => e.to_string(),
                    };
                    let mut failed = failed_files.lock().unwrap();
                    failed.push((file_name.clone(), reason.clone()));
                    println!("  [{}/{}] {} - 失败: {}", current, total_files, file_name, reason);
                    Vec::new()
                }
            }
        })
        .collect();

    let success_count = success_count.load(Ordering::Relaxed);
    let total_chars = total_chars.load(Ordering::Relaxed);
    let failed_files = failed_files.into_inner().unwrap();

    print_info(&format!(
        "解析完成: 成功 {} 个, 失败 {} 个, 共 {} 个文本块, {} 字符",
        success_count,
        failed_files.len(),
        all_chunks.len(),
        total_chars
    ));

    // 如果所有文件都解析失败
    if all_chunks.is_empty() {
        print_info("所有文档解析失败，无法构建向量库");
        if !failed_files.is_empty() {
            print_info("失败列表:");
            for (name, reason) in &failed_files {
                println!("  - {}: {}", name, reason);
            }
        }
        return Ok(());
    }

    // 7. 向量化
    let model_dir = data_dir
        .parent()
        .unwrap_or(data_dir)
        .join("model");

    print_info("正在加载嵌入模型...");
    let embedding_service = EmbeddingService::new(&model_dir)?;

    print_info(&format!("正在向量化 {} 个文本块...", all_chunks.len()));
    let texts: Vec<&str> = all_chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = embedding_service.encode_batch(texts)?;

    // 8. 重建向量库（记录文件状态）
    let vector_db_path = data_dir.join("vector_db");
    let vector_store = VectorStore::new(&vector_db_path);
    
    // 将 FileInfo 转换为 &Path 引用（使用绝对路径）
    let file_paths: Vec<&std::path::Path> = scan_result.files.iter().map(|f| f.absolute_path.as_path()).collect();
    vector_store.rebuild(&all_chunks, &embeddings, &file_paths).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("向量库重建失败: {}", e))
    })?;

    println!();
    print_info(&format!("初始化完成！"));
    print_info(&format!("  文档数量: {} 个", success_count));
    print_info(&format!("  文本块数量: {} 个", all_chunks.len()));
    print_info(&format!("  总字符数: {} 个", total_chars));

    // 输出失败列表（如有）
    if !failed_files.is_empty() {
        print_info(&format!("⚠️  {} 个文档处理失败:", failed_files.len()));
        for (name, reason) in &failed_files {
            println!("  - {}: {}", name, reason);
        }
    }

    Ok(())
}
