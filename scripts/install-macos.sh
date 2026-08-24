#!/bin/bash
# Familiar macOS 一键安装脚本：构建 release → 安装到 /Applications → 启动。
# 供开发者与用户在本机安装本地构建使用（仅支持 macOS）。
#
# 默认模式：先拉取 origin main（要求工作区干净）再安装，适合“更新到最新”。
# ./install-macos.sh --local：跳过拉取与干净工作区检查，直接用当前工作区
# 代码构建安装，用于改动完成后的一键安装测试。
set -euo pipefail

LOCAL_MODE=false
if [ "${1:-}" = "--local" ]; then
    LOCAL_MODE=true
fi

if [ "$(uname)" != "Darwin" ]; then
    echo "install-macos.sh 仅支持 macOS；其他平台请用 cargo/tauri 自行构建" >&2
    exit 1
fi

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_SRC="$REPO_DIR/target/release/bundle/macos/Familiar.app"
APP_DST="/Applications/Familiar.app"

log() { printf '\n\033[1;32m[%s]\033[0m %s\n' "$(date +%H:%M:%S)" "$*"; }
die() { printf '\n\033[1;31m[失败]\033[0m %s\n' "$*" >&2; exit 1; }

# 1. 同步代码（--local 模式跳过拉取，直接装当前工作区代码）
NEW_HEAD="$(git -C "$REPO_DIR" rev-parse --short HEAD)"
if [ "$LOCAL_MODE" = true ]; then
    log "1/4 使用本地工作区代码（--local）"
    cd "$REPO_DIR"
    if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
        echo "注意：工作区含未提交改动，本次安装包含这些改动"
    fi
else
    log "1/4 拉取最新代码"
    cd "$REPO_DIR"
    if [ -n "$(git status --porcelain --untracked-files=no)" ]; then
        die "已跟踪文件有未提交改动，请先处理后重试（git status 查看）"
    fi
    OLD_HEAD="$NEW_HEAD"
    git pull --ff-only origin main
    NEW_HEAD="$(git rev-parse --short HEAD)"
    if [ "$OLD_HEAD" = "$NEW_HEAD" ]; then
        echo "代码已是最新（${NEW_HEAD}）"
    fi
fi

# 2. 构建
log "2/4 构建 app（首次或依赖变更时耗时较长）"
cd "$REPO_DIR/app"
npm run tauri build -- --bundles app

# 3. 校验产物
log "3/4 校验产物"
[ -d "$APP_SRC" ] || die "未找到构建产物 $APP_SRC"
VERSION="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_SRC/Contents/Info.plist")"
echo "构建版本: $VERSION"

# 4. 安装并启动
log "4/4 安装到 /Applications 并启动"
osascript -e 'tell application "Familiar" to quit' 2>/dev/null || true
sleep 2
pkill -f "Familiar.app/Contents/MacOS/familiar-app" 2>/dev/null || true
rm -rf "$APP_DST"
ditto "$APP_SRC" "$APP_DST"
xattr -dr com.apple.quarantine "$APP_DST" 2>/dev/null || true
INSTALLED="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP_DST/Contents/Info.plist")"
[ "$INSTALLED" = "$VERSION" ] || die "安装版本 $INSTALLED 与构建版本 $VERSION 不一致"
open "$APP_DST"

log "完成：已安装并启动 Familiar v${INSTALLED}（${NEW_HEAD}）"
