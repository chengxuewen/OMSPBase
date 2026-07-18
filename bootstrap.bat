@echo off
REM bootstrap.bat — First-time setup for OMSPBase development (Windows)
REM Usage: bootstrap.bat
REM After initial setup, use: pixi.bat

set "SCRIPT_DIR=%~dp0"

echo ================================================
echo   OMSPBase Development Environment Bootstrap
echo ================================================
echo.

echo [1/2] Installing pixi and project dependencies...
call "%SCRIPT_DIR%scripts\pixi-init.bat"

echo.
echo [2/2] Activating pixi environment...
call "%SCRIPT_DIR%scripts\pixi-shell.bat"

echo.
echo ================================================
echo   OMSPBase environment ready!
echo ================================================
echo.
echo Next time, just run: pixi.bat
