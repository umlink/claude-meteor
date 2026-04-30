# 构建发布版本说明

## Tauri Build 产物位置

构建后的应用会在：
```
src-tauri/target/release/bundle/
```

对于 macOS，会在：
```
src-tauri/target/release/bundle/macos/
src-tauri/target/release/bundle/dmg/
```

## 推荐流程

1. **构建应用**
```bash
pnpm tauri build
```

2. **（可选）复制到 releases/ 目录保留**
```bash
mkdir -p releases
cp -r src-tauri/target/release/bundle/dmg/*.dmg releases/ 2>/dev/null || true
```

3. **如果需要在 git 中保留发布版本**
编辑 `.gitignore`，注释掉或删除：
```
# *.dmg
# *.app
```
并取消注释：
```
releases/
```
