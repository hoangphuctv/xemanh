@echo off
setlocal
set "ISCC=%LocalAppData%\Programs\Inno Setup 6\ISCC.exe"

echo ============================================
echo   XemAnh - Build Release + Create Installer
echo ============================================
echo.

:: Step 1: Build release
echo [1/3] Building release...
where cargo >nul 2>nul
if %errorlevel% equ 0 (
    cargo build --release
) else (
    if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
        "%USERPROFILE%\.cargo\bin\cargo.exe" build --release
    ) else (
        echo [ERROR] Cargo not found.
        pause
        exit /b 1
    )
)

if %errorlevel% neq 0 (
    echo [ERROR] Build failed.
    pause
    exit /b 1
)
echo [OK] Release build succeeded.
echo.

:: Embed Windows icon into the exe (Start Menu / Explorer / taskbar)
echo [1b/3] Embedding application icon...
powershell -NoProfile -ExecutionPolicy Bypass -File "stamp-icon.ps1" -ExePath "target\release\xemanh.exe"
if %errorlevel% neq 0 (
    echo [ERROR] Failed to embed icon.
    pause
    exit /b 1
)
echo.

:: Step 2: Verify exe exists
if not exist "target\release\xemanh.exe" (
    echo [ERROR] target\release\xemanh.exe not found.
    pause
    exit /b 1
)

:: Step 3: Run Inno Setup
echo [2/3] Creating installer with Inno Setup...
if not exist "%ISCC%" (
    echo [ERROR] Inno Setup not found at: %ISCC%
    echo Please install Inno Setup 6 from https://jrsoftware.org/isinfo.php
    pause
    exit /b 1
)

"%ISCC%" "installer.iss"
if %errorlevel% neq 0 (
    echo [ERROR] Inno Setup failed.
    pause
    exit /b 1
)
echo [OK] Installer created.
echo.

:: Done
echo [3/3] Done!
echo Installer: installer\xemanh-0.1.0-setup.exe
echo.
REM pause
