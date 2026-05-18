// 文本分块器模块
// 将文本按字符数分块，支持重叠区域
// 使用 .chars().count() 确保中文字符正确计数

/// 文本块结构体，表示分块后的单个文本片段
pub struct TextChunk {
    /// 文本内容
    pub content: String,
    /// 来源文件路径
    pub source_file: String,
    /// 块序号（从 0 开始）
    pub chunk_index: usize,
}

/// 文本分块器，按字符数将文本切分为带重叠的块
pub struct ChunkSplitter {
    /// 每块的字符数上限，默认 500
    pub chunk_size: usize,
    /// 相邻块之间的重叠字符数，默认 50
    pub overlap: usize,
}

impl ChunkSplitter {
    /// 创建新的分块器实例
    ///
    /// # 参数
    /// - `chunk_size`: 每块最大字符数
    /// - `overlap`: 相邻块重叠字符数
    pub fn new(chunk_size: usize, overlap: usize) -> Self {
        Self { chunk_size, overlap }
    }

    /// 使用默认参数创建分块器（chunk_size=500, overlap=50）
    pub fn default() -> Self {
        Self {
            chunk_size: 500,
            overlap: 50,
        }
    }

    /// 将文本按字符数分块，保留重叠区域
    ///
    /// # 参数
    /// - `text`: 待分块的文本内容
    /// - `source_file`: 来源文件路径标识
    ///
    /// # 返回
    /// 分块后的 TextChunk 列表
    ///
    /// # 逻辑说明
    /// - 文本长度小于等于 chunk_size 时，作为单个块返回
    /// - 使用 .chars() 按字符（而非字节）计数，确保中文正确处理
    /// - 每个块之间有 overlap 个字符的重叠，保证上下文连续性
    pub fn split(&self, text: &str, source_file: &str) -> Vec<TextChunk> {
        // 空文本直接返回空列表
        if text.is_empty() {
            return Vec::new();
        }

        let chars: Vec<char> = text.chars().collect();
        let total_chars = chars.len();

        // 文本长度不超过 chunk_size，作为单个块返回
        if total_chars <= self.chunk_size {
            return vec![TextChunk {
                content: text.to_string(),
                source_file: source_file.to_string(),
                chunk_index: 0,
            }];
        }

        let mut chunks = Vec::new();
        let mut start = 0;
        let mut chunk_index = 0;

        while start < total_chars {
            // 计算当前块的结束位置
            let end = (start + self.chunk_size).min(total_chars);

            // 从 chars 切片构建字符串
            let content: String = chars[start..end].iter().collect();

            chunks.push(TextChunk {
                content,
                source_file: source_file.to_string(),
                chunk_index,
            });

            // 计算下一块的起始位置（前进 chunk_size - overlap 个字符）
            let step = self.chunk_size.saturating_sub(self.overlap);
            // 防止 step 为 0 导致无限循环
            let step = if step == 0 { 1 } else { step };
            start += step;
            chunk_index += 1;
        }

        chunks
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
    fn test_text_equal_to_chunk_size() {
        // 创建恰好 10 个字符的文本
        let splitter = ChunkSplitter::new(10, 3);
        let text = "一二三四五六七八九十";
        assert_eq!(text.chars().count(), 10);

        let chunks = splitter.split(text, "exact.txt");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].content, text);
    }

    #[test]
    fn test_basic_splitting_with_overlap() {
        // chunk_size=5, overlap=2, step=3, 文本 10 个字符
        // 块1: start=0, [0..5] = "一二三四五"
        // 块2: start=3, [3..8] = "四五六七八"
        // 块3: start=6, [6..10] = "七八九十" (4个字符，不足chunk_size)
        // 块4: start=9, [9..10] = "十" (1个字符)
        let splitter = ChunkSplitter::new(5, 2);
        let text = "一二三四五六七八九十";
        let chunks = splitter.split(text, "test.md");

        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].content, "一二三四五");
        assert_eq!(chunks[0].chunk_index, 0);
        assert_eq!(chunks[1].content, "四五六七八");
        assert_eq!(chunks[1].chunk_index, 1);
        assert_eq!(chunks[2].content, "七八九十");
        assert_eq!(chunks[2].chunk_index, 2);
        assert_eq!(chunks[3].content, "十");
        assert_eq!(chunks[3].chunk_index, 3);
    }

    #[test]
    fn test_chinese_character_counting() {
        // 确保按字符而非字节计数
        // 中文字符在 UTF-8 中占 3 字节，但应按 1 个字符计数
        let splitter = ChunkSplitter::new(3, 1);
        let text = "你好世界测试";
        // 6 个字符，chunk_size=3, overlap=1, step=2
        // 块1: [0..3] = "你好世"
        // 块2: [2..5] = "世界测"
        // 块3: [4..6] = "测试"
        let chunks = splitter.split(text, "chinese.txt");

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "你好世");
        assert_eq!(chunks[1].content, "世界测");
        assert_eq!(chunks[2].content, "测试");
    }

    #[test]
    fn test_mixed_chinese_english() {
        // 混合中英文文本
        let splitter = ChunkSplitter::new(5, 1);
        let text = "Hello你好World";
        // 10 个字符: H,e,l,l,o,你,好,W,o,r,l,d -> 12 个字符
        let char_count = text.chars().count();
        assert_eq!(char_count, 12);

        let chunks = splitter.split(text, "mixed.txt");
        // step = 5 - 1 = 4
        // 块1: [0..5] = "Hello"
        // 块2: [4..9] = "o你好Wo"
        // 块3: [8..12] = "orld"
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "Hello");
        assert_eq!(chunks[1].content, "o你好Wo");
        assert_eq!(chunks[2].content, "orld");
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
        assert_eq!(splitter.chunk_size, 500);
        assert_eq!(splitter.overlap, 50);
    }

    #[test]
    fn test_no_overlap() {
        // overlap=0 时不应有重叠
        let splitter = ChunkSplitter::new(3, 0);
        let text = "一二三四五六七八九";
        // step = 3 - 0 = 3
        // 块1: [0..3] = "一二三"
        // 块2: [3..6] = "四五六"
        // 块3: [6..9] = "七八九"
        let chunks = splitter.split(text, "test.txt");

        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[0].content, "一二三");
        assert_eq!(chunks[1].content, "四五六");
        assert_eq!(chunks[2].content, "七八九");
    }

    #[test]
    fn test_overlap_larger_than_chunk_size_no_infinite_loop() {
        // overlap >= chunk_size 时，step 应至少为 1，避免无限循环
        let splitter = ChunkSplitter::new(3, 5);
        let text = "一二三四五六";
        let chunks = splitter.split(text, "test.txt");

        // 应该能正常完成，不会无限循环
        assert!(!chunks.is_empty());
        // 每个块最多 3 个字符
        for chunk in &chunks {
            assert!(chunk.content.chars().count() <= 3);
        }
    }

    #[test]
    fn test_long_chinese_text() {
        // 模拟较长的中文文本，使用默认参数
        let splitter = ChunkSplitter::new(500, 50);
        // 创建 1000 个中文字符的文本
        let text: String = "这是测试文本内容。".chars().cycle().take(1000).collect();
        let chunks = splitter.split(&text, "long_doc.txt");

        // step = 500 - 50 = 450
        // 预期块数: ceil((1000 - 500) / 450) + 1 = ceil(500/450) + 1 = 2 + 1 = 3
        assert!(chunks.len() >= 2);

        // 第一个块应该有 500 个字符
        assert_eq!(chunks[0].content.chars().count(), 500);

        // 最后一个块字符数应 <= chunk_size
        let last = chunks.last().unwrap();
        assert!(last.content.chars().count() <= 500);
    }
}
