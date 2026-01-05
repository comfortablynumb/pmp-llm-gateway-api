@echo off
REM Run integration tests using Docker Compose

setlocal enabledelayedexpansion

cd /d "%~dp0.."

echo Building and starting test services...
docker compose --profile test up --build -d mock-openai mock-anthropic mock-azure-openai app

echo Waiting for services to be healthy...
timeout /t 10 /nobreak > nul

echo Running integration tests...
docker compose --profile test run --rm hurl
set EXIT_CODE=%ERRORLEVEL%

echo Stopping test services...
docker compose --profile test down

exit /b %EXIT_CODE%
