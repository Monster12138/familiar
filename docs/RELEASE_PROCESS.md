# Familiar 发布流程

本文档描述 Familiar 的版本升级、发布验证、macOS 与 Windows 打包、本地制品归档、Git 分支与 Tag，以及 GitHub Release 发布流程。

当前自动化尚未覆盖完整的发布链路。执行发布前应确认操作者明确授权了提交、推送、创建 Tag 和发布外部制品。

## 1. 发布目录约定

发布相关文件分为两类：

- `docs/releases/vX.Y.Z.md`：进入 Git 的 Release Notes 源文件。
- `release-artifacts/vX.Y.Z/`：本地发布制品归档，不进入 Git。

本地制品目录结构：

```text
release-artifacts/
└── vX.Y.Z/
    ├── Familiar_X.Y.Z_aarch64.dmg
    ├── Familiar_X.Y.Z_x64-setup.exe
    ├── familiar_X.Y.Z_amd64.deb
    └── SHA256SUMS
```

`release-artifacts/` 已在根目录 `.gitignore` 中忽略。不要使用 `git add -f` 提交其中的二进制或校验文件。Release Notes 只保存在 `docs/releases/`，本地制品目录不保留副本。

每个版本必须使用独立子目录。不要覆盖旧版本制品；重新构建同一版本时，应先核实该版本是否已发布。已经发布的版本原则上不能替换附件，应升级版本后重新发布。

## 2. 发布前检查

从仓库根目录开始：

```bash
git status --short
git branch --show-current
git log -5 --oneline --decorate
git remote -v
gh auth status
```

确认以下事项：

- 当前工作树中的改动都属于本次发布。
- 本地配置、编辑器状态、构建输出和其他无关文件不会被提交。
- 目标版本对应的分支、Tag 和 GitHub Release 尚不存在。
- GitHub CLI 已登录并具有仓库写入权限。
- 发布范围符合 Familiar 的本地优先和隐私约束。

检查远端 Tag 和 Release，例如：

```bash
VERSION=v0.2.0
git ls-remote --tags origin "refs/tags/$VERSION"
gh release view "$VERSION" --repo Monster12138/familiar
```

`gh release view` 返回 `release not found` 才表示同名 Release 尚未创建。

## 3. 创建发布分支

分支使用 `codex/` 前缀：

```bash
VERSION=v0.2.0
git switch -c "codex/release-$VERSION"
```

如果分支已经存在，不要盲目删除或覆盖。先检查它是否包含需要保留的提交。

## 4. 同步版本元数据

去掉 Git Tag 的 `v` 前缀后，将版本同步写入：

- 根目录 `Cargo.toml` 的 `[workspace.package].version`。
- `Cargo.lock` 中所有 Familiar workspace package。
- `app/package.json`。
- `app/package-lock.json` 的根版本和根 package 版本。

例如 Tag 为 `v0.2.0` 时，文件内版本应为 `0.2.0`。

使用 Cargo metadata 验证所有 Rust crate：

```bash
cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name | startswith("familiar")) | "\(.name): \(.version)"' \
  | sort
```

Tauri 默认从 `familiar-app` crate 继承 bundle 版本，因此无需在 `tauri.conf.json` 中重复声明版本。

## 5. 编写双语 Release Notes

在创建 Tag 之前新增：

```text
docs/releases/vX.Y.Z.md
```

Release Notes 使用中文在前、英文在后的结构：

```markdown
# Familiar vX.Y.Z

[中文](#中文) | [English](#english)

## 中文

中文发布说明……

---

## English

English release notes...
```

至少包含：

- 主要功能和修复。
- 兼容性或配置迁移说明。
- 隐私影响。
- 支持的平台和架构。
- Apple 签名与公证状态。
- 可复现的性能数据及其采样条件（如适用）。

DMG 构建完成后，将最终 SHA-256 写入 Release Notes。不要在没有实测的情况下填写校验和或性能数据。

## 6. 发布验证

在创建发布提交和 Tag 前运行：

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings

cd app
npm run build
cd ..

