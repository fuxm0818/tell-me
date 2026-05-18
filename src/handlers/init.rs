// init 命令处理器
// 初始化文档知识库：验证路径、扫描文档、解析文本、分块、向量化、存储

use std::path::Path;

use crate::config::{Config, ConfigStore};
use crate::embedding::EmbeddingService;
use crate::error::CoiError;
use crate::parser::DocumentParser;
use crate::scanner::DocumentScanner;
use crate::splitter::{ChunkSplitter, TextChunk};
use crate::vector_store::VectorStore;

/// 处理 init 命令
///
/// 完整流程：
/// 1. 验证文档路径是否存在且为目录
/// 2. 转换为绝对路径
/// 3. 保存配置到 config.json
/// 4. 扫描文档文件夹
/// 5. 逐文件解析提取文本（部分失败时继续）
/// 6. 文本分块
/// 7. 向量化
/// 8. 重建向量库
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

    println!("📁 文档路径: {}", abs_path_str);

    // 4. 扫描文档
    let scanner = DocumentScanner::new();
    let scan_result = scanner.scan(&abs_path).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("扫描文档失败: {}", e))
    })?;

    println!("🔍 扫描完成，发现 {} 个支持格式的文档", scan_result.files.len());

    // 如果没有支持格式的文档，输出提示并列出支持格式
    if scan_result.files.is_empty() {
        println!("⚠️  未找到可处理的文档。");
        println!("   当前支持的文件格式: {}", scanner.supported_extensions().join(", "));
        return Ok(());
    }

    // 5. 逐文件解析，收集失败信息，继续处理
    let parser = DocumentParser::new();
    let splitter = ChunkSplitter::default();
    let mut all_chunks: Vec<TextChunk> = Vec::new();
    let mut success_count: usize = 0;
    let mut failed_files: Vec<(String, String)> = Vec::new(); // (文件名, 原因)

    for file_path in &scan_result.files {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        // 解析文件
        match parser.parse(file_path) {
            Ok(parse_result) => {
                // 跳过解析后内容为空的文件
                if parse_result.content.trim().is_empty() {
                    failed_files.push((file_name, "解析后内容为空".to_string()));
                    continue;
                }

                // 6. 文本分块
                let source = file_path.to_string_lossy().to_string();
                let chunks = splitter.split(&parse_result.content, &source);
                all_chunks.extend(chunks);
                success_count += 1;
            }
            Err(e) => {
                let reason = match &e {
                    CoiError::ParseError { reason, .. } => reason.clone(),
                    _ => e.to_string(),
                };
                eprintln!("⚠️  解析失败: {} - {}", file_name, reason);
                failed_files.push((file_name, reason));
            }
        }
    }

    println!("📄 解析完成，成功 {} 个文档，共 {} 个文本块", success_count, all_chunks.len());

    // 如果所有文件都解析失败
    if all_chunks.is_empty() {
        println!("⚠️  所有文档解析失败，无法构建向量库。");
        if !failed_files.is_empty() {
            println!("   失败列表:");
            for (name, reason) in &failed_files {
                println!("   - {}: {}", name, reason);
            }
        }
        return Ok(());
    }

    // 7. 向量化
    // model_dir 为 data_dir 的父目录下的 model 目录
    let model_dir = data_dir
        .parent()
        .unwrap_or(data_dir)
        .join("model");

    println!("🧠 正在加载嵌入模型...");
    let embedding_service = EmbeddingService::new(&model_dir)?;

    println!("🔄 正在向量化 {} 个文本块...", all_chunks.len());
    let texts: Vec<&str> = all_chunks.iter().map(|c| c.content.as_str()).collect();
    let embeddings = embedding_service.encode_batch(texts)?;

    // 8. 重建向量库
    let vector_db_path = data_dir.join("vector_db");
    let vector_store = VectorStore::new(&vector_db_path);
    vector_store.rebuild(&all_chunks, &embeddings).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("向量库重建失败: {}", e))
    })?;

    println!("✅ 初始化完成！成功处理 {} 个文档", success_count);

    // 输出失败列表（如有）
    if !failed_files.is_empty() {
        println!("⚠️  以下文档处理失败:");
        for (name, reason) in &failed_files {
            println!("   - {}: {}", name, reason);
        }
    }

    Ok(())
}
