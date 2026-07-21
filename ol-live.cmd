@echo off
setlocal
"%~dp0target\fast-run\oracle_lab_client.exe" --canonical-fast-run %*
exit /b %ERRORLEVEL%
