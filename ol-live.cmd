@echo off
setlocal
"%~dp0target\release\oracle_lab_client.exe" --canonical-oracle %*
exit /b %ERRORLEVEL%
