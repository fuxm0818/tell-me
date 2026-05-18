// init 命令处理器
// 初始化文档知识库：验证路径、扫描文档、解析文本、分块、向量化、存储

use std::collections::HashMap;
use std::path::Path;

use crate::config::{Config, ConfigStore};
use crate::embedding::EmbeddingService;
use crate::error::CoiError;
use crate::parser::DocumentParser;
use crate::scanner::{DocumentScanner, SkipInfo};
use crate::splitter::{ChunkSplitter, TextChunk};
use crate::vector_store::VectorStore;

/// 打印带前缀的信息
fn print_info(message: &str) {
    println!("[COI] {}", message);
}

/// 打印进度条
fn print_progress(current: usize, total: usize, stage: &str) {
    if total == 0 {
        return;
    }
    let percentage = (current as f64 / total as f64) * 100.0;
    println!("[COI] {}: [{}/{}] {:.1}%", stage, current, total, percentage);
}

/// 处理 init 命令
///
/// 完整流程：
/// 1. 验证文档路径是否存在且为目录
/// 2. 转换为绝对路径
/// 3. 保存配置到 config.json
/// 4. 扫描文档文件夹
/// 5. 流式解析文件、分块、向量化、存储
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

    let total_files = scan_result.files.len();
    print_info(&format!(
        "扫描完成: 发现 {} 个文档, 跳过 {} 个隐藏文件, {} 个隐藏目录",
        total_files,
        skipped_files.len(),
        skipped_dirs.len()
    ));

    // 如果没有支持格式的文档，输出提示并列出支持格式
    if total_files == 0 {
        print_info("未找到可处理的文档");
        print_info(&format!("支持的文件格式: {}", scanner.supported_extensions().join(", ")));
        return Ok(());
    }

    // 输出扩展名统计
    print_info("文件类型分布:");
    for (ext, count) in ext_stats {
        println!("  .{}: {} 个", ext, count);
    }

    // 5. 流式处理：解析、分块、向量化、存储
    let parser = DocumentParser::new();
    let splitter = ChunkSplitter::default();

    // 流式处理参数
    const BATCH_SIZE: usize = 32; // 每批处理的文本块数量
    let mut current_file = 0;
    let mut success_count = 0;
    let mut total_chars = 0;
    let mut total_chunks = 0;
    let mut failed_files: Vec<(String, String)> = Vec::new();
    
    // 当前批次的 chunks 和 embeddings
    let mut batch_chunks: Vec<TextChunk> = Vec::new();

    // 初始化向量存储
    let vector_db_path = data_dir.join("vector_db");
    let mut vector_store = VectorStore::new(&vector_db_path);
    
    // 加载嵌入模型
    let model_dir = data_dir
        .parent()
        .unwrap_or(data_dir)
        .join("model");
    print_info("正在加载嵌入模型...");
    let embedding_service = EmbeddingService::new(&model_dir)?;

    print_info(&format!("开始处理 {} 个文件（流式处理模式）...", total_files));

    // 顺序处理每个文件（避免过多 CPU 占用）
    for file_info in &scan_result.files {
        current_file += 1;
        
        let file_name = file_info
            .relative_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // 解析文件
        match parser.parse(&file_info.absolute_path) {
            Ok(parse_result) => {
                // 跳过解析后内容为空的文件
                if parse_result.content.trim().is_empty() {
                    failed_files.push((file_name.clone(), "解析后内容为空".to_string()));
                    println!("  [{}/{}] {} - 跳过（内容为空）", current_file, total_files, file_name);
                    print_progress(current_file, total_files, "处理进度");
                    continue;
                }

                // 文本分块
                let source = file_info.relative_path.to_string_lossy().to_string();
                let chunks = splitter.split(&parse_result.content, &source);
                let chunk_count = chunks.len();
                
                success_count += 1;
                total_chars += parse_result.content.chars().count();
                total_chunks += chunk_count;

                println!("  [{}/{}] {} - {} 个文本块", current_file, total_files, file_name, chunk_count);

                // 将 chunks 添加到批次
                batch_chunks.extend(chunks);

                // 如果批次达到阈值，进行向量化和存储
                if batch_chunks.len() >= BATCH_SIZE {
                    process_batch(
                        &embedding_service,
                        &mut vector_store,
                        &mut batch_chunks,
                        total_chunks,
                    )?;
                }
            }
            Err(e) => {
                let reason = match &e {
                    CoiError::ParseError { reason, .. } => reason.clone(),
                    _ => e.to_string(),
                };
                failed_files.push((file_name.clone(), reason.clone()));
                println!("  [{}/{}] {} - 失败: {}", current_file, total_files, file_name, reason);
            }
        }

        // 打印进度百分比
        print_progress(current_file, total_files, "处理进度");
    }

    // 处理剩余的文本块
    if !batch_chunks.is_empty() {
        process_batch(
            &embedding_service,
            &mut vector_store,
            &mut batch_chunks,
            total_chunks,
        )?;
    }

    // 完成向量库重建
    vector_store.complete_rebuild().map_err(|e| {
        CoiError::Other(anyhow::anyhow!("向量库重建完成失败: {}", e))
    })?;

    println!();
    print_info(&format!("处理进度: [{}/{}] 100.0%", total_files, total_files));
    print_info(&format!("初始化完成！"));
    print_info(&format!("  文档数量: {} 个", success_count));
    print_info(&format!("  文本块数量: {} 个", total_chunks));
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

/// 处理一批文本块：向量化并存储
fn process_batch(
    embedding_service: &EmbeddingService,
    vector_store: &mut VectorStore,
    batch_chunks: &mut Vec<TextChunk>,
    total_chunks_processed: usize,
) -> Result<(), CoiError> {
    let batch_size = batch_chunks.len();
    
    // 向量化
    let texts: Vec<&str> = batch_chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = embedding_service.encode_batch(texts)?;

    // 存储到向量库（增量添加）
    vector_store.add_chunks(batch_chunks, &embeddings).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("向量化存储失败: {}", e))
    })?;

    println!("    已处理 {} 个文本块（累计: {}）", batch_size, total_chunks_processed);

    // 清空批次，释放内存
    batch_chunks.clear();

    Ok(())
}
