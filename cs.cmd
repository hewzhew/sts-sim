@echo off
setlocal
pushd "%~dp0"
set "combat_search_frontend=%~dp0target\debug\combat_search_v2.exe"
set "combat_search_worker=%~dp0target\release\combat_search_v2_worker.exe"
if not exist "%combat_search_frontend%" (
  echo Combat-search frontend is missing. Run: cargo build -p sts_combat_search_driver --bin combat_search_v2 1>&2
  popd
  exit /b 1
)
if "%~1"=="" goto frontend
if /i "%~1"=="--help" goto frontend
if /i "%~1"=="-h" goto frontend
if not exist "%combat_search_worker%" (
  echo Combat-search worker is missing. Run: cargo build --release -p sts_combat_search_driver --features backend --bin combat_search_v2_worker 1>&2
  popd
  exit /b 1
)
"%combat_search_worker%" %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%

:frontend
"%combat_search_frontend%" %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%
