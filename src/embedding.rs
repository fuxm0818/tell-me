// 嵌入模型封装模块
// 使用 fastembed 进行文本向量化，加载 BAAI/bge-small-zh-v1.5 模型
//
// 单文件分发策略：
// 1. 模型文件使用 include_bytes! 宏嵌入到可执行文件中
// 2. 首次运行时自动提取到 coi_data/model/ 目录
// 3. 后续运行直接使用已提取的模型
// 4. 分发时只需要一个可执行文件，无需额外拷贝 model/ 目录

use std::fs::{self, File};
use std::io::Write;
use std::path::Path;

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};

use crate::error::CoiError;

/// 嵌入服务：封装 fastembed 模型，提供文本向量化能力
pub struct EmbeddingService {
    model: TextEmbedding,
}

// 嵌入模型文件（编译时打包进可执行文件）
const MODEL_ONNX: &[u8] = include_bytes!("../model/model.onnx");
const TOKENIZER_JSON: &[u8] = include_bytes!("../model/tokenizer.json");
const TOKENIZER_CONFIG_JSON: &[u8] = include_bytes!("../model/tokenizer_config.json");
const SPECIAL_TOKENS_MAP_JSON: &[u8] = include_bytes!("../model/special_tokens_map.json");
const CONFIG_JSON: &[u8] = include_bytes!("../model/config.json");
const MODULES_JSON: &[u8] = include_bytes!("../model/modules.json");

impl EmbeddingService {
    /// 从嵌入的模型数据初始化嵌入模型
    ///
    /// 使用 BAAI/bge-small-zh-v1.5 中文优化模型（384 维）
    ///
    /// 模型查找顺序：
    /// 1. 检查 coi_data/model/ 目录是否有已提取的模型
    /// 2. 如果有，直接使用
    /// 3. 如果没有，从嵌入的二进制数据提取到 coi_data/model/ 目录
    pub fn new(model_dir: &Path) -> Result<Self, CoiError> {
        // 模型提取目标目录（直接使用传入的 model_dir）
        let model_path = model_dir.join("model.onnx");

        // 如果模型目录不存在，从嵌入数据提取
        if !model_path.exists() {
            println!("[COI] 首次运行，正在提取嵌入的模型文件...");
            Self::extract_embedded_model(&model_dir)?;
            println!("[COI] 模型提取完成");
        }

        println!("[COI] 使用本地模型");

        let options = InitOptions::new(EmbeddingModel::BGESmallZHV15)
            .with_cache_dir(model_dir.to_path_buf())
            .with_show_download_progress(false);

        let model = TextEmbedding::try_new(options).map_err(|e| CoiError::ModelError {
            reason: e.to_string(),
        })?;

        Ok(Self { model })
    }

    /// 从嵌入的二进制数据提取模型文件到目标目录
    fn extract_embedded_model(model_dir: &Path) -> Result<(), CoiError> {
        // 创建模型目录
        fs::create_dir_all(model_dir).map_err(|e| CoiError::ModelError {
            reason: format!("创建模型目录失败: {}", e),
        })?;

        // 定义要提取的文件
        let files = vec![
            ("model.onnx", MODEL_ONNX),
            ("tokenizer.json", TOKENIZER_JSON),
            ("tokenizer_config.json", TOKENIZER_CONFIG_JSON),
            ("special_tokens_map.json", SPECIAL_TOKENS_MAP_JSON),
            ("config.json", CONFIG_JSON),
            ("modules.json", MODULES_JSON),
        ];

        // 提取每个文件
        for (filename, data) in files {
            let file_path = model_dir.join(filename);
            let mut file = File::create(&file_path).map_err(|e| CoiError::ModelError {
                reason: format!("创建文件 {} 失败: {}", filename, e),
            })?;
            file.write_all(data).map_err(|e| CoiError::ModelError {
                reason: format!("写入文件 {} 失败: {}", filename, e),
            })?;
        }

        Ok(())
    }

    /// 批量将文本转为 384 维向量
    pub fn encode_batch(&self, texts: Vec<&str>) -> Result<Vec<Vec<f32>>, CoiError> {
        self.model
            .embed(texts, None)
            .map_err(|e| CoiError::ModelError {
                reason: e.to_string(),
            })
    }

    /// 单条文本向量化
    pub fn encode(&self, text: &str) -> Result<Vec<f32>, CoiError> {
        let results = self.encode_batch(vec![text])?;
        results.into_iter().next().ok_or_else(|| CoiError::ModelError {
            reason: "模型未返回向量结果".to_string(),
        })
    }
}