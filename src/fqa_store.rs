// 标准问答存储模块
// 管理用户补充的问题-答案对，支持余弦相似度语义检索

use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// FQA 条目：存储单个问答对及其向量表示
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FQAEntry {
    /// 用户问题
    pub question: String,
    /// 标准答案
    pub answer: String,
    /// 问题的向量表示（用于语义检索）
    pub embedding: Vec<f32>,
    /// 创建时间（ISO 8601 格式）
    pub created_at: String,
    /// 更新时间（ISO 8601 格式）
    pub updated_at: String,
}

/// FQA 文件结构：用于 JSON 序列化/反序列化
#[derive(Serialize, Deserialize, Debug)]
pub struct FQAFile {
    /// 文件格式版本号
    pub version: u32,
    /// 所有问答条目
    pub entries: Vec<FQAEntry>,
}

/// FQA 搜索结果
#[derive(Debug, Clone)]
pub struct FQASearchResult {
    /// 匹配的问题
    pub question: String,
    /// 对应的标准答案
    pub answer: String,
    /// 余弦相似度分数
    pub score: f32,
}

/// FQA 搜索配置
#[derive(Debug, Clone)]
pub struct FQASearchConfig {
    /// 返回结果数量上限
    pub top_k: usize,
    /// 相似度阈值，低于此值的结果将被过滤（范围：0.0-1.0）
    pub similarity_threshold: f32,
    /// 是否启用阈值过滤
    pub enable_threshold: bool,
}

impl Default for FQASearchConfig {
    fn default() -> Self {
        Self {
            top_k: 3,
            similarity_threshold: 0.85,
            enable_threshold: true,
        }
    }
}

/// 标准问答存储
/// 管理 fqa.json 文件的读写和语义检索
pub struct FQAStore {
    /// fqa.json 文件路径
    fqa_path: PathBuf,
    /// 内存中的问答条目列表
    entries: Vec<FQAEntry>,
}

impl FQAStore {
    /// 创建 FQAStore 实例
    /// 如果 fqa.json 文件已存在则加载，否则初始化空列表
    pub fn new(fqa_path: &Path) -> Result<Self> {
        let entries = if fqa_path.exists() {
            let content = std::fs::read_to_string(fqa_path)?;
            let fqa_file: FQAFile = serde_json::from_str(&content)?;
            fqa_file.entries
        } else {
            Vec::new()
        };

        Ok(Self {
            fqa_path: fqa_path.to_path_buf(),
            entries,
        })
    }

    /// 添加或更新问答对
    /// 如果问题完全一致（精确字符串匹配），则更新答案并返回 true（表示更新）
    /// 否则新增条目并返回 false（表示新增）
    pub fn add(&mut self, question: &str, answer: &str, embedding: Vec<f32>) -> Result<bool> {
        let now = chrono::Local::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        // 查找是否存在完全一致的问题
        if let Some(entry) = self.entries.iter_mut().find(|e| e.question == question) {
            // 精确匹配：更新答案、向量和更新时间
            entry.answer = answer.to_string();
            entry.embedding = embedding;
            entry.updated_at = now;
            Ok(true)
        } else {
            // 新增条目
            let entry = FQAEntry {
                question: question.to_string(),
                answer: answer.to_string(),
                embedding,
                created_at: now.clone(),
                updated_at: now,
            };
            self.entries.push(entry);
            Ok(false)
        }
    }

    #[allow(dead_code)]
    pub fn search(&self, query_embedding: &[f32], top_k: usize) -> Vec<FQASearchResult> {
        self.search_with_config(query_embedding, &FQASearchConfig {
            top_k,
            ..FQASearchConfig::default()
        })
    }

    /// 基于余弦相似度匹配标准答案（带配置版本）
    /// 支持相似度阈值过滤（参考 Python 版本的 0.85 阈值）
    pub fn search_with_config(&self, query_embedding: &[f32], config: &FQASearchConfig) -> Vec<FQASearchResult> {
        if self.entries.is_empty() {
            return Vec::new();
        }

        let mut results: Vec<FQASearchResult> = self
            .entries
            .iter()
            .map(|entry| {
                let score = cosine_similarity(query_embedding, &entry.embedding);
                FQASearchResult {
                    question: entry.question.clone(),
                    answer: entry.answer.clone(),
                    score,
                }
            })
            .filter(|r| {
                if config.enable_threshold {
                    r.score >= config.similarity_threshold
                } else {
                    true
                }
            })
            .collect();

        // 按分数降序排列
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));

        // 取 Top-K
        results.truncate(config.top_k);
        results
    }

    /// 持久化到 fqa.json 文件
    /// 如果父目录不存在则自动创建
    pub fn save(&self) -> Result<()> {
        // 确保父目录存在
        if let Some(parent) = self.fqa_path.parent() {
            if !parent.exists() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let fqa_file = FQAFile {
            version: 1,
            entries: self.entries.clone(),
        };

        let content = serde_json::to_string_pretty(&fqa_file)?;
        std::fs::write(&self.fqa_path, content)?;
        Ok(())
    }
}

