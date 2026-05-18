// 向量存储模块
// 使用 ndarray 实现余弦相似度检索
// 向量数据以 bincode 序列化持久化，元数据以 JSON 格式存储
// 支持文件哈希记录用于变更检测和增量更新

use ndarray::{Array1, Array2};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::splitter::TextChunk;

/// 检索结果结构体
#[derive(Debug, Clone)]
pub struct SearchResult {
    /// 文本内容
    pub content: String,
    /// 来源文件路径
    pub source_file: String,
    /// 相似度分数
    pub score: f32,
}

/// 文件元数据，记录文件状态用于变更检测
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct FileMetadata {
    /// 文件路径
    pub file_path: String,
    /// 文件哈希值（SHA256）
    pub file_hash: String,
    /// 文件最后修改时间（Unix 时间戳）
    pub modified_at: u64,
    /// 文件大小（字节）
    pub file_size: u64,
}

/// 元数据条目，用于序列化/反序列化
#[derive(Serialize, Deserialize, Debug, Clone)]
struct MetadataEntry {
    /// 文本内容
    content: String,
    /// 来源文件路径
    source_file: String,
    /// 块序号
    chunk_index: usize,
    /// 来源文件哈希
    file_hash: String,
}

/// 向量存储元数据文件结构
#[derive(Serialize, Deserialize, Debug)]
struct StoreMetadata {
    /// 版本号
    version: u32,
    /// 元数据条目列表
    entries: Vec<MetadataEntry>,
    /// 文件状态映射（文件路径 -> 文件元数据）
    file_status: HashMap<String, FileMetadata>,
}

/// 向量存储，管理向量数据的持久化和检索
pub struct VectorStore {
    /// 向量数据库目录路径
    db_path: PathBuf,
    /// 文件状态映射（缓存）
    file_status: HashMap<String, FileMetadata>,
}

impl VectorStore {
    /// 创建新的向量存储实例
    ///
    /// # 参数
    /// - `db_path`: 向量数据库目录路径（如 coi_data/vector_db/）
    pub fn new(db_path: &Path) -> Self {
        let mut store = Self {
            db_path: db_path.to_path_buf(),
            file_status: HashMap::new(),
        };
        // 加载文件状态缓存
        store.load_file_status();
        store
    }

    /// 加载文件状态映射
    fn load_file_status(&mut self) {
        if let Ok(metadata_str) = fs::read_to_string(self.metadata_path()) {
            if let Ok(store_metadata) = serde_json::from_str::<StoreMetadata>(&metadata_str) {
                self.file_status = store_metadata.file_status;
            }
        }
    }

    /// 计算文件的 SHA256 哈希值
    pub fn compute_file_hash(file_path: &Path) -> anyhow::Result<String> {
        let content = fs::read(file_path)?;
        let hash = sha256::digest(&content);
        Ok(hash)
    }

    /// 获取文件的最后修改时间（Unix 时间戳）
    fn get_file_modified_time(file_path: &Path) -> anyhow::Result<u64> {
        let metadata = fs::metadata(file_path)?;
        let modified = metadata.modified()?;
        Ok(modified.duration_since(std::time::UNIX_EPOCH)?.as_secs())
    }

    /// 获取文件大小
    fn get_file_size(file_path: &Path) -> anyhow::Result<u64> {
        let metadata = fs::metadata(file_path)?;
        Ok(metadata.len())
    }

    /// 创建文件元数据
    pub fn create_file_metadata(file_path: &Path) -> anyhow::Result<FileMetadata> {
        Ok(FileMetadata {
            file_path: file_path.to_string_lossy().to_string(),
            file_hash: Self::compute_file_hash(file_path)?,
            modified_at: Self::get_file_modified_time(file_path)?,
            file_size: Self::get_file_size(file_path)?,
        })
    }

    /// 获取 embeddings.bin 文件路径
    fn embeddings_path(&self) -> PathBuf {
        self.db_path.join("embeddings.bin")
    }

    /// 获取 metadata.json 文件路径
    fn metadata_path(&self) -> PathBuf {
        self.db_path.join("metadata.json")
    }

