@echo off
rem Compile les portages en release et prépare les archives à partager :
rem   target\Nanaca.Crash.Flash.Windows.zip (nanaca_crash_flash.exe)
rem   target\Nanaca.Crash.Android.Windows.zip (nanaca_crash_android.exe)
rem   target\Nanaca.Crash.All.Windows.zip (les deux exécutables)
rem Les archives ne contiennent que les exécutables : chaque joueur fournit
rem son propre fichier original (voir README.md).
rem Pour Linux : build_linux.bat (WSL) ou build.sh.

setlocal
cd /d "%~dp0"

cargo build --release -p nanaca-crash-flash -p nanaca-crash-android
if errorlevel 1 goto :fail

call :zip Nanaca.Crash.Flash nanaca_crash_flash
if errorlevel 1 goto :fail
call :zip Nanaca.Crash.Android nanaca_crash_android
if errorlevel 1 goto :fail

rem Les deux exécutables ensemble.
set "ZIP=target\Nanaca.Crash.All.Windows.zip"
if exist "%ZIP%" del "%ZIP%"
powershell -NoProfile -Command "Compress-Archive -Path 'target\release\nanaca_crash_flash.exe','target\release\nanaca_crash_android.exe' -DestinationPath '%ZIP%'"
if errorlevel 1 goto :fail
echo %ZIP% est disponible.
exit /b 0

rem En cas d'échec, la fenêtre reste ouverte sur le message d'erreur.
:fail
pause
exit /b 1

rem :zip nom-de-l'archive nom-de-l'exécutable
rem (messages sans accents : la page de code de la console reste celle
rem de l'utilisateur, chcp 65001 change la police de certaines consoles)
:zip
set "EXE=target\release\%~2.exe"
set "ZIP=target\%~1.Windows.zip"
if exist "%ZIP%" del "%ZIP%"
powershell -NoProfile -Command "Compress-Archive -Path '%EXE%' -DestinationPath '%ZIP%'"
if errorlevel 1 exit /b 1
echo %ZIP% est disponible.
exit /b 0
