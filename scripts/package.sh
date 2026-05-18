#!/bin/bash
# COI 离线分发包打包脚本
# 将可执行文件和模型文件打包为一个 zip，解压即用，完全无需联网
#
# 使用方法：
#   chmod +x scripts/package.sh
#   ./scripts/package.sh
#
# 前提：已执行过 ./scripts/build_all.sh 且 dist/ 目录下有编译产物
#       已执行过一次 coi init（让模型自动下载到 model/ 目录）

set -e

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

DIST_DIR="dist"
PACKAGE_DIR="packages"

echo "=========================================="
echo "  COI 离线分发包打包"
echo "=========================================="
echo ""

# 检查 dist 目录
if [ ! -d "$DIST_DIR" ]; then
    echo -e "${RED}[错误] dist/ 目录不存在，请先执行 ./scripts/build_all.sh${NC}"
    exit 1
fi

# 查找模型文件
MODEL_CACHE_DIR=""
EXE_DIR="$(cd "$(dirname "$0")/.." && pwd)"

# 检查 dist/model/ 目录（fastembed 默认下载位置）
if [ -d "$EXE_DIR/dist/model" ]; then
    MODEL_CACHE_DIR="$EXE_DIR/dist/model"
# 检查程序同级 model/ 目录
elif [ -d "$EXE_DIR/model" ]; then
    MODEL_CACHE_DIR="$EXE_DIR/model"
fi

if [ -z "$MODEL_CACHE_DIR" ] || [ ! -d "$MODEL_CACHE_DIR" ]; then
    echo -e "${YELLOW}[提示] 未找到已下载的模型文件${NC}"
    echo "  需要先运行一次 coi 让模型自动下载："
    echo "  mkdir /tmp/coi_test && echo 'test' > /tmp/coi_test/test.txt"
    echo "  ./dist/coi-macos init /tmp/coi_test"
    echo ""
    echo "  下载完成后重新执行本脚本"
    exit 1
fi

echo "模型目录: $MODEL_CACHE_DIR"
echo ""

# 创建打包输出目录
rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR"

# 打包函数
package_platform() {
    local binary_name="$1"
    local package_name="$2"
    local binary_path="$DIST_DIR/$binary_name"

    if [ ! -f "$binary_path" ]; then
        echo -e "  ${YELLOW}⚠️  $binary_name 不存在，跳过${NC}"
        return
    fi

    echo -e "${GREEN}打包 $package_name ...${NC}"

    local tmp_dir="$PACKAGE_DIR/tmp_$package_name"
    mkdir -p "$tmp_dir/coi/model"

    # 复制可执行文件
    cp "$binary_path" "$tmp_dir/coi/"
    chmod +x "$tmp_dir/coi/$binary_name"

    # 复制模型文件
    cp -r "$MODEL_CACHE_DIR"/* "$tmp_dir/coi/model/" 2>/dev/null || \
    cp -r "$MODEL_CACHE_DIR" "$tmp_dir/coi/model/"

    # 创建使用说明
    cat > "$tmp_dir/coi/使用说明.txt" << 'EOF'
COI - 我问你答（本地离线文档问答工具）
======================================

【首次使用 - 必读】

macOS 用户请先双击运行「安装.command」文件，完成权限设置。
（只需运行一次，之后直接在终端使用 coi-macos 即可）

使用方法（在终端中执行）：

1. 初始化知识库：
   ./coi-macos init /path/to/your/documents

2. 提问：
   ./coi-macos ask "你的问题"

3. 补充标准答案：
   ./coi-macos add-fqa "问题" "标准答案"

4. 清空所有数据：
   ./coi-macos clear

注意：
- 所有数据保存在程序同级的 coi_data/ 文件夹中
- 完全离线运行，无需联网
EOF

    # 创建一键安装脚本（macOS 用户双击即可完成权限设置）
    if [[ "$binary_name" == *macos* ]]; then
        cat > "$tmp_dir/coi/安装.command" << 'SCRIPT'
#!/bin/bash
# COI 安装脚本 - 双击运行即可
# 自动完成 macOS 安全权限设置

cd "$(dirname "$0")"

echo "正在设置 COI 权限..."
echo ""

# 移除 quarantine 标记（消除"无法验证开发者"弹窗）
xattr -d com.apple.quarantine ./coi-macos 2>/dev/null
xattr -cr . 2>/dev/null

# 添加执行权限
chmod +x ./coi-macos

echo "✅ 安装完成！"
echo ""
echo "现在可以在终端中使用 COI 了："
echo "  cd $(pwd)"
echo "  ./coi-macos init /path/to/your/documents"
echo "  ./coi-macos ask \"你的问题\""
echo ""
echo "按任意键关闭此窗口..."
read -n 1
SCRIPT
        chmod +x "$tmp_dir/coi/安装.command"
    fi

    # 打包为 zip
    (cd "$tmp_dir" && zip -r "../../$PACKAGE_DIR/$package_name.zip" coi/)
    rm -rf "$tmp_dir"

    local size=$(du -h "$PACKAGE_DIR/$package_name.zip" | cut -f1)
    echo -e "  ${GREEN}✅ $PACKAGE_DIR/$package_name.zip ($size)${NC}"
}

# 打包各平台
package_platform "coi-macos" "coi-macos-offline"
package_platform "coi-windows.exe" "coi-windows-offline"
package_platform "coi-linux" "coi-linux-offline"

echo ""
echo "=========================================="
echo "  打包完成！"
echo "=========================================="
echo ""
ls -lh "$PACKAGE_DIR/"*.zip 2>/dev/null
echo ""
echo "分发方式：将对应平台的 zip 文件复制到目标电脑，解压即用"
echo "无需联网，无需安装任何依赖"
