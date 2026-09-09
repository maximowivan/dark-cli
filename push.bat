@echo off
cd /d "%~dp0"

echo ==========================================================
echo   Syncing Dark-CLI with GitHub (maximowivan/dark-cli)
echo ==========================================================
echo.

git push -u origin main

echo.
if errorlevel 1 (
    echo [ERROR] Push failed. Please check your GitHub access rights.
) else (
    echo [SUCCESS] Repository successfully synchronized with GitHub!
)
echo.
pause
