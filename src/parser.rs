// 文档解析器模块
// 根据文件格式提取文本内容，支持 TXT/MD/PDF/DOCX/DOC/XLSX/XLS/CSV/PPTX/PPT/RTF/WPS/ET/DPS

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

        let (content, _page_count) = match extension.as_str() {
            "txt" => (self.parse_txt(file_path, &file_name)?, None),
            "md" => (self.parse_md(file_path, &file_name)?, None),
            "pdf" => self.parse_pdf(file_path, &file_name)?,
            "docx" | "wps" => (self.parse_docx(file_path, &file_name)?, None),
            "doc" => (self.parse_doc(file_path, &file_name)?, None),
            "xlsx" | "et" => (self.parse_xlsx(file_path, &file_name)?, None),
            "xls" => (self.parse_xls(file_path, &file_name)?, None),
            "csv" => (self.parse_csv(file_path, &file_name)?, None),
            "rtf" => (self.parse_rtf(file_path, &file_name)?, None),
            "pptx" | "dps" => (self.parse_pptx(file_path, &file_name)?, None),
            "ppt" => (self.parse_ppt(file_path, &file_name)?, None),
            _ => {
                return Err(CoiError::ParseError {
                    file: file_name,
                    reason: format!("不支持的文件格式: .{}", extension),
                });
            }
        };

        Ok(ParseResult {
            content,
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
            // 使用 catch_unwind 捕获可能的 panic
            std::panic::catch_unwind(|| pdf_extract::extract_text(&path))
        });

        match handle.join() {
            Ok(Ok(Ok(content))) => Ok((content, None)),
            Ok(Ok(Err(e))) => Err(CoiError::ParseError {
                file: name,
                reason: format!("PDF 解析失败: {}", e),
            }),
            Ok(Err(_)) | Err(_) => Err(CoiError::ParseError {
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

    fn parse_doc(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        let doc = office_oxide::Document::open(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("DOC 解析失败: {}", e),
        })?;

        Ok(doc.plain_text().to_string())
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

    fn parse_xls(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        use calamine::{open_workbook, Reader, Xls};

        let mut workbook: Xls<_> =
            open_workbook(file_path).map_err(|e| CoiError::ParseError {
                file: file_name.to_string(),
                reason: format!("XLS 打开失败: {}", e),
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

    /// 解析 RTF 文件：提取 RTF 文档中的纯文本内容
    /// RTF 格式中，可读文本以明文形式存储，控制字以 \ 开头
    fn parse_rtf(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        let bytes = std::fs::read(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("读取 RTF 文件失败: {}", e),
        })?;

        // 尝试多种编码读取
        let content = Self::decode_bytes(&bytes);
        
        // 从 RTF 中提取可读文本 - 简单且可靠的方法
        let mut result = Vec::new();
        let mut in_brace_depth: usize = 0;
        let mut skip_dest: bool = false;
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;
        
        while i < chars.len() {
            let c = chars[i];
            
            if c == '{' {
                in_brace_depth += 1;
                i += 1;
                // 检查是否是特殊目标（如字体表、颜色表等），需要跳过
                if i < chars.len() && chars[i] == '\\' {
                    let mut j = i + 1;
                    let mut word = String::new();
                    while j < chars.len() && chars[j].is_ascii_alphabetic() {
                        word.push(chars[j]);
                        j += 1;
                    }
                    if word == "fonttbl" || word == "colortbl" || word == "generator" || word == "*" {
                        skip_dest = true;
                    }
                }
                continue;
            }
            
            if c == '}' {
                in_brace_depth = in_brace_depth.saturating_sub(1);
                skip_dest = false;
                i += 1;
                continue;
            }
            
            if in_brace_depth > 1 || skip_dest {
                i += 1;
                continue;
            }
            
            if c == '\\' {
                // 检查是否是 RTF 控制字
                if i + 1 < chars.len() {
                    let next_c = chars[i + 1];
                    
                    // 跳过 RTF 控制字（以字母开头）
                    if next_c.is_ascii_alphabetic() {
                        let mut j = i + 1;
                        let mut word = String::new();
                        while j < chars.len() && chars[j].is_ascii_alphabetic() {
                            word.push(chars[j]);
                            j += 1;
                        }
                        
                        // 处理特殊控制字
                        if word == "par" || word == "line" {
                            result.push('\n');
                        } else if word == "tab" {
                            result.push('\t');
                        }
                        
                        i = j;
                        // 跳过数字参数
                        while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '-') {
                            i += 1;
                        }
                        // 跳过可能的空格
                        if i < chars.len() && chars[i] == ' ' {
                            i += 1;
                        }
                        continue;
                    }
                    
                    // 如果是特殊转义字符
                    if next_c == '\\' || next_c == '{' || next_c == '}' {
                        result.push(next_c);
                        i += 2;
                        continue;
                    }
                    
                    // 其他单个字符转义，跳过反斜杠
                    i += 1;
                    continue;
                }
            }
            
            // 普通字符，只要在第一层就收集
            result.push(c);
            i += 1;
        }
        
        // 清理和规范化文本
        let text = result.into_iter().collect::<String>();
        // 简单清理：保留中文、英文、常见标点，过滤掉可能的控制字符
        let cleaned: String = text.chars()
            .filter(|&c| {
                (c as u32 >= 0x4E00 && c as u32 <= 0x9FFF) || // 中文
                c.is_ascii_alphabetic() || c.is_ascii_punctuation() || c.is_whitespace()
            })
            .collect();
        let final_cleaned = Self::clean_text(&cleaned);
        
        if final_cleaned.trim().is_empty() {
            return Err(CoiError::ParseError {
                file: file_name.to_string(),
                reason: "RTF 文件内容为空或无法提取文本".to_string(),
            });
        }
        
        Ok(final_cleaned)
    }

    /// 解析 PPTX 文件：PPTX 本质上是 ZIP 压缩包，包含 XML 文件
    fn parse_pptx(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        let bytes = std::fs::read(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("读取 PPTX 文件失败: {}", e),
        })?;

        let cursor = std::io::Cursor::new(bytes);
        let mut archive = zip::ZipArchive::new(cursor).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("PPTX 文件损坏: {}", e),
        })?;

        let mut all_text = Vec::new();

        // PPTX 中幻灯片内容在 ppt/slides/ 目录下
        for i in 1..=100 {
            let slide_path = format!("ppt/slides/slide{}.xml", i);
            let file_exists = archive.by_name(&slide_path).is_ok();
            
            if file_exists {
                if let Ok(mut file) = archive.by_name(&slide_path) {
                    use std::io::Read;
                    let mut xml_content = String::new();
                    if file.read_to_string(&mut xml_content).is_ok() {
                        let slide_text = Self::extract_text_from_xml(&xml_content);
                        if !slide_text.trim().is_empty() {
                            all_text.push(format!("=== 第 {} 页 ===\n{}", i, slide_text));
                        }
                    }
                }
            } else {
                break;
            }
        }

        // 如果没找到幻灯片，尝试从所有文件列表中提取
        if all_text.is_empty() {
            // 先收集所有文件名（释放不可变借用）
            let file_names: Vec<String> = archive.file_names()
                .map(|s| s.to_string())
                .collect();
            
            // 再逐个处理（可变借用 archive）
            for name in &file_names {
                if name.contains("slide") && name.ends_with(".xml") {
                    if let Ok(mut file) = archive.by_name(name) {
                        use std::io::Read;
                        let mut xml_content = String::new();
                        if file.read_to_string(&mut xml_content).is_ok() {
                            let text = Self::extract_text_from_xml(&xml_content);
                            if !text.trim().is_empty() {
                                all_text.push(text);
                            }
                        }
                    }
                }
            }
        }

        let result = all_text.join("\n\n");
        if result.trim().is_empty() {
            return Err(CoiError::ParseError {
                file: file_name.to_string(),
                reason: "PPTX 文件中未找到可提取的文本内容".to_string(),
            });
        }

        Ok(result)
    }

    /// 解析 PPT 文件：旧版 PowerPoint 格式（二进制）
    /// 由于 PPT 是二进制格式，尝试提取其中的文本片段
    fn parse_ppt(&self, file_path: &Path, file_name: &str) -> Result<String, CoiError> {
        let bytes = std::fs::read(file_path).map_err(|e| CoiError::ParseError {
            file: file_name.to_string(),
            reason: format!("读取 PPT 文件失败: {}", e),
        })?;

        // PPT 二进制文件中，文本通常以特定模式存储
        // 尝试提取可读的中英文字符序列
        let mut text_parts = Vec::new();
        let mut current_text = Vec::new();
        let mut consecutive_printable = 0;
        const MIN_TEXT_LENGTH: usize = 2;

        for &byte in &bytes {
            let c = byte as char;
            
            // 判断是否为可打印字符（包括中文）
            let is_printable = c.is_alphanumeric() || 
                               c.is_whitespace() || 
                               c == '.' || c == ',' || c == '!' || c == '?' ||
                               c == '(' || c == ')' || c == '-' || c == ':' ||
                               c == '"' || c == '\'' || c == ';';
            
            if is_printable && byte >= 32 {
                current_text.push(c);
                if c.is_alphanumeric() {
                    consecutive_printable += 1;
                } else {
                    consecutive_printable = 0;
                }
            } else {
                // 非打印字符
                if consecutive_printable >= MIN_TEXT_LENGTH && !current_text.iter().all(|c| c.is_ascii_punctuation() && *c != '.') {
                    let text = current_text.iter().collect::<String>().trim().to_string();
                    if !text.is_empty() && text.chars().any(|c| c.is_alphanumeric()) {
                        text_parts.push(text);
                    }
                }
                current_text.clear();
                consecutive_printable = 0;
            }
        }

        // 处理最后一段文本
        if consecutive_printable >= MIN_TEXT_LENGTH {
            let text = current_text.iter().collect::<String>().trim().to_string();
            if !text.is_empty() {
                text_parts.push(text);
            }
        }

        let result = text_parts.join("\n");
        
        if result.trim().is_empty() {
            return Err(CoiError::ParseError {
                file: file_name.to_string(),
                reason: "PPT 文件中未找到可提取的文本内容（可能需要手动转换为 PPTX 格式）".to_string(),
            });
        }

        Ok(Self::clean_text(&result))
    }

    /// 从字节数组尝试多种编码解码
    fn decode_bytes(bytes: &[u8]) -> String {
        // 首先尝试 UTF-8
        if let Ok(s) = std::str::from_utf8(bytes) {
            return s.to_string();
        }

        // 尝试 GBK/GB2312（Windows 中文常用）
        let (s, _, had_errors) = encoding_rs::GBK.decode(bytes);
        if !had_errors {
            return s.to_string();
        }

        // 尝试 GB18030
        let (s, _, had_errors) = encoding_rs::GB18030.decode(bytes);
        if !had_errors {
            return s.to_string();
        }

        // 尝试 Big5（繁体中文）
        let (s, _, had_errors) = encoding_rs::BIG5.decode(bytes);
        if !had_errors {
            return s.to_string();
        }

        // 尝试 Latin-1 作为最后手段
        let (s, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
        s.to_string()
    }

    /// 从 XML 字符串中提取文本内容
    fn extract_text_from_xml(xml: &str) -> String {
        let mut results = Vec::new();
        let mut in_tag = false;
        let mut current_text = Vec::new();
        let bytes = xml.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            let b = bytes[i];
            
            if b == b'<' {
                in_tag = true;
                if !current_text.is_empty() {
                    let text = current_text.iter().collect::<String>().trim().to_string();
                    if !text.is_empty() {
                        results.push(text);
                    }
                    current_text.clear();
                }
            } else if b == b'>' {
                in_tag = false;
            } else if !in_tag {
                current_text.push(b as char);
            }
            
            i += 1;
        }

        // 处理最后一个文本节点
        if !current_text.is_empty() {
            let text = current_text.iter().collect::<String>().trim().to_string();
            if !text.is_empty() {
                results.push(text);
            }
        }

        Self::clean_text(&results.join(" "))
    }

    /// 清理和规范化文本
    fn clean_text(text: &str) -> String {
        let mut result = Vec::new();
        let mut last_was_whitespace = false;
        let mut last_was_newline = false;

        for c in text.chars() {
            // 规范化空白字符
            if c.is_whitespace() {
                if c == '\n' || c == '\r' {
                    if !last_was_newline {
                        result.push('\n');
                        last_was_newline = true;
                    }
                } else if !last_was_whitespace {
                    result.push(' ');
                    last_was_whitespace = true;
                }
                continue;
            }

            last_was_whitespace = false;
            last_was_newline = false;
            result.push(c);
        }

        // 移除首尾空白和多余换行
        let s = result.into_iter().collect::<String>();
        let lines: Vec<&str> = s.lines()
            .map(|l| l.trim())
            .filter(|l| !l.is_empty())
            .collect();
        
        lines.join("\n")
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
    }
}
