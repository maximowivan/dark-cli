@echo off
chcp 65001 > nul
title DARK-CLI
cd /d "%~dp0"

cargo run

if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Ошибка при выполнении программы.
    pause
)
