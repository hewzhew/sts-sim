@echo off
setlocal
pushd "%~dp0"
if "%~1"=="" goto compact_help
if /i "%~1"=="--help" goto compact_help
if /i "%~1"=="help" if "%~2"=="" goto compact_help
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
echo Usage: ol.cmd ^<contract^|artifact^|case^> ...
echo.
echo   contract combat   Run one bounded exact-combat contract.
echo   artifact summary  Read one compact V2 result.
echo   artifact search   Inspect compact exact-search service accounting.
echo   artifact trace    Replay-inspect its selected witness.
echo   artifact compare  Replay-compare contract-aligned and local-HP candidates.
echo   artifact turn     Inspect one exact complete-turn surface on a candidate.
echo   artifact rerun    Re-run one stored V2 request.
echo   case import       Admit one exact CombatCase to the V2 catalog.
echo   case list         Query the explicit V2 case catalog.
echo.
echo Run ol.cmd ^<command^> --help for the typed command surface.
popd
exit /b 0
