@echo off
setlocal
pushd "%~dp0"
cargo ol-live %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%
