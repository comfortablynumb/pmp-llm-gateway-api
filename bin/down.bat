@echo off
setlocal

set PROFILE=%~1

:: Default to "full" profile if none specified
if "%PROFILE%"=="" set PROFILE=full

docker compose --profile %PROFILE% rm -f -s -v

endlocal
