@echo off
setlocal
pushd "%~dp0"
cargo ol %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%
