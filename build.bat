@echo off
echo Building XemAnh in Debug mode...
where cargo >nul 2>nul
if %errorlevel% equ 0 (
    cargo build
) else (
    if exist "%USERPROFILE%\.cargo\bin\cargo.exe" (
        "%USERPROFILE%\.cargo\bin\cargo.exe" build
    ) else (
        echo [ERROR] Cargo was not found in your PATH or under %USERPROFILE%\.cargo\bin\cargo.exe.
        echo Please ensure Rust is installed correctly.
        pause
        exit /b 1
    )
)

if %errorlevel% equ 0 (
    echo [SUCCESS] Build debug version succeeded!
) else (
    echo [ERROR] Build failed.
)
pause
