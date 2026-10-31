#!/usr/bin/env bash
# ============================================================
# wayhug.com 数据备份脚本
# 备份内容：故事数据 wayhug.json、原画 sketches/、字体 fonts/
# 策略：每日全量打包，保留最近 30 份，超出自动清理
# 用法：./scripts/backup.sh [项目根目录]
# ============================================================
set -euo pipefail

ROOT="${1:-/opt/wayhug.com/rust}"
DATA_DIR="$ROOT/data"
BACKUP_DIR="$ROOT/backups"
KEEP=30

mkdir -p "$BACKUP_DIR"

TS="$(date +%Y%m%d-%H%M%S)"
ARCHIVE="$BACKUP_DIR/wayhug-data-$TS.tar.gz"

# 只打包存在的目录，避免 tar 报错
TAR_ARGS=()
[ -f "$DATA_DIR/wayhug.json" ] && TAR_ARGS+=("data/wayhug.json")
[ -d "$DATA_DIR/sketches" ] && TAR_ARGS+=("data/sketches")
[ -d "$DATA_DIR/fonts" ] && TAR_ARGS+=("data/fonts")

if [ ${#TAR_ARGS[@]} -eq 0 ]; then
  echo "[$(date '+%F %T')] 没有可备份的数据，跳过。"
  exit 0
fi

tar -czf "$ARCHIVE" -C "$ROOT" "${TAR_ARGS[@]}"
SIZE="$(du -h "$ARCHIVE" | cut -f1)"
echo "[$(date '+%F %T')] 备份完成: $ARCHIVE ($SIZE)"

# 清理超出保留份数的旧备份
cd "$BACKUP_DIR"
ls -1t wayhug-data-*.tar.gz 2>/dev/null | tail -n +$((KEEP + 1)) | while read -r old; do
  rm -f "$old"
  echo "[$(date '+%F %T')] 清理旧备份: $old"
done
