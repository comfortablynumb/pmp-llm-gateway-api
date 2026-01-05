@echo off
setlocal

set SCRIPT_DIR=%~dp0
set PROFILE=%1

:: Run down first
call "%SCRIPT_DIR%down.bat" %PROFILE%

if "%PROFILE%"=="" (
    docker compose up -d
) else (
    docker compose --profile %PROFILE% up -d
)

endlocal
