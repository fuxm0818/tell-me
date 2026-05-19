# COI - 我问你答

**Chat Offline Intelligence** — 纯本地离线文档问答工具

COI 让你指定一个本地文档文件夹，自动构建向量知识库，通过命令行提问即可检索文档内容。全程不联网、不调用任何云端大模型，所有数据留在本地。

## 特性

- 🔒 **纯离线** — 零网络请求，无外部 AI 服务依赖
- ⚡ **原生性能** — Rust 编写，启动快、资源占用低
- 📄 **多格式支持** — TXT、MD、PDF、DOCX、DOC、XLSX、XLS、CSV、PPTX、PPT、RTF、WPS、ET、DPS
- 🧠 **语义检索** — 基于 ONNX Runtime 本地嵌入模型（BAAI/bge-small-zh-v1.5）
- 📦 **单文件分发** — 模型已嵌入可执行文件，分发时只需一个文件
- 🗂️ **数据透明** — 所有数据存放在可见的 `coi_data/` 文件夹，删除即重置

## 安装

### 从 GitHub Releases 下载（推荐）

直接下载预编译的可执行文件：

**Windows（PowerShell）：**
```powershell
$coiDir = "$env:LOCALAPPDATA\coi"
New-Item -ItemType Directory -Force -Path $coiDir | Out-Null
Invoke-WebRequest -Uri "https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-windows.exe" -OutFile "$coiDir\coi.exe"
& "$coiDir\coi.exe" --help
```

**如果下载失败，使用代理下载：**
```powershell
$coiDir = "$env:LOCALAPPDATA\coi"
New-Item -ItemType Directory -Force -Path $coiDir | Out-Null
curl.exe -L "https://ghproxy.net/https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-windows.exe" -o "$coiDir\coi.exe"
& "$coiDir\coi.exe" --help
```

**macOS：**
```bash
curl -L "https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-macos" -o /usr/local/bin/coi
chmod +x /usr/local/bin/coi
coi --help
```

**Linux：**
```bash
curl -L "https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-linux" -o /usr/local/bin/coi
chmod +x /usr/local/bin/coi
coi --help
```

### 从源码编译

