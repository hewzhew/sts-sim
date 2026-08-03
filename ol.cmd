@echo off
setlocal
pushd "%~dp0"
set "oracle_binary=%~dp0target\release\oracle_lab.exe"
if not exist "%oracle_binary%" (
  echo Canonical oracle_lab is missing. Run: cargo oracle-lab ^<command^> ... 1>&2
  popd
  exit /b 1
)
"%oracle_binary%" --canonical-oracle %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%
