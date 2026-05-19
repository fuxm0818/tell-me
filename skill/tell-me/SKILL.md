---
name: tell-me
description: "告诉我（tell-me）本地离线文档问答工具。**触发条件**：当用户的问题以「告诉我」、「tell-me」或「tell me」（支持多个空格）开头时触发此技能。其他情况如提到「本地文档问答」、「离线问答」、「知识库问答」、「文档检索」等关键词但不以这些前缀开头的，不得触发此技能。"
---

# 告诉我（tell-me）

纯本地离线文档问答工具。用户指定文档文件夹，程序一次性构建向量知识库，之后可反复提问获得秒级响应。不调用外部大模型，不上网。

**支持格式**: TXT、MD、PDF、DOCX、DOC、XLSX、XLS、CSV、PPTX、PPT、RTF、WPS、ET、DPS

**数据目录**:

- macOS / Linux: `~/.local/bin/tell_me_data/`
- Windows: `%LOCALAPPDATA%\tell-me\tell_me_data\`

---

## 核心用法

### 安装检查与版本更新

**发现用户需要使用 tell-me 告诉我功能时，按以下顺序执行**：

#### 第一步：检查是否已安装及版本

**macOS / Linux**:

```bash
# 检查是否已安装
which tell-me 2>/dev/null || echo "NOT_INSTALLED"

# 如果已安装，检查当前版本
tell-me --version 2>/dev/null
```

**Windows**:

```powershell
# 检查是否已安装
Get-Command tell-me -ErrorAction SilentlyContinue | Select-Object -ExpandProperty Source
if (-not $?) { Write-Output "NOT_INSTALLED" }

# 如果已安装，检查当前版本
tell-me --version 2>$null
```

#### 第二步：检查远程最新版本

**macOS / Linux**:

```bash
curl -sI "https://github.com/fuxm0818/tell-me/releases/latest" 2>/dev/null | grep -i "location:" | grep -oE "v[0-9]+\.[0-9]+\.[0-9]+"
```

**Windows**:

```powershell
try {
    $response = Invoke-WebRequest -Uri "https://github.com/fuxm0818/tell-me/releases/latest" -MaximumRedirection 0 -ErrorAction Stop
} catch {
    $_.Exception.Response.Headers.Location -match "v(\d+\.\d+\.\d+)" | Out-Null; $Matches[0]
}
```

如果远程版本获取失败（网络不通），跳过版本检查，使用本地已有版本继续。

#### 第三步：判断是否需要安装/更新

- **未安装** → 执行下载安装
- **已安装但版本落后** → 提示用户："检测到 tell-me 有新版本（远程 vX.Y.Z，本地 vA.B.C），是否更新？" 用户确认后执行更新
- **已安装且是最新版** → 跳过，直接进入下一步

#### 第四步：下载安装（需要时才执行）

按顺序尝试以下下载源，**第一个成功即停止**：

**macOS**:

```bash
# 方案1：GitHub 直连
curl -L "https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-apple-darwin.tar.gz" -o /tmp/tell-me.tar.gz

# 方案2：ghproxy 代理
curl -L "https://ghproxy.net/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-apple-darwin.tar.gz" -o /tmp/tell-me.tar.gz

# 方案3：gh-proxy.com 代理
curl -L "https://gh-proxy.com/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-apple-darwin.tar.gz" -o /tmp/tell-me.tar.gz

# 方案4：mirror.ghproxy.com 代理
curl -L "https://mirror.ghproxy.com/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-apple-darwin.tar.gz" -o /tmp/tell-me.tar.gz

# 方案5：gitee 镜像（备用）
curl -L "https://gitee.com/fuxm0818/tell-me/releases/download/latest/tell-me-x86_64-apple-darwin.tar.gz" -o /tmp/tell-me.tar.gz

# 下载成功后解压安装
mkdir -p ~/.local/bin
tar -xzf /tmp/tell-me.tar.gz -C ~/.local/bin/
chmod +x ~/.local/bin/tell-me
xattr -d com.apple.quarantine ~/.local/bin/tell-me 2>/dev/null
```

**Linux**:

```bash
# 方案1：GitHub 直连
curl -L "https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-unknown-linux-gnu.tar.gz" -o /tmp/tell-me.tar.gz

# 方案2：ghproxy 代理
curl -L "https://ghproxy.net/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-unknown-linux-gnu.tar.gz" -o /tmp/tell-me.tar.gz

