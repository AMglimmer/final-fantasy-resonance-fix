@echo off
powershell.exe -NoProfile -ExecutionPolicy Bypass -File "%~dp0Manage-Patch.ps1" -Action Enable
pause
