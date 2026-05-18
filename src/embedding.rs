// 嵌入模型封装模块
// 使用 fastembed 进行文本向量化，加载 BAAI/bge-small-zh-v1.5 模型

use std::path::Path;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::error::CoiError;

/// 嵌入服务：封装 fastembed 模型，提供文本向量化能力
pub struct EmbeddingService {
    model: TextEmbedding,
}

impl EmbeddingService {
    /// 从本地模型目录初始化嵌入模型
    ///
    /// 使用 BAAI/bge-small-zh-v1.5 中文优化模型（384 维）
    /// 首次运行时自动下载模型到指定的 cache_dir 目录
    pub fn new(model_dir: &Path) -> Result<Self, CoiError> {
        let options = InitOptions::new(EmbeddingModel::BGESmallZHV15)
            .with_cache_dir(model_dir.to_path_buf())
            .with_show_download_progress(true);

        let model = TextEmbedding::try_new(options).map_err(|e| CoiError::ModelError {
            reason: e.to_string(),
        })?;

        Ok(Self { model })
    }

    /// 批量将文本转为 384 维向量
    ///
    /// 输入多条文本，返回对应的向量列表
    pub fn encode_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>, CoiError> {
        self.model
            .embed(texts, None)
            .map_err(|e| CoiError::ModelError {
                reason: e.to_string(),
            })
    }

    /// 单条文本向量化
    ///
    /// 将单条文本转为 384 维向量
    pub fn encode(&self, text: &str) -> Result<Vec<f32>, CoiError> {
        let results = self.encode_batch(vec![text])?;
        results.into_iter().next().ok_or_else(|| CoiError::ModelError {
            reason: "模型未返回向量结果".to_string(),
        })
    }
}
