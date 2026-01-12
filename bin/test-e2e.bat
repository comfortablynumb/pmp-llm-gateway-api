@echo off
REM Run Playwright E2E tests against the running application

setlocal enabledelayedexpansion

set "SCRIPT_DIR=%~dp0"
set "PROJECT_ROOT=%SCRIPT_DIR%.."
set "E2E_DIR=%PROJECT_ROOT%\resources\e2e"

REM Default values
if not defined BASE_URL set "BASE_URL=http://localhost:8080"
set "HEADED=false"

REM Parse arguments
:parse_args
if "%~1"=="" goto :run_tests
if "%~1"=="--headed" (
    set "HEADED=true"
    shift
    goto :parse_args
)
if "%~1"=="--url" (
    set "BASE_URL=%~2"
    shift
    shift
    goto :parse_args
)
echo Unknown option: %1
echo Usage: %0 [--headed] [--url base_url]
exit /b 1

:run_tests
cd /d "%E2E_DIR%"

REM Install dependencies if node_modules doesn't exist
if not exist "node_modules" (
    echo Installing dependencies...
    call npm install
    call npx playwright install chromium
)

REM Run tests
echo Running E2E tests against %BASE_URL%...

if "%HEADED%"=="true" (
    call npm run test:headed
) else (
    call npm test
)

endlocal