需要 [Rust 工具链](https://rustup.rs/)（1.70+）：

```bash
git clone https://github.com/fuxm0818/coi.git
cd coi
cargo build --release
```

编译产物在 `target/release/coi`。

### macOS 首次运行

macOS 会拦截未签名的程序，首次使用前需要在终端执行：

```bash
xattr -d com.apple.quarantine ./coi-macos
```

执行一次即可，之后不会再弹窗。

### 模型文件

模型已嵌入可执行文件中，**分发时只需复制单个可执行文件**，无需额外拷贝模型目录。

首次运行时，程序会自动将模型提取到 `coi_data/model/` 目录，之后运行直接使用已提取的模型。

- **模型名称**: BAAI/bge-small-zh-v1.5（384 维中文优化嵌入模型）
- **模型大小**: 约 90MB（嵌入后可执行文件约 110MB）
- **模型格式**: ONNX（轻量化，跨平台）

## 使用方法

### 初始化知识库

```bash
coi init /path/to/your/documents
```

扫描指定文件夹内所有支持格式的文档，构建向量知识库。

### 提问查询

```bash
coi ask "如何配置数据库连接？"
```

每次提问会自动重新扫描文档文件夹，确保结果与最新文档同步。输出包含：
- 📄 文档检索结果（Top 5 相关片段）
- 💡 标准答案（如有匹配的 FQA 条目）

### 补充标准答案

```bash
coi add-fqa "项目的部署流程是什么？" "先执行 build，再通过 Docker 部署到生产环境"
```

添加自定义问答对，后续提问相似问题时会优先匹配返回。

### 一键清空

```bash
coi clear
```

删除所有程序数据（配置、向量库、FQA 问答库），恢复初始状态。

### 全局选项

```bash
coi --verbose ask "问题"   # 开启详细日志
coi --help                  # 查看帮助
```

## 支持的文档格式

| 格式 | 扩展名 | 说明 |
|------|--------|------|
| 纯文本 | `.txt` | 直接读取全部内容 |
| Markdown | `.md` | 保留标题层级结构 |
| PDF | `.pdf` | 按页面顺序提取文本 |
| Word | `.docx` | 按段落顺序提取文本 |
| Word（旧版） | `.doc` | 支持传统 Word 格式 |
| Excel | `.xlsx` | 逐 Sheet、逐行提取 |
| Excel（旧版） | `.xls` | 支持传统 Excel 格式 |
| CSV | `.csv` | 逐行提取各字段 |
| PowerPoint | `.pptx` | 逐幻灯片提取文本 |
| PowerPoint（旧版） | `.ppt` | 支持传统 PPT 格式 |
| RTF | `.rtf` | Rich Text Format 格式 |
| WPS 文字 | `.wps` | 金山文字格式 |
| WPS 表格 | `.et` | 金山表格格式 |
| WPS 演示 | `.dps` | 金山演示格式 |

限制：
- 单文件大小上限 100MB
- 加密/受密码保护的文件会被跳过
- 空文件（0 字节）会被跳过
- 隐藏文件（名称以 `.` 开头）会被跳过

**性能优化：**
- 大文件（超过 1MB）使用智能分块策略，避免产生过多小块
- 单文件最大块数限制为 1000 个，保证处理效率
- 日志类文件（如 HTTP 访问日志）自动合并短行，减少块数

**Windows 格式兼容：**
- 自动识别并处理传统 Windows 格式（DOC、XLS、PPT）
- 自动识别并处理 WPS 办公软件格式（WPS、ET、DPS）

## 数据存储

所有程序数据存放在可执行文件同级目录下的 `coi_data/` 文件夹：

```
coi_data/
├── config.json    # 配置文件（文档路径）
├── fqa.json       # 标准问答库
└── vector_db/     # 向量数据库
    ├── embeddings.bin
    └── metadata.json
```

直接删除 `coi_data/` 文件夹等同于执行 `coi clear`。

## 构建与分发

### 一键全平台构建

提供了一键构建脚本，在 macOS 上同时编译多平台的可执行文件：

```bash
# 首次使用需要赋予执行权限
chmod +x scripts/build_all.sh

# 执行全平台构建
./scripts/build_all.sh
```

构建完成后，所有产物在 `dist/` 目录：

```
dist/
├── coi-macos           # macOS 通用版（Intel + M芯片都能运行）
├── coi-windows.exe     # Windows 64位版本
└── coi-linux           # Linux 64位版本
```

#### 前置依赖

全平台构建需要以下工具（脚本会自动检查并提示安装）：

| 工具 | 用途 | 安装方式 |
|------|------|----------|
| Rust 工具链 | 编译 Rust 代码 | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| LLVM | cargo-xwin 依赖 | `brew install llvm` |
| cargo-xwin | 交叉编译到 Windows | `cargo install cargo-xwin` |
| cross | 交叉编译到 Linux | `cargo install cross` |
| Podman 或 Docker | cross 的容器引擎 | `brew install podman` 或安装 Docker Desktop |

> **提示**：如果不需要全平台构建，可以只安装 Rust 工具链，使用 `cargo build --release` 编译当前平台版本。

### 单平台编译

如果只需要编译当前平台：

```bash
cargo build --release
# 产物在 target/release/coi
```

### 分发清单

复制到其他电脑时**只需复制单个可执行文件**：

```
分发包/
└── coi (或 coi.exe)    # 可执行文件（模型已嵌入，约 110MB）
```

**分发说明：**
- 模型已嵌入可执行文件，分发时无需额外拷贝 `model/` 目录
- 首次运行时程序会自动提取模型到 `coi_data/model/`
- `coi_data/` 是运行时生成的用户数据目录，包含配置、向量库、FQA 问答库
- 直接删除 `coi_data/` 等同于执行 `coi clear`

## 技术架构

- **语言**: Rust
- **CLI 框架**: clap 4
- **嵌入模型**: BAAI/bge-small-zh-v1.5（384 维，通过 fastembed-rs / ONNX Runtime）
- **向量检索**: ndarray 余弦相似度
- **文档解析**: pdf-extract、docx-rs、calamine、csv、office-oxide
- **序列化**: serde_json + bincode

## 项目目录结构

```
coi/
├── Cargo.toml              # Rust 项目配置文件（依赖、编译选项）
├── Cargo.lock              # 依赖版本锁定文件
├── LICENSE                 # MIT 许可证
├── README.md               # 本文件
├── .gitignore              # Git 忽略规则
│
├── src/                    # 源代码目录
│   ├── main.rs             # 程序入口，命令分发
│   ├── cli.rs              # 命令行参数定义（clap）
│   ├── error.rs            # 统一错误类型
│   ├── config.rs           # 配置文件读写（config.json）
│   ├── scanner.rs          # 文档扫描器（递归遍历文件夹）
│   ├── parser.rs           # 文档解析器（TXT/MD/PDF/DOCX/DOC/XLSX/XLS/CSV/PPTX/PPT/RTF/WPS/ET/DPS）
│   ├── splitter.rs         # 文本分块器（智能切分）
│   ├── embedding.rs        # 嵌入模型封装（fastembed/ONNX）
│   ├── vector_store.rs     # 向量存储与检索（余弦相似度）
│   ├── fqa_store.rs        # 标准问答库管理
│   └── handlers/           # 命令处理器
│       ├── mod.rs          # 模块声明
│       ├── init.rs         # init 命令逻辑
│       ├── ask.rs          # ask 命令逻辑
│       ├── add_fqa.rs      # add-fqa 命令逻辑
│       └── clear.rs        # clear 命令逻辑
│
├── .github/workflows/      # GitHub Actions CI
│   └── release.yml         # 打 tag 时自动构建发布
│
├── dist/                   # [构建产物，git忽略] 编译后的可执行文件
│   ├── coi-macos           # macOS 通用版（Intel + M芯片）
│   ├── coi-windows.exe     # Windows 版
│   └── coi-linux           # Linux 版
│
│
└── target/                 # [构建产物，git忽略] Rust 编译缓存
```

**说明：**
- `target/`、`dist/`、`packages/` 是构建产物，已在 `.gitignore` 中排除，不需要手动管理
- `model/` 目录包含模型文件（开发时使用，编译时嵌入可执行文件）
- `coi_data/` 是运行时生成的用户数据目录，不在版本控制中
- 日常开发只需关注 `src/` 和 `scripts/` 目录

## 许可证

MIT
