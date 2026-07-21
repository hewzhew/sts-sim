@echo off
setlocal
"%~dp0target\fast-run\oracle_lab.exe" --canonical-fast-run %*
exit /b %ERRORLEVEL%
