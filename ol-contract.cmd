@echo off
setlocal
pushd "%~dp0"
cargo combat-contract %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%
