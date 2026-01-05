@echo off
setlocal

set PROFILE=%1

if "%PROFILE%"=="" (
    docker compose rm -f -s -v
) else (
    docker compose --profile %PROFILE% rm -f -s -v
)

endlocal
