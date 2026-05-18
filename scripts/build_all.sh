#!/bin/bash
# COI 全平台一键构建脚本
# 在 macOS 上一次性编译 Windows、Linux、macOS(ARM)、macOS(Intel) 四个平台的可执行文件
#
# 前置依赖：
#   1. Rust 工具链: rustup (https://rustup.rs)
#   2. cargo-xwin: cargo install cargo-xwin (用于交叉编译 Windows)
#   3. LLVM: brew install llvm (cargo-xwin 依赖，用于 Windows 编译)
#   4. cross: cargo install cross (用于交叉编译 Linux)
#   5. 容器引擎: Podman 或 Docker（cross 依赖，仅 Linux 编译需要）
#
# 使用方法：
#   chmod +x scripts/build_all.sh
#   ./scripts/build_all.sh

# 遇到错误不立即退出，改为逐步处理
set +e

# 颜色输出
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # 无颜色

# 输出目录
OUTPUT_DIR="dist"
# 构建结果统计
SUCCESS_COUNT=0
FAIL_COUNT=0
FAILED_TARGETS=""

echo "=========================================="
echo "  COI 全平台构建脚本"
echo "=========================================="
echo ""

# 检查前置依赖
check_dependency() {
    if ! command -v "$1" &> /dev/null; then
        echo -e "${RED}[错误] 未找到 $1，请先安装：$2${NC}"
        exit 1
    fi
}

check_dependency "cargo" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
check_dependency "rustup" "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"

# 添加编译目标
echo -e "${GREEN}[准备] 添加编译目标...${NC}"
rustup target add aarch64-apple-darwin 2>/dev/null || true
rustup target add x86_64-apple-darwin 2>/dev/null || true
rustup target add x86_64-pc-windows-msvc 2>/dev/null || true
rustup target add x86_64-unknown-linux-gnu 2>/dev/null || true

# 创建输出目录
rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR"

# ============================================================
# 1. macOS Universal Binary (ARM + Intel)
# ============================================================
echo ""
echo -e "${GREEN}[1/3] 编译 macOS (Universal Binary)...${NC}"

MACOS_OK=true

echo "  编译 ARM (M芯片)..."
if ! cargo build --release --target aarch64-apple-darwin 2>&1; then
    echo -e "${RED}  ❌ macOS ARM 编译失败${NC}"
    MACOS_OK=false
fi

echo "  编译 Intel..."
if ! cargo build --release --target x86_64-apple-darwin 2>&1; then
    echo -e "${RED}  ❌ macOS Intel 编译失败${NC}"
    MACOS_OK=false
fi

if [ "$MACOS_OK" = true ]; then
    lipo -create \
        target/aarch64-apple-darwin/release/coi \
        target/x86_64-apple-darwin/release/coi \
        -output "$OUTPUT_DIR/coi-macos"
    chmod +x "$OUTPUT_DIR/coi-macos"
    echo -e "  ${GREEN}✅ 完成: $OUTPUT_DIR/coi-macos${NC}"
    SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
else
    echo -e "  ${RED}❌ macOS 构建失败${NC}"
    FAIL_COUNT=$((FAIL_COUNT + 1))
    FAILED_TARGETS="$FAILED_TARGETS macOS"
fi

# ============================================================
# 2. Windows
# ============================================================
echo ""
echo -e "${GREEN}[2/3] 编译 Windows x64...${NC}"

# 检查 cargo-xwin
if ! cargo xwin --version &> /dev/null 2>&1; then
    echo -e "${YELLOW}  [提示] 未找到 cargo-xwin，正在安装...${NC}"
    cargo install cargo-xwin
fi

# 检查 llvm-lib（cargo-xwin 依赖 LLVM 工具链）
if ! command -v llvm-lib &> /dev/null 2>&1; then
    # 尝试从 brew 安装的 llvm 路径查找
    LLVM_PREFIX="$(brew --prefix llvm 2>/dev/null || true)"
    if [ -n "$LLVM_PREFIX" ] && [ -f "$LLVM_PREFIX/bin/llvm-lib" ]; then
        export PATH="$LLVM_PREFIX/bin:$PATH"
        echo "  使用 LLVM: $LLVM_PREFIX"
    else
        echo -e "  ${YELLOW}⚠️  未找到 llvm-lib，Windows 编译需要 LLVM${NC}"
        echo -e "  ${YELLOW}   请执行: brew install llvm${NC}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        FAILED_TARGETS="$FAILED_TARGETS Windows(需要LLVM)"
        # 跳过 Windows 编译，继续后续步骤
        SKIP_WINDOWS=true
    fi
fi

if [ "${SKIP_WINDOWS:-}" != "true" ]; then
    if cargo xwin build --release --target x86_64-pc-windows-msvc 2>&1; then
        cp target/x86_64-pc-windows-msvc/release/coi.exe "$OUTPUT_DIR/coi-windows.exe"
        echo -e "  ${GREEN}✅ 完成: $OUTPUT_DIR/coi-windows.exe${NC}"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    else
        echo -e "  ${RED}❌ Windows 构建失败${NC}"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        FAILED_TARGETS="$FAILED_TARGETS Windows"
    fi
fi

# ============================================================
# 3. Linux
# ============================================================
echo ""
echo -e "${GREEN}[3/3] 编译 Linux x64...${NC}"
echo -e "  ${YELLOW}⚠️  Linux 版本需要通过 GitHub Actions 构建（ONNX Runtime 交叉编译限制）${NC}"
echo -e "  ${YELLOW}   推送代码并打 tag 后，CI 会自动在 Linux 环境中编译${NC}"
echo -e "  ${YELLOW}   或者在 Linux 机器上直接执行: cargo build --release${NC}"
FAIL_COUNT=$((FAIL_COUNT + 1))
FAILED_TARGETS="$FAILED_TARGETS Linux(需要CI或Linux机器)"

# ============================================================
# 汇总
# ============================================================
echo ""
echo "=========================================="
echo "  构建完成！"
echo "=========================================="
echo ""

if [ -d "$OUTPUT_DIR" ] && [ "$(ls -A $OUTPUT_DIR 2>/dev/null)" ]; then
    echo "产物目录 $OUTPUT_DIR/："
    ls -lh "$OUTPUT_DIR/"
    echo ""
fi

echo "结果: 成功 $SUCCESS_COUNT 个, 失败 $FAIL_COUNT 个"

if [ $FAIL_COUNT -gt 0 ]; then
    echo -e "${YELLOW}未完成的平台:${FAILED_TARGETS}${NC}"
fi

echo ""
echo "分发说明："
echo "  • macOS 用户（Intel/M芯片通用） → coi-macos"
echo "  • Windows 用户                  → coi-windows.exe"
echo "  • Linux 用户                    → coi-linux"
echo ""
echo "注意：分发时需要同时提供 model/ 目录（或让用户首次运行时联网下载）"