# 方案3：gh-proxy.com 代理
curl -L "https://gh-proxy.com/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-unknown-linux-gnu.tar.gz" -o /tmp/tell-me.tar.gz

# 方案4：mirror.ghproxy.com 代理
curl -L "https://mirror.ghproxy.com/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-unknown-linux-gnu.tar.gz" -o /tmp/tell-me.tar.gz

# 方案5：gitee 镜像（备用）
curl -L "https://gitee.com/fuxm0818/tell-me/releases/download/latest/tell-me-x86_64-unknown-linux-gnu.tar.gz" -o /tmp/tell-me.tar.gz

# 下载成功后解压安装
mkdir -p ~/.local/bin
tar -xzf /tmp/tell-me.tar.gz -C ~/.local/bin/
chmod +x ~/.local/bin/tell-me
```

**Windows**:

```powershell
$tellMeDir = "$env:LOCALAPPDATA\tell-me"
New-Item -ItemType Directory -Force -Path $tellMeDir | Out-Null

# 按顺序尝试下载源
$urls = @(
    "https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-pc-windows-msvc.zip",
    "https://ghproxy.net/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-pc-windows-msvc.zip",
    "https://gh-proxy.com/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-pc-windows-msvc.zip",
    "https://mirror.ghproxy.com/https://github.com/fuxm0818/tell-me/releases/latest/download/tell-me-x86_64-pc-windows-msvc.zip",
    "https://gitee.com/fuxm0818/tell-me/releases/download/latest/tell-me-x86_64-pc-windows-msvc.zip"
)

$downloaded = $false
foreach ($url in $urls) {
    try {
        Invoke-WebRequest -Uri $url -OutFile "$tellMeDir\tell-me.zip" -TimeoutSec 30
        $downloaded = $true
        break
    } catch {
        Write-Output "下载失败: $url, 尝试下一个..."
    }
}

if ($downloaded) {
    Expand-Archive -Path "$tellMeDir\tell-me.zip" -DestinationPath $tellMeDir -Force
    Remove-Item "$tellMeDir\tell-me.zip" -Force
} else {
    Write-Output "所有下载源均失败，请手动下载"
}
```

**所有方案都失败时的兜底方案**：

- 告知用户手动下载地址：`https://github.com/fuxm0818/tell-me/releases`
- macOS / Linux：建议将 `tell-me` 可执行文件放到 `~/.local/bin/` 目录并 `chmod +x`
- Windows：建议将 `tell-me.exe` 放到 `%LOCALAPPDATA%\tell-me\` 目录，并将该目录加入系统 PATH
- 如果用户有 VPN 或其他网络工具，建议开启后重试

---

### 首次安装后的引导（重要！）

当 OpenClaw 帮助用户完成 tell-me 的安装后，**必须**主动向用户介绍功能并引导初始化：

**第一步：功能介绍**

向用户展示以下信息：
```
🎉 tell-me（告诉我）已安装完成！

这是一个纯本地离线文档问答工具，主要功能：

📖 文档问答 — 指定一个文件夹，构建知识库后即可秒级检索
💡 补充答案 — 检索结果不满意时，可以补充标准答案，下次自动优先返回
🗑️ 一键清空 — 随时重建知识库

使用方式：用「告诉我」或「tell-me」开头提问即可！
```

**第二步：主动引导初始化**

功能介绍后，立即询问用户：
```
请提供一个包含文档的文件夹路径，我来帮你构建本地知识库。

支持格式：TXT、MD、PDF、DOCX、DOC、XLSX、XLS、CSV、PPTX、PPT、RTF、WPS、ET、DPS
```

**第三步：执行初始化**

用户提供文件夹路径后，执行：
```bash
tell-me init /用户提供的路径
```

初始化完成后，按照「初始化完成后的行为」规则展示 3 个示例问题。

---

### 会话恢复（新会话时的行为）

**⚠️ 重要规则：新会话不得重复初始化！**

每次新会话开始时，按以下逻辑判断知识库是否已存在：

**macOS / Linux**:

```bash
ls ~/.local/bin/tell_me_data/ 2>/dev/null
```

**Windows**:

```powershell
Get-ChildItem "$env:LOCALAPPDATA\tell-me\tell_me_data" -ErrorAction SilentlyContinue
```

**判断逻辑**：

- **数据目录存在且有内容** → 直接告知用户："知识库已就绪，可以直接提问。" 然后等待用户提问，**不要要求用户重新输入文档路径**
- **数据目录不存在或为空** → 这时才询问用户文档目录路径，执行初始化

```bash
# 仅在确认没有知识库数据时才执行
tell-me init /用户指定的目录路径
```

#### 初始化完成后的行为（重要！）

`tell-me init` 执行成功后，**必须**基于刚构建的知识库内容，给用户提供 3 个使用示例：

1. 先查看 `tell-me init` 的输出，获取扫描到的文件列表和文档概要信息
2. 根据文档内容，生成 3 个有代表性的示例问题（问题应覆盖不同主题/模块）
3. 以友好格式展示给用户

**输出格式参考**：
```
✅ 知识库构建完成！共扫描了 N 个文档。

