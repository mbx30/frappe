@echo off
REM Build script for Windows 10+ (x86_64).
REM
REM Tauri v2 on Windows uses Microsoft Edge WebView2 for the webview
REM layer, so the minimum supported Windows version is Windows 10
REM (build 17763+) with the WebView2 Runtime installed. Windows 7/8/8.1
REM and Windows XP are NOT supported — the resulting binary will not
REM start because WebView2 has no presence on those releases.
REM
REM Run this from the project root on a Windows machine with the
REM MSVC toolchain (Visual Studio 2015+ Build Tools) and WebView2
REM Runtime installed.

setlocal enabledelayedexpansion

echo.
echo ========================================
echo Building Frappe for Windows 10+ (x86_64)
echo ========================================
echo.

REM Check Rust installation
rustc --version >nul 2>&1
if errorlevel 1 (
    echo ERROR: Rust is not installed or not in PATH
    echo Visit: https://rustup.rs/
    exit /b 1
)

REM Add target
echo [1/5] Adding x86_64-pc-windows-msvc target...
rustup target add x86_64-pc-windows-msvc

REM Build frontend
echo [2/5] Building frontend (React + TypeScript)...
call npm ci
if errorlevel 1 exit /b 1
call npm run build
if errorlevel 1 exit /b 1

REM Build Rust backend
echo [3/5] Building Rust backend...
cd src-tauri
cargo build --release --lib --target x86_64-pc-windows-msvc
if errorlevel 1 (
    echo ERROR: Cargo build failed
    exit /b 1
)
cd ..

REM Build Tauri app
echo [4/5] Building Tauri app (MSI + NSIS installers)...
call npm run tauri build -- --target x86_64-pc-windows-msvc
if errorlevel 1 exit /b 1

REM Verify output
echo [5/5] Verifying output...
if exist "src-tauri\target\x86_64-pc-windows-msvc\release\frappe.exe" (
    echo.
    echo Build successful!
    echo.
    echo Output binary:
    dir /b "src-tauri\target\x86_64-pc-windows-msvc\release\frappe.exe"
    echo.
    echo Installers (if WiX / NSIS toolchains are installed):
    if exist "src-tauri\target\x86_64-pc-windows-msvc\release\bundle" (
        dir /s /b "src-tauri\target\x86_64-pc-windows-msvc\release\bundle"
    )
    echo.
    echo Minimum runtime: Windows 10 1809 (build 17763) with WebView2 Runtime
    echo Download WebView2: https://developer.microsoft.com/microsoft-edge/webview2/
) else (
    echo ERROR: frappe.exe not found in expected location
    exit /b 1
)

echo.
echo ========================================
echo Build complete for Windows 10+!
echo ========================================
echo.
pause
