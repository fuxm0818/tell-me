// 文档解析器模块
// 根据文件格式提取文本内容，支持 TXT/MD/PDF/DOCX/XLSX/CSV

use std::path::Path;

use crate::error::CoiError;

/// 尝试多种编码读取文件内容
fn read_file_with_fallback(file_path: &Path) -> Result<String, std::io::Error> {
    // 首先尝试 UTF-8
    if let Ok(content) = std::fs::read_to_string(file_path) {
        return Ok(content);
    }
    
    // 尝试其他编码
    let bytes = std::fs::read(file_path)?;
    
    // 尝试 GBK/GB2312 编码（Windows 中文常用）
    let (content, _, had_errors) = encoding_rs::GBK.decode(&bytes);
    if !had_errors && !content.is_empty() {
        return Ok(content.to_string());
    }
    
    // 尝试 GB18030 编码（更完整的中文编码）
    let (content, _, had_errors) = encoding_rs::GB18030.decode(&bytes);
    if !had_errors && !content.is_empty() {
        return Ok(content.to_string());
    }
    
    // 尝试 ISO-8859-1（Latin-1）作为最后的尝试
    let (content, _, _) = encoding_rs::WINDOWS_1252.decode(&bytes);
    if !content.is_empty() {
        return Ok(content.to_string());
    }
    
    Err(std::io::Error::new(
        std::io::ErrorKind::InvalidData,
        "无法识别文件编码",
    ))
}

/// 文档解析结果
pub struct ParseResult {
    /// 提取的文本内容
    pub content: String,
    /// 文档元数据
    pub metadata: DocMetadata,
}

/// 文档元数据
pub struct DocMetadata {
    /// 文件名
    pub file_name: String,
    /// 文件类型（扩展名）
    pub file_type: String,
    /// 页数（仅 PDF 有值）
    pub page_count: Option<usize>,
}

/// 文档解析器
/// 根据文件扩展名分发到对应的解析逻辑
pub struct DocumentParser;

impl DocumentParser {
    /// 创建新的文档解析器实例
    pub fn new() -> Self {
        Self
    }

    /// 根据文件扩展名调用对应解析器
    pub fn parse(&self, file_path: &Path) -> Result<ParseResult, CoiError> {
        let file_name = file_path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let extension = file_path
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();

        let (content, page_count) = match extension.as_str() {
            "txt" => (self.parse_txt(file_path, &file_name)?, None),
            "md" => (self.parse_md(file_path, &file_name)?, None),
            "pdf" => self.parse_pdf(file_path, &file_name)?,
            "docx" => (self.parse_docx(file_path, &file_name)?, None),
            "xlsx" => (self.parse_xlsx(file_path, &file_name)?, None),
            "csv" => (self.parse_csv(file_path, &file_name)?, None),
            _ => {
                return Err(CoiError::ParseError {
                    file: file_name,
                    reason: format!("不支持的文件格式: .{}", extension),
                });
            }
        };

        Ok(ParseResult {
            content,
            metadata: DocMetadata {
                file_name,
                file_type: extension,
                page_count,
            },
        })
    }

