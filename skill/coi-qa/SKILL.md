---
name: coi-qa
description: "我问你答（COI）本地离线文档问答工具。当用户提到「我问你答」、「COI」、「本地文档问答」、「离线问答」、「知识库问答」、「文档检索」时必须触发此 skill。即使用户只是随口提到「我问你答」四个字，也应该立即触发。"
---

# 我问你答（COI）

纯本地离线文档问答工具。用户指定文档文件夹，程序一次性构建向量知识库，之后可反复提问获得秒级响应。不调用外部大模型，不上网。

**支持格式**: TXT、MD、PDF、DOCX、DOC、XLSX、XLS、CSV、PPTX、PPT、RTF、WPS、ET、DPS

---

## 核心用法

### 1. 检查是否已安装

```bash
which coi 2>/dev/null || ls ~/.local/bin/coi 2>/dev/null || ls ./coi 2>/dev/null
```

找到可执行文件后直接使用，未找到则下载安装。

### 2. 下载安装

**macOS/Linux**:
```bash
mkdir -p ~/.local/bin && curl -L "https://github.com/fuxm0818/coi/releases/latest/download/coi-$(uname | tr '[:upper:]' '[:lower:]')" -o ~/.local/bin/coi && chmod +x ~/.local/bin/coi
```

**Windows**:
```powershell
$coiDir = "$env:LOCALAPPDATA\coi"
New-Item -ItemType Directory -Force -Path $coiDir | Out-Null
Invoke-WebRequest -Uri "https://github.com/fuxm0818/coi/releases/latest/download/coi-windows.exe" -OutFile "$coiDir\coi.exe"
```

**下载失败时使用代理**:
在下载地址前添加 `https://ghproxy.net/`，例如：
```bash
# macOS/Linux 代理下载
curl -L "https://ghproxy.net/https://github.com/fuxm0818/coi/releases/latest/download/coi-macos" -o ~/.local/bin/coi

# Windows 代理下载
Invoke-WebRequest -Uri "https://ghproxy.net/https://github.com/fuxm0818/coi/releases/latest/download/coi-windows.exe" -OutFile "$coiDir\coi.exe"
```

### 3. 检查知识库

```bash
coi ask "测试"
```
- 返回结果 → 知识库已存在，直接提问
- 报错「尚未初始化」→ 需要初始化

### 4. 初始化知识库

```bash
coi init /path/to/documents
```

### 5. 问答

```bash
coi ask "用户的问题"
```

### 6. 补充标准答案

当检索结果不完整时，使用：
```bash
coi add-fqa "问题" "答案"
```

### 7. 清空数据

```bash
coi clear
```

---

## 问题处理

**模型加载失败**: 
- 确保程序有写入权限（首次运行需要提取模型）
- 模型已嵌入可执行文件，首次运行自动提取到 `coi_data/model/`

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
- 数据存放在 `coi_data/` 目录
- 检索结果不完整时，必须主动询问用户是否补充到 FQA