#!/bin/bash
# macOS App Bundle 构建脚本
# 用法: ./scripts/build_app_bundle.sh

set -e

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
APP_NAME="ePortal Guard"
BUNDLE_DIR="$PROJECT_ROOT/dist/$APP_NAME.app"
EXECUTABLE_NAME="eportal_guard"
DEVELOPER_NAME="Brofea"

RESOURCES_DIR="$BUNDLE_DIR/Contents/Resources"
MACOS_DIR="$BUNDLE_DIR/Contents/MacOS"

VERSION=$(awk '
    /^\[package\]/ { in_pkg=1; next }
    /^\[/ { in_pkg=0 }
    in_pkg && $1 == "version" {
        gsub(/"/, "", $3)
        print $3
        exit
    }
' "$PROJECT_ROOT/Cargo.toml")

if [ -z "$VERSION" ]; then
    echo "❌ 无法从 Cargo.toml 读取 package.version"
    exit 1
fi

echo "🏗️  构建 macOS App Bundle..."
echo "🏷️  版本号: $VERSION"

# 1. 编译 Rust 二进制文件
echo "1️⃣  编译 Rust 发布版本..."
cd "$PROJECT_ROOT"
cargo build --release 2>&1 | grep -E "(Compiling|Finished|error)" || true

# 2. 创建 App Bundle 目录结构
echo "2️⃣  创建 App Bundle 目录结构..."
mkdir -p "$MACOS_DIR"
mkdir -p "$RESOURCES_DIR"

# 3. 复制二进制文件
echo "3️⃣  复制可执行文件..."
cp "$PROJECT_ROOT/target/release/$EXECUTABLE_NAME" "$MACOS_DIR/$EXECUTABLE_NAME"
chmod +x "$MACOS_DIR/$EXECUTABLE_NAME"

# 4. 转换并复制应用图标
echo "4️⃣  处理应用图标..."
APPICONSET="$PROJECT_ROOT/src/Assets.xcassets/AppIcon.appiconset"

if [ -d "$APPICONSET" ]; then
    
    # 复制最大的图标作为应用主图标
    cp "$APPICONSET/1024-mac.png" "$RESOURCES_DIR/AppIcon.png"
    echo "  ✓ 应用图标已配置"
else
    echo "  ⚠️  警告: $APPICONSET 不存在，跳过图标处理"
fi

# 5. 生成 Info.plist
echo "5️⃣  生成 Info.plist..."
cat > "$BUNDLE_DIR/Contents/Info.plist" << EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>ePortal Guard</string>
    <key>CFBundleDisplayName</key>
    <string>ePortal Guard</string>
    <key>CFBundleIdentifier</key>
    <string>com.brofea.eportal-guard</string>
    <key>CFBundleGetInfoString</key>
    <string>ePortal Guard by ${DEVELOPER_NAME}</string>
    <key>CFBundleVersion</key>
    <string>$VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026 ${DEVELOPER_NAME}</string>
    <key>CFBundleExecutable</key>
    <string>eportal_guard</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>LSMinimumSystemVersion</key>
    <string>11.0</string>
    <key>LSUIElement</key>
    <true/>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
EOF

# 6. 验证构建结果
echo "6️⃣  验证构建结果..."
if [ -f "$MACOS_DIR/$EXECUTABLE_NAME" ] && [ -f "$BUNDLE_DIR/Contents/Info.plist" ]; then
    echo "✅ App Bundle 构建成功！"
    echo "🏷️  应用版本: $VERSION"
    echo ""
    echo "📦 应用包位置: $BUNDLE_DIR"
    echo "📊 应用包大小: $(du -sh "$BUNDLE_DIR" | cut -f1)"
    echo ""
    echo "🚀 启动应用:"
    echo "   open \"$BUNDLE_DIR\""
else
    echo "❌ App Bundle 构建失败！"
    exit 1
fi
