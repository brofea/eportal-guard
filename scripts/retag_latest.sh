#!/usr/bin/env bash

set -euo pipefail

if [[ $# -lt 1 || "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  echo "用法: $0 <版本号> [提交SHA]"
  echo "示例: $0 0.1-alpha"
  echo "示例: $0 v0.1-alpha"
  echo "示例: $0 0.1-alpha HEAD"
  exit 1
fi

raw_version="$1"
target_ref="${2:-HEAD}"

if [[ "$raw_version" == v* ]]; then
  tag="$raw_version"
else
  tag="v$raw_version"
fi

if [[ ! "$tag" =~ ^v[0-9]+(\.[0-9A-Za-z]+)*([-+][0-9A-Za-z.-]+)?$ ]]; then
  echo "错误: 非法版本号 '$raw_version'"
  echo "示例: 0.1-alpha / 1.0.0 / v1.2.3-rc1"
  exit 1
fi

if ! git rev-parse --is-inside-work-tree >/dev/null 2>&1; then
  echo "错误: 当前目录不是 Git 仓库"
  exit 1
fi

if ! git rev-parse "$target_ref" >/dev/null 2>&1; then
  echo "错误: 无效的提交引用: $target_ref"
  exit 1
fi

commit_sha="$(git rev-parse --short "$target_ref")"
commit_msg="$(git log -1 --pretty=%s "$target_ref")"

echo "准备重绑标签: $tag"
echo "目标提交: $commit_sha"
echo "提交说明: $commit_msg"

if git show-ref --tags --verify --quiet "refs/tags/$tag"; then
  git tag -d "$tag"
fi

git tag -a "$tag" "$target_ref" -m "retag $tag -> $commit_sha"

git push origin ":refs/tags/$tag" || true
git push origin "$tag"

echo "完成: $tag 已指向 $(git rev-parse --short "$tag")"
