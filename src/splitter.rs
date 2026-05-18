// 文本分块器模块
// 实现智能分块策略，根据文档特征动态调整块大小和重叠
// 支持按中文句子/段落边界优先切分

use std::collections::HashMap;
use std::path::Path;

/// 文本块结构体，表示分块后的单个文本片段
#[derive(Debug, Clone)]
pub struct TextChunk {
    /// 文本内容
    pub content: String,
    /// 来源文件路径
    pub source_file: String,
    /// 块序号（从 0 开始）
    pub chunk_index: usize,
    /// token数量
    pub token_count: usize,
}

/// 文档分析结果
#[derive(Debug)]
struct DocumentAnalysis {
    avg_sentence_length: f64,
    paragraph_count: usize,
    complexity_level: ComplexityLevel,
    title_density: f64,
    list_density: f64,
    has_tables: bool,
    content_type: ContentType,
}

/// 复杂度级别
#[derive(Debug, PartialEq)]
enum ComplexityLevel {
    Low,
    Medium,
    High,
}

/// 内容类型
#[derive(Debug, PartialEq)]
enum ContentType {
    Narrative,
    List,
    Table,
    Structured,
}

/// Chunk策略
struct ChunkStrategy {
    chunk_size: usize,
    chunk_overlap: usize,
    min_chunk_size: usize,
    merge_short_chunks: bool,
    boundary_preference: BoundaryPreference,
}

/// 边界偏好
#[derive(Debug, PartialEq)]
enum BoundaryPreference {
    Sentence,
    Paragraph,
}

/// 文本分块器，实现智能分块策略
pub struct ChunkSplitter {
    /// 默认每块的字符数上限
    default_chunk_size: usize,
    /// 默认相邻块之间的重叠字符数
    default_overlap: usize,
    /// 各文件类型的基础chunk大小
    chunk_size_by_type: HashMap<&'static str, usize>,
    /// 各文件类型的基础重叠大小
    overlap_by_type: HashMap<&'static str, usize>,
}