    /// 全量重建向量库（带文件元数据版本）
    ///
    /// 将向量矩阵以 bincode 序列化存储到 embeddings.bin，
    /// 元数据存储到 metadata.json（包含文件哈希信息）。
    ///
    /// # 参数
    /// - `chunks`: 文本块列表
    /// - `embeddings`: 对应的向量列表，与 chunks 一一对应
    /// - `source_files`: 源文件路径列表（用于生成文件哈希）
    ///
    /// # 错误
    /// 当目录创建失败或文件写入失败时返回错误
    pub fn rebuild(
        &self,
        chunks: &[TextChunk],
        embeddings: &[Vec<f32>],
        source_files: &[&Path],
    ) -> anyhow::Result<()> {
        // 确保目录存在
        fs::create_dir_all(&self.db_path)?;

        // 序列化向量数据为 bincode 格式
        let encoded = bincode::serialize(embeddings)?;
        fs::write(self.embeddings_path(), encoded)?;

        // 构建文件状态映射
        let mut file_status: HashMap<String, FileMetadata> = HashMap::new();
        for file_path in source_files {
            if let Ok(metadata) = Self::create_file_metadata(file_path) {
                file_status.insert(metadata.file_path.clone(), metadata);
            }
        }

        // 为每个文本块获取文件哈希
        let mut file_hash_cache: HashMap<String, String> = HashMap::new();
        for file_path in source_files {
            if let Ok(hash) = Self::compute_file_hash(file_path) {
                file_hash_cache.insert(file_path.to_string_lossy().to_string(), hash);
            }
        }

        // 构建元数据列表
        let metadata_entries: Vec<MetadataEntry> = chunks
            .iter()
            .map(|chunk| {
                let file_hash = file_hash_cache
                    .get(&chunk.source_file)
                    .cloned()
                    .unwrap_or_default();
                MetadataEntry {
                    content: chunk.content.clone(),
                    source_file: chunk.source_file.clone(),
                    chunk_index: chunk.chunk_index,
                    file_hash,
                }
            })
            .collect();

        // 构建完整的存储元数据
        let store_metadata = StoreMetadata {
            version: 2,
            entries: metadata_entries,
            file_status,
        };

        let metadata_json = serde_json::to_string_pretty(&store_metadata)?;
        fs::write(self.metadata_path(), metadata_json)?;

        Ok(())
    }

    /// 余弦相似度检索 Top-K
    ///
    /// 加载向量数据和元数据，计算查询向量与所有存储向量的余弦相似度，
    /// 返回相似度最高的 top_k 个结果（降序排列）。
    ///
    /// # 参数
    /// - `query_embedding`: 查询向量
    /// - `top_k`: 返回结果数量上限
    ///
    /// # 返回
    /// 按相似度降序排列的检索结果列表
    ///
    /// # 错误
    /// 当文件读取失败或数据格式错误时返回错误
    pub fn query(&self, query_embedding: &[f32], top_k: usize) -> anyhow::Result<Vec<SearchResult>> {
        // 检查文件是否存在
        if !self.embeddings_path().exists() || !self.metadata_path().exists() {
            return Ok(Vec::new());
        }

        // 加载向量数据
        let embeddings_data = fs::read(self.embeddings_path())?;
        let embeddings: Vec<Vec<f32>> = bincode::deserialize(&embeddings_data)?;

        // 加载元数据（支持新格式和旧格式）
        let metadata_data = fs::read_to_string(self.metadata_path())?;
        let metadata: Vec<MetadataEntry> = match serde_json::from_str::<StoreMetadata>(&metadata_data) {
            Ok(store_metadata) => {
                // 新格式：包含 file_status
                store_metadata.entries
            }
            Err(_) => {
                // 旧格式：直接是 MetadataEntry 数组
                serde_json::from_str(&metadata_data)?
            }
        };

        // 空数据直接返回
        if embeddings.is_empty() || metadata.is_empty() {
            return Ok(Vec::new());
        }

        // 获取向量维度
        let dim = embeddings[0].len();
        let num_vectors = embeddings.len();

        // 将 Vec<Vec<f32>> 转换为 ndarray Array2
        let mut matrix = Array2::<f32>::zeros((num_vectors, dim));
        for (i, emb) in embeddings.iter().enumerate() {
            for (j, &val) in emb.iter().enumerate() {
                matrix[[i, j]] = val;
            }
        }

        // 将查询向量转换为 ndarray Array1
        let query_vec = Array1::from_vec(query_embedding.to_vec());

        // 计算查询向量的 L2 范数
        let query_norm = query_vec.dot(&query_vec).sqrt();
        if query_norm == 0.0 {
            return Ok(Vec::new());
        }

        // 计算每个存储向量与查询向量的余弦相似度
        let mut scores: Vec<(usize, f32)> = Vec::with_capacity(num_vectors);
        for i in 0..num_vectors {
            let row = matrix.row(i);
            let row_norm = row.dot(&row).sqrt();
            if row_norm == 0.0 {
                scores.push((i, 0.0));
                continue;
            }
            let dot_product = row.dot(&query_vec);
            let cosine_sim = dot_product / (row_norm * query_norm);
            scores.push((i, cosine_sim));
        }

        // 按相似度降序排列
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // 取 Top-K 结果
        let results: Vec<SearchResult> = scores
            .iter()
            .take(top_k)
            .filter_map(|(idx, score)| {
                metadata.get(*idx).map(|meta| SearchResult {
                    content: meta.content.clone(),
                    source_file: meta.source_file.clone(),
                    score: *score,
                })
            })
            .collect();

        Ok(results)
    }

