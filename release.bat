@echo off
setlocal EnableDelayedExpansion
cd /d "%~dp0"

REM UTF-8 so Vietnamese release notes survive agent stdout / console I/O
chcp 65001 >nul
set PYTHONUTF8=1

echo =========================================
echo       XemAnh Automatic Release Script
echo =========================================
echo.

where powershell >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] PowerShell is required to run release automation.
    exit /b 1
)

if /i "%~1"=="rebuild" (
    REM rebuild [version] — build + upload/clobber assets for an existing tag
    REM version optional: defaults to current Cargo.toml version
    if "%~2"=="" (
        powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\release.ps1" -BumpType rebuild
    ) else (
        powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\release.ps1" -RebuildVersion "%~2"
    )
) else (
    set "BUMP_TYPE=%~1"
    if "!BUMP_TYPE!"=="" set "BUMP_TYPE=patch"
    powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\release.ps1" -BumpType !BUMP_TYPE! %2 %3 %4 %5
)

if %errorlevel% neq 0 (
    echo [ERROR] Release process failed!
    exit /b 1
)

echo [SUCCESS] Release process finished!
