# Implementation Plan: COI 本地离线文档问答工具

## Overview

基于 Rust 开发 COI（我问你答）CLI 工具，采用增量开发方式：先搭建项目骨架和核心接口，再逐步实现各服务层组件，最后将命令处理器串联起来。每个任务构建在前序任务之上，确保代码始终可编译运行。

## Tasks

- [x] 1. 项目初始化与核心基础设施
  - [x] 1.1 创建 Rust 项目结构和 Cargo.toml 配置
    - 使用 `cargo init` 创建项目
    - 配置 Cargo.toml 中所有依赖：clap、fastembed、ndarray、serde、serde_json、bincode、pdf-extract、docx-rs、calamine、csv、walkdir、chrono、anyhow、thiserror
    - 配置 `[profile.release]` 优化选项（opt-level="z"、lto=true、strip=true、codegen-units=1）
    - 创建 src/ 目录结构：main.rs、cli.rs、error.rs、config.rs、scanner.rs、parser.rs、splitter.rs、embedding.rs、vector_store.rs、fqa_store.rs、handlers/mod.rs、handlers/init.rs、handlers/ask.rs、handlers/add_fqa.rs、handlers/clear.rs
    - _需求: 5.1, 7.3, 7.4, 7.5_

  - [x] 1.2 实现统一错误类型（src/error.rs）
    - 使用 `thiserror` 定义 `CoiError` 枚举，包含所有错误变体：InvalidPath、NotInitialized、InvalidInput、ParseError、ModelError、ClearError、Other
    - 每个错误变体包含中文格式化的错误信息
    - 实现 `From<anyhow::Error>` 转换
    - _需求: 1.4, 1.5, 2.7, 2.8, 3.3, 4.4_

  - [x] 1.3 实现 CLI 命令定义（src/cli.rs）
    - 使用 clap derive 宏定义 `Cli` 结构体和 `Commands` 枚举
    - 实现 4 个子命令：Init（doc_path 参数）、Ask（question 参数）、AddFqa（question + answer 参数）、Clear
    - 添加全局 `--verbose` 选项
    - 在 main.rs 中完成命令分发骨架（各命令暂时输出占位信息）
    - _需求: 1, 2, 3, 4_

- [x] 2. 配置与存储基础组件
  - [x] 2.1 实现 ConfigStore（src/config.rs）
    - 定义 `Config` 结构体（doc_folder_path、last_init_time），使用 serde 派生序列化
    - 实现 `ConfigStore` 结构体，包含 config_path 字段
    - 实现 `new`、`save`、`load`、`exists` 方法
    - `save` 方法自动创建 coi_data 目录（如不存在）
    - `load` 方法在文件不存在时返回 `Ok(None)`
    - _需求: 1.1, 5.1, 5.2_

  - [ ]* 2.2 编写 ConfigStore 属性测试
    - **Property 1: 配置路径存储往返一致性**
    - 对任意有效路径字符串，save 后 load 应返回完全一致的路径
    - **验证: 需求 1.1**

  - [x] 2.3 实现 FQAStore（src/fqa_store.rs）
    - 定义 `FQAEntry` 结构体（question、answer、embedding、created_at、updated_at）
    - 定义 `FQAFile` 结构体（version、entries），用于 JSON 序列化
    - 实现 `FQAStore` 结构体，包含 fqa_path 和 entries 字段
    - 实现 `new`（加载已有文件或初始化空列表）、`add`（精确匹配更新或新增）、`search`（余弦相似度匹配 Top-K）、`save`（持久化到 fqa.json）方法
    - `add` 方法：问题完全一致时更新答案并返回 true，否则新增并返回 false
    - _需求: 3.1, 3.2, 3.3, 3.4_

  - [ ]* 2.4 编写 FQAStore 属性测试
    - **Property 7: 标准问答对存储往返一致性**
    - **Property 8: 重复问题更新幂等性**
    - 验证 add 后 save 再 load 能找到对应答案
    - 验证相同问题多次 add 后仅保留一条记录
    - **验证: 需求 3.1, 3.2**

- [x] 3. 检查点 - 确保基础组件测试通过
  - 确保所有测试通过，如有问题请询问用户。

