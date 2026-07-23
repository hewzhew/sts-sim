@echo off
setlocal
"%~dp0target\release\oracle_lab.exe" --canonical-oracle %*
exit /b %ERRORLEVEL%
