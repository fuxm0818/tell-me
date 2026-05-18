# Requirements Document

## Introduction

"我问你答"（COI）是一个纯本地离线文档问答工具。COI 全称 **Chat Offline Intelligence**（离线智能问答），亦可释义为 **Content Organize Inquiry**（内容整理检索），三字母短标识便于命令行输入。用户指定本地文档文件夹后，程序自动读取文件夹内各类文档并构建向量知识库，支持提问检索、补充标准答案、一键清空等功能。全程不调用任何外部大模型、不联网，所有内部数据与程序文件同级存放，便于管理和删除。

## Glossary

- **COI_System**：我问你答（COI）本地离线文档问答系统
- **Vector_DB**：向量数据库，用于存储文档片段的向量化表示，支持语义检索
- **FQA_Store**：标准问答存储，保存用户手动补充的问题-答案对
- **Document_Scanner**：文档扫描器，负责扫描指定文件夹内所有支持格式的文档
- **Config_Store**：配置存储，保存用户初始化时指定的文档文件夹路径
- **coi_data**：程序同级目录下的数据文件夹，存放所有内部数据
- **Supported_Formats**：支持的文档格式，包括 TXT、MD、PDF、DOCX、XLSX、CSV

## Requirements

### 需求 1：初始化命令（init）

**用户故事：** 作为用户，我希望通过 init 命令指定文档文件夹路径并构建向量知识库，以便后续进行文档问答。

#### 验收标准

1. WHEN 用户执行 init 命令并传入文档文件夹路径, THE COI_System SHALL 将该路径保存至 coi_data/config.json 配置文件中，若 coi_data 目录不存在则自动创建
2. WHEN 用户执行 init 命令并传入有效的文档文件夹路径, THE Document_Scanner SHALL 递归扫描该文件夹内所有 Supported_Formats 格式的文档，并向用户输出扫描到的文档数量
3. WHEN Document_Scanner 完成文档扫描, THE COI_System SHALL 对所有扫描到的文档进行向量化处理并存储至 coi_data/vector_db 目录，处理完成后向用户输出成功提示及已处理的文档数量
4. IF 用户传入的文档文件夹路径不存在, THEN THE COI_System SHALL 返回错误提示信息，包含所传入的无效路径
5. IF 指定文件夹内无任何支持格式的文档, THEN THE COI_System SHALL 返回提示信息，告知用户未找到可处理的文档并列出当前支持的文件格式
6. IF 向量化处理过程中部分文档处理失败, THEN THE COI_System SHALL 继续处理剩余文档，完成后向用户输出成功处理的文档数量及失败的文档列表
7. WHEN 用户对已初始化的文档文件夹再次执行 init 命令, THE COI_System SHALL 重新扫描并重建向量知识库，覆盖 coi_data/vector_db 中的已有数据

### 需求 2：提问查询命令（ask）

**用户故事：** 作为用户，我希望通过 ask 命令输入问题并获取知识库中的相关答案，以便快速获取文档中的信息。

#### 验收标准

1. WHEN 用户执行 ask 命令并传入问题, THE COI_System SHALL 读取 Config_Store 中保存的文档文件夹路径，对该文件夹执行全量扫描并重建 Vector_DB
2. WHEN Vector_DB 重建完成, THE COI_System SHALL 基于用户问题在 Vector_DB 中进行语义检索，返回相似度最高的前 5 条文档片段
3. WHEN Vector_DB 检索完成, THE COI_System SHALL 在 FQA_Store 中匹配与用户问题语义相似的标准答案，返回相似度最高的前 3 条匹配结果
4. WHEN Vector_DB 和 FQA_Store 检索均完成, THE COI_System SHALL 将文档片段结果与 FQA_Store 匹配结果分区展示输出给用户，每条结果标明其来源（文档片段或标准答案）
5. IF Vector_DB 检索到相关内容但 FQA_Store 未匹配到结果, THEN THE COI_System SHALL 仅输出 Vector_DB 检索到的文档片段
6. IF FQA_Store 匹配到结果但 Vector_DB 未检索到相关内容, THEN THE COI_System SHALL 仅输出 FQA_Store 匹配到的标准答案
7. IF 用户未执行过 init 命令（config.json 不存在）, THEN THE COI_System SHALL 返回错误提示要求用户先执行 init 命令
8. IF 用户传入的问题为空字符串或仅包含空白字符, THEN THE COI_System SHALL 返回错误提示要求用户输入有效的问题内容
9. IF Vector_DB 和 FQA_Store 均未检索到相关内容, THEN THE COI_System SHALL 返回提示信息告知用户未找到相关答案

