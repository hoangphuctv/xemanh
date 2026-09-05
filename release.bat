@echo off
setlocal
cd /d "%~dp0"

echo =========================================
echo       XemAnh Automatic Release Script
echo =========================================
echo.

set BUMP_TYPE=%1
if "%BUMP_TYPE%"=="" set BUMP_TYPE=patch

where powershell >nul 2>&1
if %errorlevel% neq 0 (
    echo [ERROR] PowerShell is required to run release automation.
    exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -File "scripts\release.ps1" -BumpType %BUMP_TYPE% %2 %3 %4 %5
if %errorlevel% neq 0 (
    echo [ERROR] Release process failed!
    exit /b 1
)

echo [SUCCESS] Release process finished!
