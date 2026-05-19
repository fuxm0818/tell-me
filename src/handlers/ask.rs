// ask 命令处理器
// 提问查询：直接检索已有向量库，不重建

use std::collections::HashMap;
use std::path::Path;

use crate::config::ConfigStore;
use crate::embedding::EmbeddingService;
use crate::error::TellMeError;
use crate::fqa_store::{FQAStore, FQASearchConfig};
use crate::vector_store::VectorStore;

/// 打印带前缀的信息
fn print_info(message: &str) {
    println!("[TELL-ME] {}", message);
}

/// 处理 ask 命令
///
/// # 流程
/// 1. 验证问题非空白
/// 2. 加载配置，验证已初始化
/// 3. 检查向量库是否存在
/// 4. 向量化用户问题，检索 Top 15
/// 5. FQA 语义匹配（带相似度阈值过滤，默认0.85）
/// 6. 双源合并展示结果（按相似度排序）
///
/// # 参数
/// - `question`: 用户提问内容
/// - `data_dir`: tell_me_data 目录路径
pub fn handle_ask(question: &str, data_dir: &Path) -> Result<(), TellMeError> {
    // 1. 验证问题非空白
    let trimmed_question = question.trim();
    if trimmed_question.is_empty() {
        return Err(TellMeError::InvalidInput {
            reason: "问题内容不能为空".to_string(),
        });
    }

    // 2. 加载配置，验证已初始化
    let config_path = data_dir.join("config.json");
    let config_store = ConfigStore::new(&config_path);

    if !config_store.exists() {
        return Err(TellMeError::NotInitialized);
    }

    let _config = config_store
        .load()
        .map_err(|e| TellMeError::Other(e))?
        .ok_or(TellMeError::NotInitialized)?;

    // 3. 检查向量库是否存在
    let vector_db_path = data_dir.join("vector_db");
    let vector_store = VectorStore::new(&vector_db_path);

    if vector_store.is_empty() {
        return Err(TellMeError::InvalidInput {
            reason: "向量库为空，请先执行 tell-me init 构建知识库".to_string(),
        });
    }

    // 4. 加载嵌入模型，向量化用户问题
    let model_dir = data_dir.join("model");

    print_info(&format!("正在检索: {}", trimmed_question));
    
    let embedding_service = EmbeddingService::new(&model_dir)?;
    let query_embedding = embedding_service.encode(trimmed_question)?;

    // 5. 检索向量库 Top 15（与Python版本保持一致）
    let doc_results = vector_store
        .query(&query_embedding, 15)
        .map_err(|e| TellMeError::Other(e))?;

    // 6. FQA 语义匹配（带相似度阈值过滤，默认0.85）
    let fqa_path = data_dir.join("fqa.json");
    let fqa_results = if fqa_path.exists() {
        let fqa_store = FQAStore::new(&fqa_path).map_err(|e| TellMeError::Other(e))?;
        let config = FQASearchConfig {
            top_k: 3,
            similarity_threshold: 0.85,
            enable_threshold: true,
        };
        fqa_store.search_with_config(&query_embedding, &config)
    } else {
        Vec::new()
    };

    println!();

    let mut has_output = false;

    // 7. FQA 部分输出（参考Python版本格式）
    if !fqa_results.is_empty() {
        has_output = true;
        println!("═══ 标准答案（FQA）═══");
        for result in &fqa_results {
            println!("  相似度: {:.2}", result.score);
            println!("  问题: {}", result.question);
            println!("  答案: {}", result.answer);
        }
        println!();
    }

    // 8. 文档检索部分输出（按文件分组显示，参考Python版本）
    if !doc_results.is_empty() {
        has_output = true;
        println!("═══ 文档检索结果 ═══");
        
        // 按文件分组
        let mut grouped: HashMap<String, Vec<(f32, String)>> = HashMap::new();
        for result in &doc_results {
            grouped
                .entry(result.source_file.clone())
                .or_insert_with(Vec::new)
                .push((result.score, result.content.clone()));
        }

        for (file_path, chunks) in grouped {
            println!("  📄 {}", file_path);
            println!("     ({} 个相关片段)", chunks.len());
            for (score, content) in chunks {
                let preview = truncate_content(&content, 400);
                println!("     • 相似度 {:.2}: {}", score, preview);
            }
            println!();
        }
    }

    // 9. 无结果提示
    if !has_output {
        print_info("未找到相关内容。");
        print_info("提示: 可能需要重新执行 'tell-me init' 更新向量库。");
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