    /// 解析 TXT 文件：尝试多种编码读取全部文本内容
    fn parse_txt(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        read_file_with_fallback(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("读取文本文件失败: {}", e),
        })
    }

    /// 解析 MD 文件：尝试多种编码读取全部内容，保留标题层级结构
    fn parse_md(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        read_file_with_fallback(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("读取 Markdown 文件失败: {}", e),
        })
    }

    /// 解析 PDF 文件：使用 pdf-extract 按页面顺序提取文本
    /// 在独立线程中运行以隔离可能的 panic（pdf-extract 对某些编码会触发 assert 失败）
    fn parse_pdf(
        &self,
        file_path: &Path,
        file_name: &str,
    ) -> Result<(String, Option<usize>), CoiError> {
        let path = file_path.to_path_buf();
        let name = file_name.to_string();

        // 在独立线程中运行 PDF 解析，捕获 panic
        let handle = std::thread::spawn(move || {
            pdf_extract::extract_text(&path)
        });

        match handle.join() {
            Ok(Ok(content)) => Ok((content, None)),
            Ok(Err(e)) => Err(CoiError::ParseError {
                file: name,
                reason: format!("PDF 解析失败: {}", e),
            }),
            Err(_) => Err(CoiError::ParseError {
                file: name,
                reason: "PDF 解析失败: 文件格式不兼容（编码不支持）".to_string(),
            }),
        }
    }

    /// 解析 DOCX 文件：使用 docx-rs 按段落顺序提取文本
    fn parse_docx(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        let bytes = std::fs::read(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("读取 DOCX 文件失败: {}", e),
        })?;

        let docx = docx_rs::read_docx(&bytes).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("DOCX 解析失败: {}", e),
        })?;

        let mut paragraphs: Vec<String> = Vec::new();

        for child in docx.document.children {
            if let docx_rs::DocumentChild::Paragraph(paragraph) = child {
                let mut para_text = String::new();
                for content in &paragraph.children {
                    if let docx_rs::ParagraphChild::Run(run) = content {
                        for run_child in &run.children {
                            if let docx_rs::RunChild::Text(text) = run_child {
                                para_text.push_str(&text.text);
                            }
                        }
                    }
                }
                if !para_text.is_empty() {
                    paragraphs.push(para_text);
                }
            }
        }

        Ok(paragraphs.join("\n"))
    }

    /// 解析 XLSX 文件：使用 calamine 逐 sheet、逐行提取单元格文本
    fn parse_xlsx(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        use calamine::{open_workbook, Reader, Xlsx};

        let mut workbook: Xlsx<_> =
            open_workbook(file_path).map_err(|e| CoiError::ParseError {
                file: file_name.to_string(),
                reason: format!("XLSX 打开失败: {}", e),
            })?;

        let mut all_text = Vec::new();
        let sheet_names: Vec<String> = workbook.sheet_names().to_vec();

        for sheet_name in &sheet_names {
            if let Ok(range) = workbook.worksheet_range(sheet_name) {
                for row in range.rows() {
                    let row_text: Vec<String> = row
                        .iter()
                        .map(|cell| cell.to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !row_text.is_empty() {
                        all_text.push(row_text.join("\t"));
                    }
                }
            }
        }

        Ok(all_text.join("\n"))
    }

    /// 解析 CSV 文件：使用 csv crate 逐行提取各字段文本
    fn parse_csv(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        let mut reader =
            csv::ReaderBuilder::new()
                .has_headers(true)
                .from_path(file_path)
                .map_err(|e| CoiError::ParseError {
                    file: file_name.to_string(),
                    reason: format!("CSV 打开失败: {}", e),
                })?;

        let mut all_text = Vec::new();

        // 先提取表头
        if let Ok(headers) = reader.headers() {
            let header_text: Vec<&str> = headers.iter().collect();
            if !header_text.is_empty() {
                all_text.push(header_text.join("\t"));
            }
        }

        // 逐行提取各字段
        for result in reader.records() {
            match result {
                Ok(record) => {
                    let row_text: Vec<&str> = record.iter().collect();
                    if !row_text.is_empty() {
                        all_text.push(row_text.join("\t"));
                    }
                }
                Err(e) => {
                    return Err(CoiError::ParseError {
                        file: file_name.to_string(),
                        reason: format!("CSV 行解析失败: {}", e),
                    });
                }
            }
        }

        Ok(all_text.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_txt_basic() {
        // 创建临时 TXT 文件
        let mut tmp = NamedTempFile::with_suffix(".txt").unwrap();
        let content = "这是一段测试文本\n第二行内容";
        tmp.write_all(content.as_bytes()).unwrap();
        tmp.flush().unwrap();

        let parser = DocumentParser::new();
        let result = parser.parse(tmp.path()).unwrap();

        assert_eq!(result.content, content);
        assert_eq!(result.metadata.file_type, "txt");
        assert!(result.metadata.page_count.is_none());
    }

    #[test]
    fn test_parse_md_preserves_headings() {
        // 创建临时 MD 文件，验证标题层级保留
        let mut tmp = NamedTempFile::with_suffix(".md").unwrap();
        let content = "# 一级标题\n\n## 二级标题\n\n正文内容\n\n### 三级标题\n\n更多内容";
        tmp.write_all(content.as_bytes()).unwrap();
        tmp.flush().unwrap();

        let parser = DocumentParser::new();
        let result = parser.parse(tmp.path()).unwrap();

        // MD 保留原始内容（包括标题标记）
        assert!(result.content.contains("# 一级标题"));
        assert!(result.content.contains("## 二级标题"));
        assert!(result.content.contains("### 三级标题"));
        assert!(result.content.contains("正文内容"));
        assert_eq!(result.metadata.file_type, "md");
    }

    #[test]
    fn test_parse_csv_basic() {
        // 创建临时 CSV 文件
        let mut tmp = NamedTempFile::with_suffix(".csv").unwrap();
        let csv_content = "姓名,年龄,城市\n张三,25,北京\n李四,30,上海";
        tmp.write_all(csv_content.as_bytes()).unwrap();
        tmp.flush().unwrap();

        let parser = DocumentParser::new();
        let result = parser.parse(tmp.path()).unwrap();

        // 验证表头和数据行都被提取
        assert!(result.content.contains("姓名"));
        assert!(result.content.contains("张三"));
        assert!(result.content.contains("李四"));
        assert!(result.content.contains("北京"));
        assert!(result.content.contains("上海"));
        assert_eq!(result.metadata.file_type, "csv");
    }

    #[test]
    fn test_parse_unsupported_format() {
        // 创建不支持格式的临时文件
        let mut tmp = NamedTempFile::with_suffix(".xyz").unwrap();
        tmp.write_all(b"some content").unwrap();
        tmp.flush().unwrap();

        let parser = DocumentParser::new();
        let result = parser.parse(tmp.path());

        assert!(result.is_err());
        if let Err(CoiError::ParseError { reason, .. }) = result {
            assert!(reason.contains("不支持的文件格式"));
        }
    }

    #[test]
    fn test_parse_nonexistent_file() {
        let parser = DocumentParser::new();
        let result = parser.parse(Path::new("/nonexistent/file.txt"));

        assert!(result.is_err());
        if let Err(CoiError::ParseError { reason, .. }) = result {
            assert!(reason.contains("读取文本文件失败"));
        }
    }

    #[test]
    fn test_parse_txt_empty_content() {
        // 空文件也应该能正常解析
        let mut tmp = NamedTempFile::with_suffix(".txt").unwrap();
        tmp.write_all(b"").unwrap();
        tmp.flush().unwrap();

        let parser = DocumentParser::new();
        let result = parser.parse(tmp.path()).unwrap();

        assert_eq!(result.content, "");
        assert_eq!(result.metadata.file_type, "txt");
    }
}
