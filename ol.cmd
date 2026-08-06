@echo off
setlocal
pushd "%~dp0"
if "%~1"=="" goto compact_help
if /i "%~1"=="--help" goto compact_help
if /i "%~1"=="-h" goto compact_help
if /i "%~1"=="help" if "%~2"=="" goto full_help
set "oracle_binary=%~dp0target\release\oracle_lab.exe"
if not exist "%oracle_binary%" (
  echo Canonical oracle_lab is missing. Run: cargo oracle-lab contract --help 1>&2
  popd
  exit /b 1
)
"%oracle_binary%" --canonical-oracle %*
set "status=%ERRORLEVEL%"
popd
exit /b %status%

:compact_help
echo Usage: ol.cmd ^<command^> ...
echo.
echo Routine V2 command groups:
echo   ol.cmd contract --help
echo   ol.cmd artifact --help
echo   ol.cmd case --help
echo.
echo Run ol.cmd ^<command^> --help for current typed options.
echo Run ol.cmd help for the full canonical command surface.
popd
exit /b 0

:full_help
set "oracle_binary=%~dp0target\release\oracle_lab.exe"
if not exist "%oracle_binary%" (
  echo Canonical oracle_lab is missing. Run: cargo oracle-lab contract --help 1>&2
  popd
  exit /b 1
)
"%oracle_binary%" --canonical-oracle --help
set "status=%ERRORLEVEL%"
popd
exit /b %status%
