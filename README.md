# COI - 我问你答

**Chat Offline Intelligence** — 纯本地离线文档问答工具

COI 让你指定一个本地文档文件夹，自动构建向量知识库，通过命令行提问即可检索文档内容。全程不联网、不调用任何云端大模型，所有数据留在本地。

## 特性

- 🔒 **纯离线** — 零网络请求，无外部 AI 服务依赖
- ⚡ **原生性能** — Rust 编写，启动快、资源占用低
- 📄 **多格式支持** — TXT、MD、PDF、DOCX、XLSX、CSV
- 🧠 **语义检索** — 基于 ONNX Runtime 本地嵌入模型（BAAI/bge-small-zh-v1.5）
- 📦 **单文件分发** — 可编译为独立可执行文件，无需安装运行时
- 🗂️ **数据透明** — 所有数据存放在可见的 `coi_data/` 文件夹，删除即重置

## 安装

### 从源码编译

需要 [Rust 工具链](https://rustup.rs/)（1.70+）：

```bash
git clone <repo-url>
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

首次运行 `init` 或 `ask` 命令时，程序会自动下载嵌入模型（约 67MB）到 `model/` 目录。下载完成后即可完全离线使用。

如需预下载模型用于离线环境分发，可手动从 HuggingFace 下载 `BAAI/bge-small-zh-v1.5` 的 ONNX 版本放入 `model/` 目录。

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
| Excel | `.xlsx` | 逐 Sheet、逐行提取 |
| CSV | `.csv` | 逐行提取各字段 |

限制：
- 单文件大小上限 100MB
- 加密/受密码保护的文件会被跳过
- 空文件（0 字节）会被跳过

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

提供了一键构建脚本，在 macOS 上同时编译四个平台的可执行文件：

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

### 单平台编译

如果只需要编译当前平台：

```bash
cargo build --release
# 产物在 target/release/coi
```

### 分发清单

复制到其他电脑时需要包含以下内容：

```
分发包/
├── coi (或 coi.exe)    # 可执行文件 (~18MB)
└── model/              # 嵌入模型目录 (~67MB)
    └── ...             # 模型文件（首次运行自动下载，或预先打包）
```

- 如果目标电脑能联网：只复制 `coi` 可执行文件即可，首次运行自动下载模型
- 如果目标电脑完全离线：需要同时复制 `coi` 和 `model/` 目录
- `coi_data/` 目录是运行时生成的用户数据，不需要复制

## 技术架构

- **语言**: Rust
- **CLI 框架**: clap 4
- **嵌入模型**: BAAI/bge-small-zh-v1.5（384 维，通过 fastembed-rs / ONNX Runtime）
- **向量检索**: ndarray 余弦相似度
- **文档解析**: pdf-extract、docx-rs、calamine、csv
- **序列化**: serde_json + bincode

## 项目目录结构

```
coi/
├── Cargo.toml              # Rust 项目配置文件（依赖、编译选项）
├── Cargo.lock              # 依赖版本锁定文件
├── README.md               # 本文件
├── .gitignore              # Git 忽略规则
│
├── src/                    # 源代码目录
│   ├── main.rs             # 程序入口，命令分发
│   ├── cli.rs              # 命令行参数定义（clap）
│   ├── error.rs            # 统一错误类型
│   ├── config.rs           # 配置文件读写（config.json）
│   ├── scanner.rs          # 文档扫描器（递归遍历文件夹）
│   ├── parser.rs           # 文档解析器（TXT/MD/PDF/DOCX/XLSX/CSV）
│   ├── splitter.rs         # 文本分块器（按字符数切分）
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
├── scripts/                # 构建与打包脚本
│   ├── build_all.sh        # 一键全平台编译（macOS + Windows）
│   └── package.sh          # 离线分发包打包（含模型文件）
│
├── .github/workflows/      # GitHub Actions CI
│   └── build-linux.yml     # Linux 版本自动构建
│
├── .kiro/                  # Kiro IDE 配置（开发辅助，不影响程序运行）
│   ├── specs/              # 需求/设计/任务文档
│   └── steering/           # AI 协作规则
│
├── dist/                   # [构建产物，git忽略] 编译后的可执行文件
│   ├── coi-macos           # macOS 通用版（Intel + M芯片）
│   └── coi-windows.exe     # Windows 版
│
├── packages/               # [构建产物，git忽略] 离线分发 zip 包
│   ├── coi-macos-offline.zip
│   └── coi-windows-offline.zip
│
└── target/                 # [构建产物，git忽略] Rust 编译缓存
```

**说明：**
- `target/`、`dist/`、`packages/` 是构建产物，已在 `.gitignore` 中排除，不需要手动管理
- `model/` 目录在首次运行时自动创建（下载模型），也已排除
- 日常开发只需关注 `src/` 和 `scripts/` 目录

## 许可证

MIT
