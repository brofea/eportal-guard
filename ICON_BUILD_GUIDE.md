# 应用图标和 App Bundle 构建指南

## 快速开始

### 方式一：使用自动化脚本（推荐）

```bash
./scripts/build_app_bundle.sh
```

该脚本会自动：
1. 编译 Rust 发布版本
2. 创建完整的 macOS App Bundle 目录结构
3. 自动转换任何 WebP 格式的图标为 PNG
4. 配置应用图标、Info.plist 和其他资源
5. 验证构建成功

### 方式二：手动构建（用于自定义需求）

```bash
# 1. 编译二进制
cargo build --release

# 2. 创建目录结构
mkdir -p "dist/ePortal Guard.app/Contents/MacOS"
mkdir -p "dist/ePortal Guard.app/Contents/Resources"

# 3. 复制binary
cp target/release/eportal_guard "dist/ePortal Guard.app/Contents/MacOS/eportal_guard"

# 4. 转换图标（如果是 WebP 格式）
sips -s format png src/Assets.xcassets/AppIcon.appiconset/*.png

# 5. 复制应用图标
cp src/Assets.xcassets/AppIcon.appiconset/1024-mac.png "dist/ePortal Guard.app/Contents/Resources/AppIcon.png"

# 6. 创建 Info.plist（见下文）
```

---

## 图标配置说明

### 目录结构

```
src/Assets.xcassets/AppIcon.appiconset/
├── Contents.json      # 图标集元数据
├── 16-mac.png         # 16×16 通知栏图标
├── 32-mac.png         # 32×32 菜单栏和 Dock
├── 64-mac.png         # 64×64（2x 放大版）
├── 128-mac.png        # 128×128 Spotlight 搜索
├── 256-mac.png        # 256×256（2x 放大版）
├── 512-mac.png        # 512×512 AppStore
└── 1024-mac.png       # 1024×1024 主应用图标
```

### Contents.json 格式

```json
{
  "images": [
    {
      "size": "16x16",
      "idiom": "mac",
      "filename": "16-mac.png",
      "scale": "1x"
    },
    // ... 其他尺寸 ...
  ],
  "info": {
    "version": 1,
    "author": "xcode"
  }
}
```

### Info.plist 配置

在 `dist/ePortal Guard.app/Contents/Info.plist` 中添加：

```xml
<key>CFBundleIconFile</key>
<string>AppIcon</string>
```

macOS 会自动查找 `AppIcon.png` 或 `AppIcon.icns`。

---

## 更新应用图标

### 方案 1：替换现有 PNG 文件

1. 使用图形编辑工具（如 Sketch、Figma）设计新图标
2. 导出为 PNG 格式，分别为各尺寸：16, 32, 64, 128, 256, 512, 1024
3. 替换 `src/Assets.xcassets/AppIcon.appiconset/` 中的相应文件
4. 重新运行 `./scripts/build_app_bundle.sh`

### 方案 2：从单一高分辨图像生成各尺寸

```bash
# 假设有一个 1024×1024 的 PNG 文件
for size in 16 32 64 128 256 512 1024; do
    sips -z $size $size input.png --out src/Assets.xcassets/AppIcon.appiconset/${size}-mac.png
done
```

### 方案 3：使用 Xcode Asset Catalog 方式

如果使用 Xcode，可以直接在 Asset Catalog 中编辑：
1. 打开 Xcode
2. 导入 `src/Assets.xcassets`
3. 在 AppIcon 中拖拽各尺寸图像
4. 导出更新的 `Contents.json`

---

## 故障排除

### iconutil 转换失败

错误信息：`Invalid Iconset`

**原因**：
- PNG 文件实际上是 WebP 格式
- Contents.json 格式不正确
- 文件尺寸不匹配

**解决方案**：
- 使用本脚本中的 WebP→PNG 转换逻辑
- 或者直接使用 `AppIcon.png` 而不用 iconutil（推荐用于轻量级应用）

### 图标在 Finder 中不显示

**可能原因**：
- Info.plist 中缺少 `CFBundleIconFile` 配置
- 图标文件名称与配置不匹配
- App Bundle 缓存未刷新

**解决方案**：
```bash
# 清除 macOS 图标缓存
sudo find /var/folders -name "com.apple.bird.client" -exec rm -rf {} \; 2>/dev/null || true

# 重建 App Bundle
./scripts/build_app_bundle.sh

# 刷新 Finder 显示
touch "dist/ePortal Guard.app"
```

---

## 最佳实践

✅ **推荐做法**：
- 提供完整的多尺寸 PNG 图标集
- 使用自动化脚本保持 App Bundle 同步
- 定期测试 `open "dist/ePortal Guard.app"` 运行

❌ **避免做法**：
- 只提供单一尺寸的图标（各平台需求不同）
- 手动编辑 App Bundle 内的文件（会被脚本覆盖）
- 忘记转换 WebP 格式的图标

