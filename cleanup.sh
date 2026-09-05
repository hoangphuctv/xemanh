#!/usr/bin/env bash
set -e

echo "=== 1. Xoá các file rác / file 0 byte dư thừa ==="
if [ -d "a" ]; then
    echo "Removing temporary directory 'a/'..."
    rm -rf a
fi

rm -f bump-version.ps1
rm -f scripts/bump-version.ps1 scripts/bump_version.ps1
rm -f output.txt

echo "=== 2. Dọn build cache & artifact tạm thời ==="
if command -v cargo &> /dev/null; then
    echo "Running 'cargo clean'..."
    cargo clean
else
    echo "Cargo not found, removing target/ manually..."
    rm -rf target/
fi

rm -rf .bridge-output/

echo "=== 3. Cập nhật .gitignore ==="
grep -qxF ".bridge-output/" .gitignore || echo ".bridge-output/" >> .gitignore
grep -qxF "output.txt" .gitignore || echo "output.txt" >> .gitignore
grep -qxF "*.log" .gitignore || echo "*.log" >> .gitignore

echo "=== 4. Kiểm tra và stage lại git ==="
git rm -r --cached --ignore-unmatch a/ output.txt bump-version.ps1 scripts/bump-version.ps1 scripts/bump_version.ps1 2>/dev/null || true
git add .gitignore

echo "Dọn dẹp hoàn tất! Kiểm tra lại với 'git status'."