### 需求 3：补充标准答案命令（add-fqa）

**用户故事：** 作为用户，我希望通过 add-fqa 命令补充标准问答对，以便后续提问相似问题时能获取精确答案。

#### 验收标准

1. WHEN 用户执行 add-fqa 命令并传入问题和标准答案, THE COI_System SHALL 将该问答对存储至 coi_data/fqa.json 文件中，并输出确认信息告知用户问答对已成功添加
2. WHEN 用户补充的问题与 FQA_Store 中已有问题完全一致（精确字符串匹配）, THE COI_System SHALL 更新该问题对应的标准答案，并输出确认信息告知用户答案已更新
3. IF 用户未提供问题或未提供答案（包括传入空字符串或仅含空白字符的内容）, THEN THE COI_System SHALL 返回错误提示要求提供完整的问答对，且不修改 fqa.json 文件
4. IF coi_data/fqa.json 文件不存在, THEN THE COI_System SHALL 自动创建该文件并写入当前问答对

### 需求 4：一键清空命令（clear）

**用户故事：** 作为用户，我希望通过 clear 命令一键删除所有程序内部数据，以便快速重置系统状态。

#### 验收标准

1. WHEN 用户执行 clear 命令, THE COI_System SHALL 删除 coi_data 文件夹及其内部所有文件（包括 config.json、fqa.json、vector_db 目录），并在删除完成后向用户输出提示信息表明数据已成功清除
2. WHEN 用户执行 clear 命令, THE COI_System SHALL 仅执行文件删除操作，不执行任何文档扫描或向量构建动作
3. IF coi_data 文件夹不存在, THEN THE COI_System SHALL 输出提示信息告知用户当前无数据需要清除，且不执行任何删除操作
4. IF 删除过程中发生错误（如文件被占用或权限不足）, THEN THE COI_System SHALL 终止删除操作，并向用户输出提示信息说明删除失败的原因

### 需求 5：文件存储规则

**用户故事：** 作为用户，我希望所有程序数据集中存放在可见的文件夹中，以便我能轻松管理和删除数据。

#### 验收标准

1. THE COI_System SHALL 将所有内部数据（配置、问答库、向量数据库）exclusively 存放在程序可执行文件同级目录下名为 coi_data 的文件夹中，不在该文件夹之外的任何位置写入持久化数据
2. WHEN COI_System 启动且 coi_data 文件夹不存在时，THE COI_System SHALL 自动创建 coi_data 文件夹并在其中生成以下文件结构：config.json（配置文件）、fqa.json（标准问答文件）、vector_db/（向量数据库目录）
3. THE COI_System SHALL 使用非隐藏的 coi_data 文件夹存储数据，不使用操作系统隐藏目录（如 ~/.config、%APPDATA% 等用户不可直接看到的路径）
4. WHEN 用户删除 coi_data 文件夹后重新启动 COI_System 时，THE COI_System SHALL 恢复至初始状态运行，等同于首次安装后启动，无报错且无残留数据影响系统行为

### 需求 6：文档格式支持

**用户故事：** 作为用户，我希望系统能处理多种常见文档格式，以便我可以将不同类型的文档纳入知识库。

#### 验收标准

