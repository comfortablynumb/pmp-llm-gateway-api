@echo off
REM Load test runner script
REM Usage: bin\load-test.bat [health|chat|all] [base_url] [api_key]

setlocal

set TEST=%1
set BASE_URL=%2
set API_KEY=%3

if "%TEST%"=="" set TEST=health
if "%BASE_URL%"=="" set BASE_URL=http://localhost:8080
if "%API_KEY%"=="" set API_KEY=test-api-key

echo.
echo === LLM Gateway Load Tests ===
echo.
echo Test: %TEST%
echo Base URL: %BASE_URL%
echo.

if "%TEST%"=="health" goto health
if "%TEST%"=="chat" goto chat
if "%TEST%"=="all" goto all
goto help

:health
echo Running health endpoint load tests...
docker run --rm -i --network=host ^
  -e BASE_URL=%BASE_URL% ^
  grafana/k6 run - < tests\load\health.js
goto end

:chat
echo Running chat completions load tests...
docker run --rm -i --network=host ^
  -e BASE_URL=%BASE_URL% ^
  -e API_KEY=%API_KEY% ^
  grafana/k6 run - < tests\load\chat.js
goto end

:all
echo Running all load tests...
echo.
echo --- Health Tests ---
docker run --rm -i --network=host ^
  -e BASE_URL=%BASE_URL% ^
  grafana/k6 run - < tests\load\health.js
echo.
echo --- Chat Tests ---
docker run --rm -i --network=host ^
  -e BASE_URL=%BASE_URL% ^
  -e API_KEY=%API_KEY% ^
  grafana/k6 run - < tests\load\chat.js
goto end

:help
echo Usage: bin\load-test.bat [health^|chat^|all] [base_url] [api_key]
echo.
echo Examples:
echo   bin\load-test.bat health
echo   bin\load-test.bat chat http://localhost:8080 my-api-key
echo   bin\load-test.bat all http://localhost:3000
goto end

:end
endlocal