impl ChunkSplitter {
    #[allow(dead_code)]
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self {
            default_chunk_size: chunk_size,
            default_overlap: overlap,
            chunk_size_by_type: Self::build_chunk_size_map(),
            overlap_by_type: Self::build_overlap_map(),
        }
    }

    /// 使用默认参数创建分块器（chunk_size=500, overlap=50）
    pub fn default() -> Self {
        Self {
            default_chunk_size: 500,
            default_overlap: 50,
            chunk_size_by_type: Self::build_chunk_size_map(),
            overlap_by_type: Self::build_overlap_map(),
        }
    }

    /// 构建文件类型到chunk大小的映射
    fn build_chunk_size_map() -> HashMap<&'static str, usize> {
        let mut map = HashMap::new();
        map.insert("txt", 512);
        map.insert("md", 512);
        map.insert("docx", 512);
        map.insert("doc", 512);
        map.insert("xlsx", 256);
        map.insert("xls", 256);
        map.insert("pdf", 384);
        map.insert("csv", 256);
        map.insert("rtf", 512);
        map.insert("pptx", 512);
        map.insert("ppt", 512);
        map.insert("wps", 512);
        map.insert("et", 256);
        map.insert("dps", 512);
        map
    }

    /// 构建文件类型到重叠大小的映射
    fn build_overlap_map() -> HashMap<&'static str, usize> {
        let mut map = HashMap::new();
        map.insert("txt", 64);
        map.insert("md", 64);
        map.insert("docx", 64);
        map.insert("doc", 64);
        map.insert("xlsx", 32);
        map.insert("xls", 32);
        map.insert("pdf", 48);
        map.insert("csv", 32);
        map.insert("rtf", 64);
        map.insert("pptx", 64);
        map.insert("ppt", 64);
        map.insert("wps", 64);
        map.insert("et", 32);
        map.insert("dps", 64);
        map
    }

    /// 根据文件类型获取基础chunk参数
    fn get_chunk_params(&self, file_path: &str) -> (usize, usize) {
        let ext = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("txt")
            .to_lowercase();

        let chunk_size = *self.chunk_size_by_type.get(ext.as_str()).unwrap_or(&self.default_chunk_size);
        let overlap = *self.overlap_by_type.get(ext.as_str()).unwrap_or(&self.default_overlap);

        (chunk_size, overlap)
    }

    /// 分析文档特征
    fn analyze_document(&self, text: &str, _file_path: &str) -> DocumentAnalysis {
        if text.trim().is_empty() {
            return DocumentAnalysis {
                avg_sentence_length: 0.0,
                paragraph_count: 0,
                complexity_level: ComplexityLevel::Low,
                title_density: 0.0,
                list_density: 0.0,
                has_tables: false,
                content_type: ContentType::Narrative,
            };
        }

        let total_chars = text.len();

        let lines: Vec<&str> = text.split('\n').collect();
        let total_lines = lines.iter().filter(|l| !l.trim().is_empty()).count();

        let paragraphs: Vec<&str> = text.split("\n\n").filter(|p| !p.trim().is_empty()).collect();
        let paragraph_count = paragraphs.len();

        let sentences: Vec<&str> = text.split(|c| matches!(c, '。' | '！' | '？' | '；')).filter(|s| !s.trim().is_empty()).collect();
        let avg_sentence_length = if sentences.is_empty() {
            0.0
        } else {
            sentences.iter().map(|s| s.len()).sum::<usize>() as f64 / sentences.len() as f64
        };

        let non_whitespace_chars = text.chars().filter(|c| !c.is_whitespace()).count();
        let density_score = non_whitespace_chars as f64 / total_chars as f64;

        let (title_lines, list_lines, has_tables) = Self::analyze_line_types(&lines);

        let title_density = if total_lines > 0 {
            title_lines as f64 / total_lines as f64
        } else {
            0.0
        };

        let list_density = if total_lines > 0 {
            list_lines as f64 / total_lines as f64
        } else {
            0.0
        };

        let complexity_level = if avg_sentence_length > 100.0 || density_score > 0.9 {
            ComplexityLevel::High
        } else if avg_sentence_length > 50.0 || density_score > 0.7 {
            ComplexityLevel::Medium
        } else {
            ComplexityLevel::Low
        };

        let content_type = if list_density > 0.4 {
            ContentType::List
        } else if has_tables || text.contains('\t') && total_chars > 1000 {
            ContentType::Table
        } else if title_density > 0.2 {
            ContentType::Structured
        } else {
            ContentType::Narrative
        };

        DocumentAnalysis {
            avg_sentence_length,
            paragraph_count,
            complexity_level,
            title_density,
            list_density,
            has_tables,
            content_type,
        }
    }

    /// 分析行类型（标题、列表、表格）
    fn analyze_line_types(lines: &[&str]) -> (usize, usize, bool) {
        let mut title_lines = 0;
        let mut list_lines = 0;
        let mut has_tables = false;

        for line in lines {
            let stripped = line.trim();
            if stripped.is_empty() {
                continue;
            }

            // 检查标题
            let title_prefixes = &["一、", "二、", "三、", "1.", "2.", "3.", "（一）", "（二）"];
            if stripped.starts_with('#') || 
               stripped.starts_with("【") || stripped.starts_with("】") ||
               stripped.starts_with("第") ||
               title_prefixes.iter().any(|p| stripped.starts_with(p)) {
                title_lines += 1;
            }

            // 检查列表
            let list_prefixes = &["* ", "- ", "• ", "· ", "○ ", "● ", "□ ", "■ ", "（1）", "（2）", "（3）"];
            if list_prefixes.iter().any(|p| stripped.starts_with(p)) ||
               stripped.starts_with(|c: char| c.is_ascii_digit()) && stripped.chars().nth(1) == Some('.') {
                list_lines += 1;
            }

            // 检查表格
            if !has_tables && line.contains('\t') || 
               (stripped.matches('|').count() >= 2 && stripped.starts_with('|')) {
                has_tables = true;
            }
        }

        (title_lines, list_lines, has_tables)
    }

    /// 根据文档分析结果制定个性化的chunk策略
    fn determine_chunk_strategy(&self, analysis: &DocumentAnalysis, file_path: &str) -> ChunkStrategy {
        let (base_chunk_size, base_overlap) = self.get_chunk_params(file_path);

        let mut chunk_size = base_chunk_size;
        let mut chunk_overlap = base_overlap;

        match analysis.content_type {
            ContentType::List => {
                chunk_size = (base_chunk_size as f64 * 0.8) as usize;
                chunk_overlap = (base_overlap as f64 * 1.2) as usize;
            }
            ContentType::Table => {
                chunk_size = (base_chunk_size as f64 * 0.5) as usize;
                chunk_overlap = (base_overlap as f64 * 1.5) as usize;
            }
            ContentType::Structured => {
                chunk_size = (base_chunk_size as f64 * 0.9) as usize;
                chunk_overlap = (base_overlap as f64 * 1.1) as usize;
            }
            ContentType::Narrative => {
                match analysis.complexity_level {
                    ComplexityLevel::High => {
                        chunk_size = (base_chunk_size as f64 * 0.75) as usize;
                        chunk_overlap = (base_overlap as f64 * 1.5) as usize;
                    }
                    ComplexityLevel::Low => {
                        chunk_size = (base_chunk_size as f64 * 1.25) as usize;
                        chunk_overlap = (base_overlap as f64 * 0.75) as usize;
                    }
                    ComplexityLevel::Medium => {}
                }
            }
        }

        // 根据句子长度调整
        if analysis.avg_sentence_length > 120.0 {
            chunk_size = std::cmp::min((chunk_size as f64 * 1.3) as usize, 1024);
        } else if analysis.avg_sentence_length < 30.0 {
            chunk_size = (chunk_size as f64 * 0.85) as usize;
        }

        // 标题密集时增加重叠
        if analysis.title_density > 0.15 {
            chunk_overlap = (chunk_overlap as f64 * 1.2) as usize;
        }

        // 列表密集时调整
        if analysis.list_density > 0.2 {
            chunk_size = (chunk_size as f64 * 0.9) as usize;
            chunk_overlap = (chunk_overlap as f64 * 1.1) as usize;
        }

        let min_chunk_size = std::cmp::max(64, (chunk_size as f64 * 0.3) as usize);

        // 确定边界偏好
        let boundary_preference = if analysis.paragraph_count > 15 || 
                                   analysis.has_tables || 
                                   analysis.content_type == ContentType::Table {
            BoundaryPreference::Paragraph
        } else if analysis.avg_sentence_length < 40.0 {
            BoundaryPreference::Sentence
        } else {
            BoundaryPreference::Sentence
        };

        ChunkStrategy {
            chunk_size,
            chunk_overlap,
            min_chunk_size,
            merge_short_chunks: analysis.content_type != ContentType::List,
            boundary_preference,
        }
    }

    /// 将文本切分为多个 TextChunk
    ///
    /// 执行流程：
    /// 1. 分析文档特征（字符数、token数、句子长度、段落数、密度、复杂度等）
    /// 2. 根据分析结果制定个性化 chunk 策略（块大小、重叠、边界偏好）
    /// 3. 执行智能切块（优先按中文句子/段落边界切分）
    ///
    /// # 参数
    /// - `text`: 待分块的文本内容
    /// - `source_file`: 来源文件路径标识
    ///
    /// # 返回
    /// 分块后的 TextChunk 列表
    pub fn split(&self, text: &str, source_file: &str) -> Vec<TextChunk> {
        // 空文本直接返回空列表
        if text.is_empty() {
            return Vec::new();
        }

        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();

        // 分析文档并确定策略
        let analysis = self.analyze_document(text, source_file);
        let strategy = self.determine_chunk_strategy(&analysis, source_file);

        let chunk_size = strategy.chunk_size;
        let chunk_overlap = strategy.chunk_overlap;
        let min_chunk_size = strategy.min_chunk_size;

        // 文本长度不超过 chunk_size，作为单个块返回
        if total_chars <= chunk_size {
            return vec![TextChunk {
                content: text.to_string(),
                source_file: source_file.to_string(),
                chunk_index: 0,
                token_count: (total_chars as f64 / 2.5) as usize,
            }];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < total_chars {
            let mut end = std::cmp::min(start + chunk_size, total_chars);

            // 根据边界偏好调整切分位置
            if end < total_chars {
                end = match strategy.boundary_preference {
                    BoundaryPreference::Paragraph => {
                        self.find_paragraph_boundary(&chars, start, end)
                    }
                    BoundaryPreference::Sentence => {
                        self.find_sentence_boundary(&chars, start, end)
                    }
                };
            }

            // 构建块内容
            let content: String = chars[start..end].iter().collect();
            let trimmed_content = content.trim();

            if !trimmed_content.is_empty() {
                let token_count = (trimmed_content.len() as f64 / 2.5) as usize;
                chunks.push(TextChunk {
                    content: trimmed_content.to_string(),
                    source_file: source_file.to_string(),
                    chunk_index,
                    token_count,
                });
            }

            if end >= total_chars {
                break;
            }

            // 计算下一块的起始位置
            let step = chunk_size.saturating_sub(chunk_overlap);
            let step = if step == 0 { 1 } else { step };
            start += step;
            chunk_index += 1;
        }

        // 合并小块
        if strategy.merge_short_chunks && chunks.len() > 1 {
            chunks = self.merge_short_chunks(chunks, min_chunk_size);
        }

        chunks
    }

    /// 查找段落边界
    fn find_paragraph_boundary(&self, chars: &[char], start: usize, end: usize) -> usize {
        let search_start = std::cmp::max(start, (start + end) / 2);

        for i in (search_start..end).rev() {
            if chars[i] == '\n' {
                if i + 1 < chars.len() && chars[i + 1] == '\n' {
                    return i + 2;
                }
                return i + 1;
            }
        }

        end
    }

    /// 查找句子边界
    fn find_sentence_boundary(&self, chars: &[char], start: usize, end: usize) -> usize {
        let search_start = std::cmp::max(start, end - 50);

        for i in (search_start..end).rev() {
            if matches!(chars[i], '。' | '！' | '？' | '；' | '\n') {
                return i + 1;
            }
        }

        end
    }

    /// 合并小于最小尺寸的相邻块
    fn merge_short_chunks(&self, mut chunks: Vec<TextChunk>, min_chunk_size: usize) -> Vec<TextChunk> {
        if chunks.len() <= 1 {
            return chunks;
        }

        let mut merged = Vec::new();
        let mut current = chunks.remove(0);

        for next_chunk in chunks {
            if current.content.chars().count() < min_chunk_size {
                current.content = format!("{}\n{}", current.content, next_chunk.content);
                current.token_count += next_chunk.token_count;
            } else {
                merged.push(current);
                current = next_chunk;
            }
        }

        merged.push(current);
        merged
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_text() {
        let splitter = ChunkSplitter::new(500, 50);
        let chunks = splitter.split("", "test.txt");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_text_shorter_than_chunk_size() {
        let splitter = ChunkSplitter::new(500, 50);
        let text = "这是一段短文本";
        let chunks = splitter.split(text, "short.txt");

        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
        assert_eq!(chunks[0].source_file, "short.txt");
        assert_eq!(chunks[0].chunk_index, 0);
    }

    #[test]
    fn test_chinese_character_counting() {
        let splitter = ChunkSplitter::new(3, 1);
        let text = "你好世界测试";
        let chunks = splitter.split(text, "chinese.txt");

        assert!(!chunks.is_empty());
        let total_chars: usize = chunks.iter().map(|c| c.content.chars().count()).sum();
        assert_eq!(total_chars, text.chars().count());
    }

    #[test]
    fn test_mixed_chinese_english() {
        let splitter = ChunkSplitter::new(5, 1);
        let text = "Hello你好World";
        let chunks = splitter.split(text, "mixed.txt");

        assert!(chunks.len() >= 1);
        let total_chars: usize = chunks.iter().map(|c| c.content.chars().count()).sum();
        assert_eq!(total_chars, text.chars().count());
    }

    #[test]
    fn test_source_file_preserved() {
        let splitter = ChunkSplitter::new(3, 1);
        let text = "一二三四五六";
        let source = "docs/readme.md";
        let chunks = splitter.split(text, source);

        for chunk in &chunks {
            assert_eq!(chunk.source_file, source);
        }
    }

    #[test]
    fn test_chunk_index_sequential() {
        let splitter = ChunkSplitter::new(3, 1);
        let text = "一二三四五六七八九十";
        let chunks = splitter.split(text, "test.txt");

        for (i, chunk) in chunks.iter().enumerate() {
            assert_eq!(chunk.chunk_index, i);
        }
    }

    #[test]
    fn test_default_parameters() {
        let splitter = ChunkSplitter::default();
        assert_eq!(splitter.default_chunk_size, 500);
        assert_eq!(splitter.default_overlap, 50);
    }

    #[test]
    fn test_no_overlap() {
        let splitter = ChunkSplitter::new(3, 0);
        let text = "一二三四五六七八九";
        let chunks = splitter.split(text, "test.txt");

        assert!(!chunks.is_empty());
        let total_chars: usize = chunks.iter().map(|c| c.content.chars().count()).sum();
        assert_eq!(total_chars, text.chars().count());
    }

    #[test]
    fn test_overlap_larger_than_chunk_size_no_infinite_loop() {
        let splitter = ChunkSplitter::new(3, 5);
        let text = "一二三四五六";
        let chunks = splitter.split(text, "test.txt");

        assert!(!chunks.is_empty());
        let total_chars: usize = chunks.iter().map(|c| c.content.chars().count()).sum();
        assert_eq!(total_chars, text.chars().count());
    }

    #[test]
    fn test_long_chinese_text() {
        let splitter = ChunkSplitter::new(500, 50);
        let text: String = "这是测试文本内容。".chars().cycle().take(1000).collect();
        let chunks = splitter.split(&text, "long_doc.txt");

        assert!(chunks.len() >= 2);
        let total_chars: usize = chunks.iter().map(|c| c.content.chars().count()).sum();
        // 允许少量误差（由于切分边界调整）
        assert!(total_chars >= 950 && total_chars <= 1050);
    }

    #[test]
    fn test_sentence_boundary_preference() {
        let splitter = ChunkSplitter::new(20, 5);
        let text = "这是第一句话。这是第二句话。这是第三句话。这是第四句话。这是第五句话。这是第六句话。";
        let chunks = splitter.split(text, "sentence.txt");

        assert!(!chunks.is_empty());
        let total_chars: usize = chunks.iter().map(|c| c.content.chars().count()).sum();
        assert_eq!(total_chars, text.chars().count());
    }

    #[test]
    fn test_file_type_based_chunk_size() {
        let splitter = ChunkSplitter::default();
        
        // CSV 文件应该使用较小的 chunk 大小
        let csv_text: String = "a,b,c,d,e,".repeat(100);
        let csv_chunks = splitter.split(&csv_text, "data.csv");
        
        // MD 文件应该使用较大的 chunk 大小
        let md_text: String = "这是一段markdown文本。".repeat(100);
        let md_chunks = splitter.split(&md_text, "document.md");
        
        // CSV 的块数应该更多（因为 chunk 更小）
        assert!(csv_chunks.len() > md_chunks.len());
    }

    #[test]
    fn test_merge_short_chunks() {
        let splitter = ChunkSplitter::new(10, 2);
        let text = "短文本。另一段短文本。";
        let chunks = splitter.split(text, "short.txt");

        // 两段短文本应该被合并
        assert!(chunks.len() <= 1 || chunks[0].content.chars().count() >= 6);
    }

    #[test]
    fn test_token_count_calculated() {
        let splitter = ChunkSplitter::new(10, 2);
        let text = "一二三四五六七八九十";
        let chunks = splitter.split(text, "test.txt");

        for chunk in &chunks {
            // token 数大约是字符数的 1/2.5
            let expected_tokens = (chunk.content.chars().count() as f64 / 2.5) as usize;
            assert_eq!(chunk.token_count, expected_tokens);
        }
    }
}