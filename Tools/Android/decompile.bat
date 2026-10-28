@echo off
rem Décompile les classes C++ du jeu Android avec Ghidra, en C, dans
rem Nanaca-Crash-Android\decompiled\cpp (un fichier par classe), et sort de
rem l'APK les fichiers texte dans Nanaca-Crash-Android\decompiled\assets.
rem Ces dossiers sont ignorés par git : ils servent seulement de référence
rem de lecture pour le portage, jamais publiés.
rem
rem Usage : Tools\Android\decompile.bat <dossier de Ghidra>
rem Java est pris dans JAVA_HOME (ou celui configuré pour Ghidra).

setlocal
set "ROOT=%~dp0..\.."
set "GHIDRA=%~1"
if "%GHIDRA%"=="" (
    echo Usage: Tools\Android\decompile.bat ^<Ghidra folder^>
    exit /b 1
)

powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0extract.ps1"
if errorlevel 1 exit /b 1

set "WORK=%ROOT%\Tools\work\android"
set "OUT=%ROOT%\Nanaca-Crash-Android\decompiled\cpp"
if exist "%OUT%" rmdir /s /q "%OUT%"
mkdir "%OUT%"
rem Le lanceur de Ghidra coupe les chemins aux espaces : leurs noms courts.
for %%I in ("%WORK%") do set "WORK=%%~sI"
for %%I in ("%OUT%") do set "OUT=%%~sI"
for %%I in ("%~dp0.") do set "SCRIPTS=%%~sI"
rem Le projet Ghidra déjà analysé est repris s'il existe (bien plus rapide).
if exist "%WORK%\nanaca.rep" (
    call "%GHIDRA%\support\analyzeHeadless.bat" "%WORK%" nanaca -process libcocos2dlua.so -noanalysis -scriptPath "%SCRIPTS%" -postScript ExportGame.java "%OUT%"
) else (
    call "%GHIDRA%\support\analyzeHeadless.bat" "%WORK%" nanaca -import "%WORK%\libcocos2dlua.so" -scriptPath "%SCRIPTS%" -postScript ExportGame.java "%OUT%"
)
