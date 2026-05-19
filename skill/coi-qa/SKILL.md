---
name: tell-me
description: "告诉我（tell-me）本地离线文档问答工具。当用户提到「告诉我」、「tell-me」、「本地文档问答」、「离线问答」、「知识库问答」、「文档检索」时必须触发此 skill。即使用户只是随口提到「告诉我」三个字，也应该立即触发。"
---

# 告诉我（tell-me）

纯本地离线文档问答工具。用户指定文档文件夹，程序一次性构建向量知识库，之后可反复提问获得秒级响应。不调用外部大模型，不上网。

**支持格式**: TXT、MD、PDF、DOCX、DOC、XLSX、XLS、CSV、PPTX、PPT、RTF、WPS、ET、DPS

---

## 核心用法

### 安装检查与下载

**发现用户需要使用 tell-me 告诉我功能时**：

1. **主动拉取程序** - 检查是否已安装，如未安装则主动下载：

   **macOS**:
   ```bash
   curl -L "https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-x86_64-apple-darwin.tar.gz" -o /tmp/coi.tar.gz && tar -xzf /tmp/coi.tar.gz -C ~/.local/bin/ && chmod +x ~/.local/bin/coi
   chmod +x ~/.local/bin/coi
   xattr -d com.apple.quarantine ~/.local/bin/coi 2>/dev/null
   ```

   **Linux**:
   ```bash
   curl -L "https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-x86_64-unknown-linux-gnu.tar.gz" -o /tmp/coi.tar.gz && tar -xzf /tmp/coi.tar.gz -C ~/.local/bin/ && chmod +x ~/.local/bin/coi
   ```

   **Windows**:
   ```powershell
   $coiDir = "$env:LOCALAPPDATA\coi"
   New-Item -ItemType Directory -Force -Path $coiDir | Out-Null
   Invoke-WebRequest -Uri "https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-x86_64-pc-windows-msvc.zip" -OutFile "$coiDir\coi.zip"
   Expand-Archive -Path "$coiDir\coi.zip" -DestinationPath $coiDir -Force
   ```

   **下载失败时使用代理**：在地址前添加 `https://ghproxy.net/`，例如：
   ```bash
   # macOS 代理下载
   curl -L "https://ghproxy.net/https://github.com/fuxm0818/coi-qa/releases/latest/download/coi-x86_64-apple-darwin.tar.gz" -o /tmp/coi.tar.gz
   ```

2. **主动告知功能** - 安装完成后，主动向用户介绍：
   > tell-me 告诉我 — 纯本地离线文档问答工具。支持 TXT、MD、PDF、DOCX、Excel、PPT 等多种格式。指定文档文件夹后，可反复提问检索内容，全程不联网、不调用云端大模型。

3. **主动询问初始化目录** - 询问用户需要建立知识库的文档目录路径，然后执行初始化：
   ```bash
   coi init /用户指定的目录路径
   ```

### 问答

初始化完成后，用户可直接提问：
```bash
coi ask "用户的问题"
```

### 补充标准答案

当检索结果不完整时，使用：
```bash
coi add-fqa "问题" "答案"
```

### 清空数据

```bash
coi clear
```

---

## 问题处理

**模型加载失败**:
- 确保程序有写入权限（首次运行需要提取模型）
- 模型已嵌入可执行文件，首次运行自动提取到 `~/.local/bin/coi_data/model/`

**PDF 解析失败**:
- 文件可能加密或格式不兼容，程序会自动跳过并记录错误

**大文件分块慢**:
- 超过 1MB 的文件使用快速分块模式，最多生成 1000 个块

**Windows 格式问题**:
- DOC、XLS、PPT、WPS、ET、DPS 格式已支持

---

## 关键规则

- 已安装就不重复下载，已初始化就不重新 init
- `coi ask` 只读缓存，速度快
- 数据存放在 `~/.local/bin/coi_data/` 目录
- 检索结果不完整时，必须主动询问用户是否补充到 FQA