    /// 判断向量库是否为空
    ///
    /// 检查 embeddings.bin 文件是否存在且非空
    pub fn is_empty(&self) -> bool {
        let path = self.embeddings_path();
        if !path.exists() {
            return true;
        }
        // 检查文件大小是否为 0
        match fs::metadata(&path) {
            Ok(meta) => meta.len() == 0,
            Err(_) => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// 创建测试用的 TextChunk 列表
    fn create_test_chunks() -> Vec<TextChunk> {
        vec![
            TextChunk {
                content: "Rust 是一门系统编程语言".to_string(),
                source_file: "docs/rust.md".to_string(),
                chunk_index: 0,
                token_count: 6,
            },
            TextChunk {
                content: "Python 是一门脚本语言".to_string(),
                source_file: "docs/python.md".to_string(),
                chunk_index: 0,
                token_count: 6,
            },
            TextChunk {
                content: "向量检索用于语义搜索".to_string(),
                source_file: "docs/search.md".to_string(),
                chunk_index: 1,
                token_count: 6,
            },
        ]
    }

    /// 创建测试用的向量数据（3 个 4 维向量）
    fn create_test_embeddings() -> Vec<Vec<f32>> {
        vec![
            vec![1.0, 0.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0, 0.0],
            vec![0.7, 0.7, 0.0, 0.0],
        ]
    }

    #[test]
    fn test_new_vector_store() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());
        assert_eq!(store.db_path, tmp_dir.path());
    }