你可以试试这样提问：

1. 「告诉我 XXX 是什么？」
2. 「tell-me 如何配置 YYY？」  
3. 「告诉我 ZZZ 的流程是什么？」

直接用「告诉我」或「tell-me」开头提问即可！
```

**生成示例问题的原则**：
- 问题必须基于实际扫描到的文档内容，不能凭空编造
- 覆盖不同维度（概念类、操作类、配置类等）
- 问题要具体、有实际意义，避免太泛

**用户主动要求重新初始化的情况**：

- 用户明确说"重新初始化"、"换个目录"、"重建知识库"时，才执行 `tell-me clear` + `tell-me init`

---

### 问答

知识库就绪后，用户可直接提问：

```bash
tell-me ask "用户的问题"
```

#### 问答完成后的行为（重要！）

每次 `tell-me ask` 返回结果后，**必须**在回答末尾附加友好提示，告知用户可以补充完善答案：

**当检索结果不够完整或用户可能需要补充时**：
```
💡 觉得回答不够完整？你可以用「告诉我 补充答案：问题 → 答案」来添加标准答案，下次提问相同问题时会优先返回你的答案！
```

**当检索结果较为完整时**（简化提示）：
```
💡 你也可以补充自己的标准答案，让下次回答更精准。
```

**补充标准答案的方式**：
```bash
tell-me add-fqa "问题" "答案"
```

**提示规则**：
- 每次问答后都要附带提示，但同一会话中从第 3 次问答起可以省略提示（避免重复打扰）
- 如果用户已经使用过 `add-fqa`，后续可不再提示

### 补充标准答案

当检索结果不完整时，使用：

```bash
tell-me add-fqa "问题" "答案"
```

### 清空数据

```bash
tell-me clear
```

---

## 问题处理

**模型加载失败**:

- 确保程序有写入权限（首次运行需要提取模型）
- 模型已嵌入可执行文件，首次运行自动提取到数据目录的 `model/` 子目录

**PDF 解析失败**:

- 文件可能加密或格式不兼容，程序会自动跳过并记录错误

**大文件分块慢**:

- 超过 1MB 的文件使用快速分块模式，最多生成 1000 个块

**Windows 格式问题**:

- DOC、XLS、PPT、WPS、ET、DPS 格式已支持

**下载失败排查**:

- 依次尝试所有代理方案
- macOS / Linux 检查网络：`curl -I https://github.com`
- Windows 检查网络：`Test-NetConnection github.com -Port 443`
- 如果所有代理都不行，建议用户用浏览器手动下载

---

## 关键规则

- **触发条件**：用户的问题以「告诉我」、「tell-me」或「tell me」（支持一个或多个空格）开头时触发此技能，否则不得触发
- **版本检查**：每次会话首次使用时，检查本地版本是否为最新，落后则提示更新
- **不重复初始化**：已有知识库数据时，新会话直接进入问答模式，绝不要求用户重新输入目录
- **不重复下载**：已安装且为最新版就不重复下载
- **多源下载**：GitHub 直连失败时，自动依次尝试多个代理源，全部失败才提示手动下载
- **跨平台适配**：根据用户操作系统自动选择对应的命令和路径
- **初始化后给示例**：`tell-me init` 完成后，必须基于文档内容生成 3 个使用示例，帮助用户快速上手
- **问答后提示 FQA**：每次问答后附带友好提示，告知用户可以补充标准答案（同一会话第 3 次起可省略）
- **安装后主动引导**：首次安装完成后，主动介绍功能并引导用户提供文件夹路径进行初始化
- `tell-me ask` 只读缓存，速度快
- 数据存放位置：macOS/Linux 在 `~/.local/bin/tell_me_data/`，Windows 在 `%LOCALAPPDATA%\tell-me\tell_me_data\`
- 检索结果不完整时，必须主动询问用户是否补充到 FQA
