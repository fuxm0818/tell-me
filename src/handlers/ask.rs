// ask 命令处理器
// 提问查询：直接检索已有向量库，不重建

use std::path::Path;

use crate::config::ConfigStore;
use crate::embedding::EmbeddingService;
use crate::error::CoiError;
use crate::fqa_store::FQAStore;
use crate::vector_store::VectorStore;

/// 处理 ask 命令
///
/// # 流程
/// 1. 验证问题非空白
/// 2. 加载配置，验证已初始化
/// 3. 检查向量库是否存在
/// 4. 向量化用户问题，检索 Top 5
/// 5. FQA 语义匹配 Top 3
/// 6. 分区展示结果
///
/// # 参数
/// - `question`: 用户提问内容
/// - `data_dir`: coi_data 目录路径
pub fn handle_ask(question: &str, data_dir: &Path) -> Result<(), CoiError> {
    // 1. 验证问题非空白
    let trimmed_question = question.trim();
    if trimmed_question.is_empty() {
        return Err(CoiError::InvalidInput {
            reason: "问题内容不能为空".to_string(),
        });
    }

    // 2. 加载配置，验证已初始化
    let config_path = data_dir.join("config.json");
    let config_store = ConfigStore::new(&config_path);

    if !config_store.exists() {
        return Err(CoiError::NotInitialized);
    }

    let _config = config_store
        .load()
        .map_err(|e| CoiError::Other(e))?
        .ok_or(CoiError::NotInitialized)?;

    // 3. 检查向量库是否存在
    let vector_db_path = data_dir.join("vector_db");
    let vector_store = VectorStore::new(&vector_db_path);

    if vector_store.is_empty() {
        return Err(CoiError::InvalidInput {
            reason: "向量库为空，请先执行 coi init 构建知识库".to_string(),
        });
    }

    // 4. 加载嵌入模型，向量化用户问题
    let model_dir = data_dir
        .parent()
        .unwrap_or(Path::new("."))
        .join("model");

    let embedding_service = EmbeddingService::new(&model_dir)?;
    let query_embedding = embedding_service.encode(trimmed_question)?;

    // 5. 检索向量库 Top 5
    let doc_results = vector_store
        .query(&query_embedding, 5)
        .map_err(|e| CoiError::Other(e))?;

    // 6. FQA 语义匹配 Top 3
    let fqa_path = data_dir.join("fqa.json");
    let fqa_results = if fqa_path.exists() {
        let fqa_store = FQAStore::new(&fqa_path).map_err(|e| CoiError::Other(e))?;
        fqa_store.search(&query_embedding, 3)
    } else {
        Vec::new()
    };

    // 7. 分区展示结果
    let has_doc_results = !doc_results.is_empty();
    let has_fqa_results = !fqa_results.is_empty();

    if !has_doc_results && !has_fqa_results {
        println!("未找到相关答案");
        return Ok(());
    }

    // 显示文档检索结果
    if has_doc_results {
        println!("📄 文档检索结果：");
        for (i, result) in doc_results.iter().enumerate() {
            println!(
                "  [{}] (来源: {}, 相似度: {:.2})",
                i + 1,
                result.source_file,
                result.score
            );
            let display_content = truncate_content(&result.content, 200);
            println!("      {}", display_content);
        }
    }

    // 显示标准答案结果
    if has_fqa_results {
        if has_doc_results {
            println!();
        }
        println!("💡 标准答案：");
        for (i, result) in fqa_results.iter().enumerate() {
            println!("  [{}] 问题: {}", i + 1, result.question);
            println!("      答案: {}", result.answer);
            println!("      (相似度: {:.2})", result.score);
        }
    }

    Ok(())
}

/// 截取文本内容用于显示
fn truncate_content(content: &str, max_chars: usize) -> String {
    let single_line: String = content
        .chars()
        .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
        .collect();

    let chars: Vec<char> = single_line.chars().collect();
    if chars.len() <= max_chars {
        single_line
    } else {
        let truncated: String = chars[..max_chars].iter().collect();
        format!("{}...", truncated)
    }
}
