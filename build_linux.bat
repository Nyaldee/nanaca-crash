@echo off
rem Compile les portages pour Linux avec WSL (Ubuntu) : build.sh y fait le
rem travail et laisse les archives dans target\ :
rem   target\Nanaca.Crash.Flash.Linux.tar.gz
rem   target\Nanaca.Crash.Android.Linux.tar.gz
rem   target\Nanaca.Crash.All.Linux.tar.gz
rem Il faut, dans Ubuntu, Rust (rustup) ; pkg-config et libasound2-dev sont
rem installés ici s'ils manquent.

setlocal
rem build.sh écrit en UTF-8 : la page de code d'origine est remise à la fin
for /f "tokens=2 delims=:." %%c in ('chcp') do set "CP=%%c"
chcp 65001 >nul
cd /d "%~dp0"

rem En root (-u root) : apt ne demande pas de mot de passe. dpkg --configure -a
rem termine une installation interrompue, sans quoi apt refuse d'installer.
wsl.exe -d Ubuntu -u root -- bash -c "export DEBIAN_FRONTEND=noninteractive; dpkg -s pkg-config libasound2-dev >/dev/null 2>&1 || (dpkg --configure -a && apt-get update && apt-get install -y pkg-config libasound2-dev)"
if errorlevel 1 (
    set "ERR=1"
    goto :end
)

wsl.exe -d Ubuntu -- bash ./build.sh
set "ERR=%errorlevel%"

:end
chcp %CP% >nul
rem En cas d'échec, la fenêtre reste ouverte sur le message d'erreur.
if not "%ERR%"=="0" pause
exit /b %ERR%