git diff --check
```

如果修改了 Rust 文件，只格式化任务触及的文件并检查：

```bash
rustfmt path/to/changed.rs
rustfmt --check path/to/changed.rs
```

任何检查失败都应先定位原因。不得把聚焦检查描述成完整 workspace 检查，也不得隐瞒预先存在的失败。

## 7. 构建安装包

### 7.1 macOS（Apple Silicon）

当前本地发布目标为 Apple Silicon macOS：

```bash
cd app
npm run tauri build -- --bundles app,dmg
cd ..
```

预期输出：

```text
target/release/bundle/macos/Familiar.app
target/release/bundle/dmg/Familiar_X.Y.Z_aarch64.dmg
```

验证 bundle 版本、架构和 DMG 完整性：

```bash
APP=target/release/bundle/macos/Familiar.app
DMG=target/release/bundle/dmg/Familiar_X.Y.Z_aarch64.dmg

hdiutil verify "$DMG"
/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist"
/usr/libexec/PlistBuddy -c 'Print :CFBundleVersion' "$APP/Contents/Info.plist"
file "$APP/Contents/MacOS/familiar-app"
file "$APP/Contents/Resources/bin/familiar-cli"
```

两个二进制都应报告 `Mach-O 64-bit executable arm64`，两个 bundle 版本都应与发布版本一致。

### 7.2 Windows（x86_64）

在 Windows 机器上（需要 MSVC 构建工具、Windows SDK 和 WebView2 Runtime）：

```powershell
cd app
npm run tauri build -- --bundles nsis
cd ..
```

预期输出：

```text
app/src-tauri/target/release/bundle/nsis/Familiar_X.Y.Z_x64-setup.exe
```

打包前确认 `target/release/familiar-cli.exe` 存在（`beforeBuildCommand` 会构建它），
且 `app/src-tauri/icons/icon.ico` 已生成。`familiar-cli.exe` 通过平台配置
`tauri.windows.conf.json` 打入安装包的 `resources/bin/`。

首次构建 NSIS 安装包时 Tauri 会自动下载 NSIS 工具链，需要网络访问。
Windows 安装包当前未做代码签名，Release Notes 必须说明 SmartScreen 可能
要求用户手动确认首次安装。

### 7.3 Linux（x86_64）

在 Linux 机器或 WSL2 Ubuntu 上（需要 GCC、pkg-config、`libwebkit2gtk-4.1-dev`、
`libgtk-3-dev`、`libayatana-appindicator3-dev`、`librsvg2-dev` 等开发包）：

```bash
cd app
npm run tauri build -- --bundles deb,appimage
cd ..
```

预期输出：

```text
app/src-tauri/target/release/bundle/deb/familiar_X.Y.Z_amd64.deb
app/src-tauri/target/release/bundle/appimage/familiar_X.Y.Z_amd64.AppImage
```

打包前确认 `target/release/familiar-cli` 存在（`beforeBuildCommand` 会构建它）。
`familiar-cli` 通过平台配置 `tauri.linux.conf.json` 打入安装包的
`resources/bin/`，运行时路径解析复用 `resolve_cli_bin_path` 的跨平台候选逻辑。

首次构建 AppImage 时 Tauri 会自动下载 linuxdeploy 工具链，需要网络访问；
生成 AppImage 的运行环境还需要 FUSE（缺少时可改用 deb，或用
`--appimage-extract-and-run` 方式启动验证）。deb 的运行期依赖
（`libwebkit2gtk-4.1-0` 等）由 Tauri 自动写入包元数据。
Linux 安装包当前未做签名，Release Notes 应说明安装来源未经分发渠道签名。

WSL 下开发调试启动 GUI 时可能需要 `WEBKIT_DISABLE_DMABUF_RENDERER=1
LIBGL_ALWAYS_SOFTWARE=1 GDK_BACKEND=x11` 规避 WSLg 的渲染问题；
打包构建本身不受影响。

## 8. 签名与公证检查

查看当前机器是否存在 Developer ID 签名身份：

```bash
security find-identity -v -p codesigning
```

验证应用签名：

```bash
codesign -dv --verbose=2 "$APP"
spctl -a -vv -t exec "$APP"
```

如果没有有效的 Apple Developer ID：

- 不得声称安装包已签名或已公证。
- Release Notes 必须明确说明 Gatekeeper 可能要求用户手动允许首次启动。
- 正式对外发布前应优先补齐 Developer ID 签名和 notarization。

## 9. 生成并归档制品

SHA-256 校验和由 CI（`release.yml` 的 `finalize-release` job）在构建完成后自动
计算，并以 `SHA256SUMS` 附件上传到 Release；Release Notes 不再包含校验和段落。
如需本地核验，可下载 `SHA256SUMS` 后用 `shasum -a 256 -c SHA256SUMS` 校验。

本地归档（可选）：创建当前版本的本地归档目录，并复制文件：

```bash
VERSION=v0.2.0
ARTIFACT_DIR="release-artifacts/$VERSION"

