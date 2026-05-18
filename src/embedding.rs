// 嵌入模型封装模块
// 使用 fastembed 进行文本向量化，加载 BAAI/bge-small-zh-v1.5 模型
//
// 模型打包策略：
// 1. 将 BAAI/bge-small-zh-v1.5 模型文件放到 model/ 目录
// 2. 首次运行时，程序会自动检查并使用本地模型
// 3. 无需每次下载，实现开箱即用
//
// 模型下载：
// 如需手动下载模型，请访问：https://huggingface.co/BAAI/bge-small-zh-v1.5
// 下载以下文件到 model/ 目录：
//   - model.onnx
//   - tokenizer.json
//   - tokenizer_config.json

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
    ///
    /// 模型查找顺序：
    /// 1. 首先检查 model/ 目录是否有本地模型文件
    /// 2. 如果有，使用本地模型（无需下载）
    /// 3. 如果没有，让 fastembed 自动下载到缓存目录
    pub fn new(model_dir: &Path) -> Result<Self, CoiError> {
        // 检查本地模型目录
        let local_model_path = Path::new("model").join("model.onnx");

        let options = if local_model_path.exists() {
            println!("[COI] 检测到本地模型文件，使用本地模型");
            println!("[COI] 模型大小: {:.2} MB",
                     std::fs::metadata(&local_model_path)
                         .map(|m| m.len() as f64 / 1024.0 / 1024.0)
                         .unwrap_or(0.0));

            // 使用本地模型目录作为缓存
            InitOptions::new(EmbeddingModel::BGESmallZHV15)
                .with_cache_dir(Path::new("model").to_path_buf())
                .with_show_download_progress(false)
        } else {
            println!("[COI] 未找到本地模型，将从网络下载...");
            println!("[COI] 提示：首次下载后，将 model/ 目录下的文件保留，可实现离线运行");
            InitOptions::new(EmbeddingModel::BGESmallZHV15)
                .with_cache_dir(model_dir.to_path_buf())
                .with_show_download_progress(true)
        };

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
