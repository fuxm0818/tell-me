// add-fqa 命令处理器
// 补充标准问答对到 FQA 存储

use std::path::Path;

use crate::embedding::EmbeddingService;
use crate::error::CoiError;
use crate::fqa_store::FQAStore;

/// 处理 add-fqa 命令
///
/// 验证输入 → 向量化问题 → 添加/更新 FQA 存储 → 持久化
///
/// # 参数
/// - `question`: 用户问题
/// - `answer`: 标准答案
/// - `data_dir`: coi_data 目录路径
///
/// # 返回
/// - `Ok(())`: 成功添加或更新
/// - `Err(CoiError)`: 输入无效或其他错误
pub fn handle_add_fqa(question: &str, answer: &str, data_dir: &Path) -> Result<(), CoiError> {
    // 1. 验证问题和答案非空白
    let question_trimmed = question.trim();
    let answer_trimmed = answer.trim();

    if question_trimmed.is_empty() {
        return Err(CoiError::InvalidInput {
            reason: "问题不能为空白".to_string(),
        });
    }

    if answer_trimmed.is_empty() {
        return Err(CoiError::InvalidInput {
            reason: "答案不能为空白".to_string(),
        });
    }

    // 2. 确保 data_dir 存在
    if !data_dir.exists() {
        std::fs::create_dir_all(data_dir).map_err(|e| {
            CoiError::Other(anyhow::anyhow!("创建数据目录失败: {}", e))
        })?;
    }

    // 3. 初始化 EmbeddingService（model_dir 为 data_dir 的父目录下的 model/）
    let model_dir = data_dir
        .parent()
        .unwrap_or(Path::new("."))
        .join("model");

    let embedding_service = EmbeddingService::new(&model_dir)?;

    // 4. 向量化问题
    let embedding = embedding_service.encode(question_trimmed)?;

    // 5. 加载 FQAStore
    let fqa_path = data_dir.join("fqa.json");
    let mut fqa_store = FQAStore::new(&fqa_path).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("加载 FQA 存储失败: {}", e))
    })?;

    // 6. 添加/更新问答对
    let is_update = fqa_store.add(question_trimmed, answer_trimmed, embedding).map_err(|e| {
        CoiError::Other(anyhow::anyhow!("添加问答对失败: {}", e))
    })?;

    // 7. 持久化保存
    fqa_store.save().map_err(|e| {
        CoiError::Other(anyhow::anyhow!("保存 FQA 存储失败: {}", e))
    })?;

    // 8. 输出确认信息
    if is_update {
        println!("[完成] 已更新标准问答对");
        println!("  问题: {}", question_trimmed);
        println!("  答案: {}", answer_trimmed);
    } else {
        println!("[完成] 已新增标准问答对");
        println!("  问题: {}", question_trimmed);
        println!("  答案: {}", answer_trimmed);
    }

    Ok(())
}