mkdir -p "$ARTIFACT_DIR"
cp "$DMG" "$ARTIFACT_DIR/"
# 如在 Windows 上构建了 NSIS 安装包，同样复制到 $ARTIFACT_DIR/
# 如在 Linux/WSL 上构建了 deb/AppImage，同样复制到 $ARTIFACT_DIR/

cd "$ARTIFACT_DIR"
shasum -a 256 *.dmg *.exe *.deb *.AppImage > SHA256SUMS 2>/dev/null || shasum -a 256 *.dmg > SHA256SUMS
shasum -a 256 -c SHA256SUMS
cd ../..
```

确认目录被 Git 忽略：

```bash
git check-ignore -v "$ARTIFACT_DIR/Familiar_${VERSION#v}_aarch64.dmg"
git status --short
```

## 10. 创建发布提交

只暂存本次发布涉及的源码、版本元数据和进入 Git 的 Release Notes：

```bash
git status --short
git add <本次发布文件>
git diff --cached --check
git diff --cached --stat
git commit -m "chore: prepare $VERSION release"
```

不要暂存 `release-artifacts/`、`target/` 或其他本地状态。提交完成后再次检查 `git status --short`。

## 11. 推送分支

常规 SSH 推送：

```bash
git push -u origin "codex/release-$VERSION"
```

如果当前网络关闭 GitHub SSH 22 端口，可使用 GitHub CLI 的 HTTPS 凭据，不需要打印或复制 token：

```bash
git -c credential.helper= \
  -c 'credential.helper=!gh auth git-credential' \
  push -u https://github.com/Monster12138/familiar.git \
  "codex/release-$VERSION:codex/release-$VERSION"
```

## 12. 创建并推送 Tag

Tag 必须在版本、Release Notes、验证结果和发布提交全部完成后创建：

```bash
git tag -a "$VERSION" -m "Familiar $VERSION"
git push origin "refs/tags/$VERSION"
```

HTTPS 方式：

```bash
git -c credential.helper= \
  -c 'credential.helper=!gh auth git-credential' \
  push https://github.com/Monster12138/familiar.git \
  "refs/tags/$VERSION"
```

已经推送的 Tag 不得移动、覆盖或强制推送。若发布内容需要修正，应升级 patch 版本并创建新 Tag。

## 13. 创建 GitHub Release

使用本地归档目录中的已验证文件（多平台发布时把 Windows 安装包一并附上）：

```bash
gh release create "$VERSION" \
  "$ARTIFACT_DIR/Familiar_${VERSION#v}_aarch64.dmg" \
  "$ARTIFACT_DIR/Familiar_${VERSION#v}_x64-setup.exe" \
  "$ARTIFACT_DIR/SHA256SUMS" \
  --repo Monster12138/familiar \
  --title "Familiar $VERSION" \
  --notes-file "docs/releases/$VERSION.md" \
  --verify-tag
```

Alpha、Beta 或 RC 版本增加 `--prerelease`。正式语义化版本不加该参数。

发布动作是外部状态变更。只有用户明确要求发布时才能执行，不得因为完成了本地构建就自动创建 Release。

## 14. 发布后复核

```bash
gh release view "$VERSION" \
  --repo Monster12138/familiar \
  --json url,name,tagName,isDraft,isPrerelease,publishedAt,targetCommitish,assets

git ls-remote origin \
  "refs/heads/codex/release-$VERSION" \
  "refs/tags/$VERSION" \
  "refs/tags/$VERSION^{}"
```

确认：

- Release 不是 Draft。
- 正式版与预发布状态正确。
- DMG、Windows 安装包（如有）和 `SHA256SUMS` 均为 `uploaded`。
- GitHub 返回的制品 digest 与本地 SHA-256 一致。
- Tag 解引用后指向预期发布提交。
- 本地 `release-artifacts/vX.Y.Z/` 中保留同一份制品。

发布总结应包含 Release URL、版本、平台架构、文件大小、SHA-256、测试结果，以及签名和公证限制。