/// 计算两个向量的余弦相似度
/// 对零范数向量返回 0.0（避免除零错误）
fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }

    let dot_product: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    // 零范数向量返回 0.0
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_new_with_nonexistent_file() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("fqa.json");

        let store = FQAStore::new(&fqa_path).unwrap();
        assert!(store.entries.is_empty());
    }

    #[test]
    fn test_new_with_existing_file() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("fqa.json");

        // 写入一个已有的 fqa.json
        let fqa_file = FQAFile {
            version: 1,
            entries: vec![FQAEntry {
                question: "什么是COI？".to_string(),
                answer: "本地离线文档问答工具".to_string(),
                embedding: vec![0.1, 0.2, 0.3],
                created_at: "2024-01-01T12:00:00".to_string(),
                updated_at: "2024-01-01T12:00:00".to_string(),
            }],
        };
        let content = serde_json::to_string_pretty(&fqa_file).unwrap();
        fs::write(&fqa_path, content).unwrap();

        let store = FQAStore::new(&fqa_path).unwrap();
        assert_eq!(store.entries.len(), 1);
        assert_eq!(store.entries[0].question, "什么是COI？");
    }

    #[test]
    fn test_add_new_entry() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("fqa.json");

        let mut store = FQAStore::new(&fqa_path).unwrap();
        let result = store.add("问题1", "答案1", vec![0.1, 0.2]).unwrap();

        // 新增返回 false
        assert!(!result);
        assert_eq!(store.entries.len(), 1);
        assert_eq!(store.entries[0].question, "问题1");
        assert_eq!(store.entries[0].answer, "答案1");
    }

    #[test]
    fn test_add_update_existing_entry() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("fqa.json");

        let mut store = FQAStore::new(&fqa_path).unwrap();
        store.add("问题1", "答案1", vec![0.1, 0.2]).unwrap();
        let result = store.add("问题1", "新答案", vec![0.3, 0.4]).unwrap();

        // 更新返回 true
        assert!(result);
        assert_eq!(store.entries.len(), 1);
        assert_eq!(store.entries[0].answer, "新答案");
        assert_eq!(store.entries[0].embedding, vec![0.3, 0.4]);
    }

    #[test]
    fn test_save_and_reload() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("tell_me_data").join("fqa.json");

        let mut store = FQAStore::new(&fqa_path).unwrap();
        store.add("问题A", "答案A", vec![1.0, 0.0]).unwrap();
        store.save().unwrap();

        // 重新加载验证持久化
        let store2 = FQAStore::new(&fqa_path).unwrap();
        assert_eq!(store2.entries.len(), 1);
        assert_eq!(store2.entries[0].question, "问题A");
        assert_eq!(store2.entries[0].answer, "答案A");
    }

    #[test]
    fn test_search_returns_top_k_sorted() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("fqa.json");

        let mut store = FQAStore::new(&fqa_path).unwrap();
        // 添加三个条目，向量方向不同
        store.add("问题1", "答案1", vec![1.0, 0.0, 0.0]).unwrap();
        store.add("问题2", "答案2", vec![0.0, 1.0, 0.0]).unwrap();
        store.add("问题3", "答案3", vec![0.9, 0.1, 0.0]).unwrap();

        // 查询向量与问题1最相似
        let results = store.search(&[1.0, 0.0, 0.0], 2);

        assert_eq!(results.len(), 2);
        // 第一个结果应该是问题1（完全匹配）
        assert_eq!(results[0].question, "问题1");
        assert!((results[0].score - 1.0).abs() < 1e-6);
        // 结果按分数降序排列
        assert!(results[0].score >= results[1].score);
    }

    #[test]
    fn test_search_empty_store() {
        let dir = tempdir().unwrap();
        let fqa_path = dir.path().join("fqa.json");

        let store = FQAStore::new(&fqa_path).unwrap();
        let results = store.search(&[1.0, 0.0], 3);
        assert!(results.is_empty());
    }

    #[test]
    fn test_cosine_similarity_identical_vectors() {
        let score = cosine_similarity(&[1.0, 0.0, 0.0], &[1.0, 0.0, 0.0]);
        assert!((score - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_orthogonal_vectors() {
        let score = cosine_similarity(&[1.0, 0.0], &[0.0, 1.0]);
        assert!(score.abs() < 1e-6);
    }

    #[test]
    fn test_cosine_similarity_zero_vector() {
        let score = cosine_similarity(&[0.0, 0.0], &[1.0, 0.0]);
        assert_eq!(score, 0.0);
    }

    #[test]
    fn test_cosine_similarity_different_lengths() {
        let score = cosine_similarity(&[1.0, 0.0], &[1.0, 0.0, 0.0]);
        assert_eq!(score, 0.0);
    }
}