- [x] 4. 文档扫描与解析
  - [x] 4.1 实现 DocumentScanner（src/scanner.rs）
    - 定义支持的扩展名集合：.txt、.md、.pdf、.docx、.xlsx、.csv
    - 定义 `ScanResult` 结构体（files、skipped、total_scanned）和 `SkipInfo` 结构体
    - 使用 `walkdir` 递归遍历目录
    - 实现文件过滤逻辑：检查扩展名、文件大小（>100MB 跳过）、空文件（0字节跳过）
    - 不支持的格式和异常文件记录到 skipped 列表
    - _需求: 1.2, 6.1-6.10, 8.1_

  - [ ]* 4.2 编写 DocumentScanner 属性测试
    - **Property 2: 文档扫描器正确识别支持格式**
    - **Property 12: 不支持格式跳过不变量**
    - 验证返回文件列表仅包含支持格式
    - 验证不支持格式的文件出现在 skipped 列表中
    - **验证: 需求 1.2, 6.7**

  - [x] 4.3 实现 DocumentParser（src/parser.rs）
    - 定义 `ParseResult` 结构体（content、metadata）和 `DocMetadata` 结构体
    - 实现 `parse` 方法，根据文件扩展名分发到对应解析逻辑
    - TXT/MD：使用 `std::fs::read_to_string`，MD 保留标题层级
    - PDF：使用 `pdf-extract` 按页面顺序提取文本
    - DOCX：使用 `docx-rs` 按段落顺序提取文本
    - XLSX：使用 `calamine` 逐 sheet、逐行提取单元格文本
    - CSV：使用 `csv` crate 逐行提取各字段文本
    - 解析失败时返回 `CoiError::ParseError`
    - _需求: 6.1, 6.2, 6.3, 6.4, 6.5, 6.6, 6.9_

  - [ ]* 4.4 编写 DocumentParser 属性测试
    - **Property 11: 文本格式文档解析往返一致性**
    - 对任意纯文本内容写入 TXT 文件后解析，提取内容应包含原始文本
    - **验证: 需求 6.1, 6.6**

  - [x] 4.5 实现 ChunkSplitter（src/splitter.rs）
    - 定义 `TextChunk` 结构体（content、source_file、chunk_index）
    - 实现按字符数分块逻辑，默认 chunk_size=500、overlap=50
    - 处理边界情况：文本长度小于 chunk_size 时作为单个块返回
    - 确保中文字符正确处理（按 char 而非 byte 计数）
    - _需求: 1.3, 2.2_

- [x] 5. 嵌入模型与向量存储
  - [x] 5.1 实现 EmbeddingService（src/embedding.rs）
    - 使用 `fastembed` crate 封装嵌入模型
    - 实现 `new` 方法：加载 `BAAI/bge-small-zh-v1.5` 模型，配置 cache_dir 为 model/ 目录
    - 实现 `encode_batch` 方法：批量将文本转为 384 维向量
    - 实现 `encode` 方法：单条文本向量化
    - 错误处理：模型文件缺失或损坏时返回 `CoiError::ModelError`
    - _需求: 7.1, 7.2, 7.6_

  - [x] 5.2 实现 VectorStore（src/vector_store.rs）
    - 定义 `SearchResult` 结构体（content、source、source_file、score）
    - 实现 `rebuild` 方法：将向量矩阵以 bincode 序列化存储到 embeddings.bin，元数据存储到 metadata.json
    - 实现 `query` 方法：使用 ndarray 计算余弦相似度，返回 Top-K 结果（降序排列）
    - 实现 `is_empty` 方法：检查向量库是否为空
    - _需求: 1.3, 2.2, 8.1_

  - [ ]* 5.3 编写 VectorStore 属性测试
    - **Property 4: 向量检索结果排序不变量**
    - 验证返回结果数量不超过 top_k，且按 score 降序排列
    - **验证: 需求 2.2**

  - [ ]* 5.4 编写 FQAStore 搜索属性测试
    - **Property 5: FQA 匹配结果排序不变量**
    - 验证 FQA search 返回结果数量不超过 3，且按 score 降序排列
    - **验证: 需求 2.3**

- [x] 6. 检查点 - 确保服务层组件测试通过
  - 确保所有测试通过，如有问题请询问用户。

