@echo off
chcp 65001 > nul
title DARK-CLI
cd /d "%~dp0"

if exist "%~dp0dark-cli.exe" (
    "%~dp0dark-cli.exe" %*
) else if exist "%~dp0target\release\dark-cli.exe" (
    "%~dp0target\release\dark-cli.exe" %*
) else if exist "%~dp0target\debug\dark-cli.exe" (
    "%~dp0target\debug\dark-cli.exe" %*
) else (
    cargo run -- %*
)

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Ошибка при выполнении программы (код: %ERRORLEVEL%).
    pause
)
