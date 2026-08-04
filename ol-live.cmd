@echo off
setlocal
pushd "%~dp0"
set "oracle_client=%~dp0target\release\oracle_lab_client.exe"
if not exist "%oracle_client%" (
  echo Canonical resident oracle client is missing. Run: cargo build --release -p oracle_lab_client --bin oracle_lab_client 1>&2
  popd
  exit /b 1
)
"%oracle_client%" --canonical-oracle %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%