    #[test]
    fn test_is_empty_when_no_files() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());
        assert!(store.is_empty());
    }

    #[test]
    fn test_is_empty_after_rebuild() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();

        store.rebuild(&chunks, &embeddings, &[]).unwrap();
        assert!(!store.is_empty());
    }

    #[test]
    fn test_is_empty_with_empty_data() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks: Vec<TextChunk> = vec![];
        let embeddings: Vec<Vec<f32>> = vec![];

        store.rebuild(&chunks, &embeddings, &[]).unwrap();
        // bincode 序列化空 Vec 仍有少量字节（长度前缀），所以文件非空
        // 但 query 会返回空结果
        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_rebuild_creates_files() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();

        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        assert!(tmp_dir.path().join("embeddings.bin").exists());
        assert!(tmp_dir.path().join("metadata.json").exists());
    }

    #[test]
    fn test_rebuild_metadata_content() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();

        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        // 验证元数据文件内容（新格式）
        let metadata_str = fs::read_to_string(tmp_dir.path().join("metadata.json")).unwrap();
        let store_metadata: StoreMetadata = serde_json::from_str(&metadata_str).unwrap();

        assert_eq!(store_metadata.version, 2);
        assert_eq!(store_metadata.entries.len(), 3);
        assert_eq!(store_metadata.entries[0].content, "Rust 是一门系统编程语言");
        assert_eq!(store_metadata.entries[0].source_file, "docs/rust.md");
        assert_eq!(store_metadata.entries[0].chunk_index, 0);
        assert_eq!(store_metadata.entries[1].content, "Python 是一门脚本语言");
        assert_eq!(store_metadata.entries[2].source_file, "docs/search.md");
        assert_eq!(store_metadata.entries[2].chunk_index, 1);
    }

    #[test]
    fn test_query_returns_correct_top_k() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();
        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        // 查询向量与第一个向量完全一致
        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 2).unwrap();

        // 应返回不超过 top_k 个结果
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_query_results_sorted_descending() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();
        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 3).unwrap();

        // 验证结果按分数降序排列
        for i in 0..results.len() - 1 {
            assert!(
                results[i].score >= results[i + 1].score,
                "结果未按降序排列: {} < {}",
                results[i].score,
                results[i + 1].score
            );
        }
    }

    #[test]
    fn test_query_cosine_similarity_correctness() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();
        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        // 查询向量 [1, 0, 0, 0] 与第一个向量 [1, 0, 0, 0] 余弦相似度为 1.0
        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 3).unwrap();

        // 第一个结果应该是与查询向量完全一致的向量
        assert_eq!(results[0].content, "Rust 是一门系统编程语言");
        assert!((results[0].score - 1.0).abs() < 1e-6);

        // 第二个结果应该是 [0.7, 0.7, 0, 0]，余弦相似度约为 0.707
        assert_eq!(results[1].content, "向量检索用于语义搜索");
        assert!((results[1].score - 0.7071068).abs() < 1e-4);

        // 第三个结果应该是 [0, 1, 0, 0]，余弦相似度为 0.0
        assert_eq!(results[2].content, "Python 是一门脚本语言");
        assert!((results[2].score - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_query_source_is_document() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();
        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        let _results = store.query(&[1.0, 0.0, 0.0, 0.0], 3).unwrap();

    }

    #[test]
    fn test_query_when_no_data() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        // 未 rebuild 时查询应返回空结果
        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_query_top_k_larger_than_data() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();
        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        // top_k 大于数据量时，返回所有数据
        let results = store.query(&[1.0, 0.0, 0.0, 0.0], 100).unwrap();
        assert_eq!(results.len(), 3);
    }

    #[test]
    fn test_query_with_zero_vector() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        let chunks = create_test_chunks();
        let embeddings = create_test_embeddings();
        store.rebuild(&chunks, &embeddings, &[]).unwrap();

        // 零向量查询应返回空结果（范数为 0 无法计算余弦相似度）
        let results = store.query(&[0.0, 0.0, 0.0, 0.0], 5).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_rebuild_overwrites_existing_data() {
        let tmp_dir = TempDir::new().unwrap();
        let store = VectorStore::new(tmp_dir.path());

        // 第一次写入
        let chunks1 = vec![TextChunk {
            content: "旧数据".to_string(),
            source_file: "old.txt".to_string(),
            chunk_index: 0,
            token_count: 2,
        }];
        let embeddings1 = vec![vec![1.0, 0.0]];
        store.rebuild(&chunks1, &embeddings1, &[]).unwrap();

        // 第二次写入（覆盖）
        let chunks2 = vec![TextChunk {
            content: "新数据".to_string(),
            source_file: "new.txt".to_string(),
            chunk_index: 0,
            token_count: 2,
        }];
        let embeddings2 = vec![vec![0.0, 1.0]];
        store.rebuild(&chunks2, &embeddings2, &[]).unwrap();

        // 查询应只返回新数据
        let results = store.query(&[0.0, 1.0], 5).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].content, "新数据");
        assert_eq!(results[0].source_file, "new.txt");
    }
}