1. THE Document_Scanner SHALL 支持读取和解析 TXT 格式文档，并提取其中全部纯文本内容
2. THE Document_Scanner SHALL 支持读取和解析 MD（Markdown）格式文档，并提取其中全部纯文本内容（保留标题层级结构）
3. THE Document_Scanner SHALL 支持读取和解析 PDF 格式文档，并按页面顺序提取其中可识别的文本内容
4. THE Document_Scanner SHALL 支持读取和解析 DOCX（Word）格式文档，并按段落顺序提取其中全部纯文本内容
5. THE Document_Scanner SHALL 支持读取和解析 XLSX（Excel）格式文档，并逐sheet、逐行提取单元格中的文本内容
6. THE Document_Scanner SHALL 支持读取和解析 CSV 格式文档，并逐行提取各字段的文本内容
7. WHEN Document_Scanner 遇到不支持的文件格式, THE Document_Scanner SHALL 跳过该文件，记录一条包含文件名和跳过原因的日志，并继续处理其他文件
8. IF 文档文件大小超过 100 MB, THEN THE Document_Scanner SHALL 跳过该文件，记录一条包含文件名和跳过原因的日志，并继续处理其他文件
9. IF 文档文件已加密、受密码保护或文件内容损坏导致无法解析, THEN THE Document_Scanner SHALL 跳过该文件，记录一条包含文件名和失败原因的日志，并继续处理其他文件
10. IF 文档文件内容为空（0字节）, THEN THE Document_Scanner SHALL 跳过该文件，记录一条包含文件名的日志，并继续处理其他文件

### 需求 7：离线运行与跨平台打包

**用户故事：** 作为用户，我希望程序完全离线运行且可打包为独立可执行文件，以便我无需安装任何依赖即可使用。

#### 验收标准

1. WHILE COI_System 运行期间, THE COI_System SHALL 不发起任何网络请求（包括 DNS 查询、HTTP/HTTPS 请求、WebSocket 连接等任何形式的网络通信），在完全断网的环境下所有功能正常运行
2. THE COI_System SHALL 不调用任何外部大语言模型服务，所有文本处理与问答功能均由本地嵌入的模型或算法完成
3. THE COI_System SHALL 支持打包为 Windows（10 及以上）平台独立可执行文件，打包完成后生成单个可执行文件或自包含目录
4. THE COI_System SHALL 支持打包为 macOS（12 Monterey 及以上）平台独立可执行文件，打包完成后生成单个可执行文件或自包含目录
5. THE COI_System SHALL 支持打包为 Linux（Ubuntu 20.04 及以上或同等 glibc 版本的发行版）平台独立可执行文件，打包完成后生成单个可执行文件或自包含目录
6. WHEN 用户在目标平台上首次启动打包后的独立可执行文件时, THE COI_System SHALL 无需用户预先安装 Python 或任何其他运行时依赖，在 30 秒内完成启动并显示可交互的用户界面
7. WHEN 在完全无网络连接的环境中运行打包后的可执行文件时, THE COI_System SHALL 成功启动并能完成文档加载与问答的完整功能流程，功能表现与有网络环境下一致

### 需求 8：向量库实时同步

**用户故事：** 作为用户，我希望每次提问时知识库都与文档文件夹保持同步，以便我总能获取最新文档内容的答案。

#### 验收标准

1. WHEN 用户执行 ask 命令, THE COI_System SHALL 在执行检索前对用户指定的文档文件夹及其所有子目录执行全量扫描，将所有支持格式的文件内容重新构建为新的 Vector_DB，完全替换先前的 Vector_DB
2. WHEN 文档文件夹中有新增文件, THE COI_System SHALL 在下次 ask 命令执行时将新文件内容纳入检索范围
3. WHEN 文档文件夹中有文件被修改, THE COI_System SHALL 在下次 ask 命令执行时反映修改后的内容
4. WHEN 文档文件夹中有文件被删除, THE COI_System SHALL 在下次 ask 命令执行时不再返回已删除文件的内容
5. IF 用户指定的文档文件夹路径不存在或无法访问, THEN THE COI_System SHALL 中止 ask 流程并向用户显示错误信息，指明文件夹路径无效
6. IF 用户指定的文档文件夹为空（不包含任何支持格式的文件）, THEN THE COI_System SHALL 向用户显示提示信息，说明未找到可索引的文档，并跳过检索步骤
