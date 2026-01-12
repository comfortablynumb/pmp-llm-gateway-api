@echo off
setlocal

set SCRIPT_DIR=%~dp0
set PROFILE=%~1

:: Default to "full" profile if none specified
if "%PROFILE%"=="" set PROFILE=full

:: Run down first
call "%SCRIPT_DIR%down.bat" %PROFILE%

docker compose --profile %PROFILE% up

endlocal