- [x] 7. 命令处理器实现
  - [x] 7.1 实现 InitHandler（src/handlers/init.rs）
    - 验证传入路径是否存在，不存在返回 InvalidPath 错误
    - 创建 coi_data 目录（如不存在）
    - 保存路径到 config.json
    - 调用 DocumentScanner 扫描文档，输出扫描数量
    - 如无支持格式文档，输出提示并列出支持格式
    - 调用 DocumentParser 逐文件解析，调用 ChunkSplitter 分块
    - 调用 EmbeddingService 向量化，调用 VectorStore rebuild 存储
    - 部分文件失败时继续处理，最终输出成功数量和失败列表
    - 重复 init 时覆盖已有向量数据
    - _需求: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 1.7_

  - [x] 7.2 实现 AskHandler（src/handlers/ask.rs）
    - 验证 config.json 存在，不存在返回 NotInitialized 错误
    - 验证问题非空白，空白返回 InvalidInput 错误
    - 读取 config.json 获取文档路径，验证路径有效
    - 全量扫描 → 解析 → 分块 → 向量化 → 重建 VectorStore
    - 向量化用户问题，在 VectorStore 中检索 Top 5
    - 在 FQAStore 中语义匹配 Top 3
    - 分区展示结果：文档片段区域和标准答案区域
    - 处理各种空结果情况（仅文档、仅FQA、均无结果）
    - _需求: 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9, 8.1, 8.2, 8.3, 8.4, 8.5, 8.6_

  - [x] 7.3 实现 AddFqaHandler（src/handlers/add_fqa.rs）
    - 验证问题和答案非空白，空白返回 InvalidInput 错误
    - 加载或创建 FQAStore
    - 调用 EmbeddingService 向量化问题
    - 调用 FQAStore.add 添加/更新问答对
    - 保存 fqa.json
    - 输出确认信息（新增或更新）
    - _需求: 3.1, 3.2, 3.3, 3.4_

  - [x] 7.4 实现 ClearHandler（src/handlers/clear.rs）
    - 检查 coi_data 目录是否存在
    - 不存在时输出"无数据需要清除"提示
    - 存在时删除整个 coi_data 目录及其内容
    - 删除失败时输出错误原因（权限不足/文件占用）
    - 仅执行删除，不执行扫描或构建操作
    - _需求: 4.1, 4.2, 4.3, 4.4_

  - [ ]* 7.5 编写输入验证属性测试
    - **Property 6: 空白输入拒绝**
    - 验证任意空白字符串作为 ask 问题或 add-fqa 参数时被拒绝
    - **验证: 需求 2.8, 3.3**

- [x] 8. 检查点 - 确保命令处理器测试通过
  - 确保所有测试通过，如有问题请询问用户。

- [x] 9. 集成串联与端到端验证
  - [x] 9.1 完善 main.rs 命令分发逻辑
    - 将 CLI 命令分发连接到各 Handler
    - 实现 coi_data 路径解析逻辑（程序可执行文件同级目录）
    - 统一错误输出格式（捕获 CoiError 并格式化输出到 stderr）
    - verbose 模式下输出详细日志
    - _需求: 5.1, 5.3, 5.4_

  - [ ]* 9.2 编写集成测试
    - 测试完整 init → ask → add-fqa → clear 工作流
    - 测试文档变更后 ask 结果同步
    - 测试各文档格式解析（使用 tests/fixtures/ 下的测试文件）
    - 测试边界条件：0 字节文件、不支持格式、路径不存在
    - **验证: 需求 1-8 全覆盖**

  - [ ]* 9.3 编写向量库实时同步属性测试
    - **Property 10: 向量库实时同步一致性**
    - 验证文件新增/修改/删除后 ask 重建的向量库仅包含当前文件内容
    - **验证: 需求 8.1, 8.2, 8.3, 8.4**

  - [ ]* 9.4 编写容错性属性测试
    - **Property 3: 部分文档处理失败时的容错性**
    - 验证成功数 + 失败数 = 总文件数，且有效文档均被成功处理
    - **验证: 需求 1.6, 6.9**

  - [ ]* 9.5 编写清空命令属性测试
    - **Property 9: 清空命令完整性**
    - 验证 clear 后 coi_data 目录不再存在
    - **验证: 需求 4.1**

- [x] 10. 最终检查点 - 确保所有测试通过
  - 确保所有测试通过，如有问题请询问用户。

## Notes

- 标记 `*` 的任务为可选测试任务，可跳过以加速 MVP 开发
- 每个任务引用了具体的需求编号，确保可追溯性
- 检查点任务确保增量验证，避免问题累积
- 属性测试验证设计文档中定义的正确性属性
- 单元测试验证具体示例和边界条件
- 跨平台构建（cargo-xwin、cross）属于发布流程，不在编码任务范围内
- 模型文件（BAAI/bge-small-zh-v1.5）首次运行时自动下载，开发阶段需确保网络可用

## Task Dependency Graph

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "1.3"] },
    { "id": 2, "tasks": ["2.1", "2.3"] },
    { "id": 3, "tasks": ["2.2", "2.4", "4.1", "4.5"] },
    { "id": 4, "tasks": ["4.2", "4.3"] },
    { "id": 5, "tasks": ["4.4", "5.1"] },
    { "id": 6, "tasks": ["5.2"] },
    { "id": 7, "tasks": ["5.3", "5.4"] },
    { "id": 8, "tasks": ["7.1", "7.3", "7.4"] },
    { "id": 9, "tasks": ["7.2", "7.5"] },
    { "id": 10, "tasks": ["9.1"] },
    { "id": 11, "tasks": ["9.2", "9.3", "9.4", "9.5"] }
  ]
}
```
