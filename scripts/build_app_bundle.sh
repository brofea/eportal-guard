#!/bin/bash
# macOS App Bundle 构建脚本
# 用法: ./scripts/build_app_bundle.sh

set -e

PROJECT_ROOT=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
APP_NAME="ePortal Guard"
BUNDLE_DIR="$PROJECT_ROOT/dist/$APP_NAME.app"
EXECUTABLE_NAME="eportal_guard"
BUNDLE_ID="com.brofea.eportal-guard"
VERSION="1.0.0"

RESOURCES_DIR="$BUNDLE_DIR/Contents/Resources"
MACOS_DIR="$BUNDLE_DIR/Contents/MacOS"

echo "🏗️  构建 macOS App Bundle..."

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
    # 转换所有 WebP 为 PNG（如果需要）
    for png_file in "$APPICONSET"/*.png; do
        if file "$png_file" | grep -q "Web/P"; then
            echo "  • 转换 $(basename "$png_file") (WebP → PNG)..."
            sips -s format png "$png_file" -o "$png_file" 2>/dev/null || true
        fi
    done
    
    # 复制最大的图标作为应用主图标
    cp "$APPICONSET/1024-mac.png" "$RESOURCES_DIR/AppIcon.png"
    echo "  ✓ 应用图标已配置"
else
    echo "  ⚠️  警告: $APPICONSET 不存在，跳过图标处理"
fi

# 5. 复制其他资源文件（如果存在）
echo "5️⃣  复制其他资源文件..."
if [ -f "$PROJECT_ROOT/src/assets/globe.png" ]; then
    cp "$PROJECT_ROOT/src/assets/globe.png" "$RESOURCES_DIR/" 2>/dev/null || true
fi

# 6. 生成 Info.plist
echo "6️⃣  生成 Info.plist..."
cat > "$BUNDLE_DIR/Contents/Info.plist" << 'EOF'
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
    <key>CFBundleVersion</key>
    <string>1.0.0</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
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

# 7. 验证构建结果
echo "7️⃣  验证构建结果..."
if [ -f "$MACOS_DIR/$EXECUTABLE_NAME" ] && [ -f "$BUNDLE_DIR/Contents/Info.plist" ]; then
    echo "✅ App Bundle 构建成功！"
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
