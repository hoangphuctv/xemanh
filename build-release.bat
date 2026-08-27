@echo off
echo Building XemAnh in Release mode...
where cargo >nul 2>nul
if %errorlevel% equ 0 (
    cargo build --release
) else (
    if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
        "%USERPROFILE%\.cargo\bin\cargo.exe" build --release
    ) else (
        echo [ERROR] Cargo was not found in your PATH or under %USERPROFILE%\.cargo\bin\cargo.exe.
        echo Please ensure Rust is installed correctly.
        pause
        exit /b 1
    )
)

if %errorlevel% equ 0 (
    echo [SUCCESS] Build release version succeeded!
    powershell -NoProfile -ExecutionPolicy Bypass -File "stamp-icon.ps1" -ExePath "target\release\xemanh.exe"
) else (
    echo [ERROR] Build failed.
)
pause
