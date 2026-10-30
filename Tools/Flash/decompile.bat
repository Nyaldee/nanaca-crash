@echo off
rem Exporte le code ActionScript 2 du SWF original avec FFDec (JPEXS Free
rem Flash Decompiler) dans Nanaca-Crash-Flash\decompiled\scripts.
rem Ce dossier est ignoré par git : il sert seulement de référence de
rem lecture pour le portage, jamais publié.
rem
rem Usage : Tools\Flash\decompile.bat <chemin de ffdec-cli.jar>
rem Java est pris dans JAVA_HOME, sinon dans le PATH.

setlocal
set "ROOT=%~dp0..\.."
set "FFDEC=%~1"
if "%FFDEC%"=="" (
    echo Usage: Tools\Flash\decompile.bat ^<path to ffdec-cli.jar^>
    exit /b 1
)
set "JAVA=java"
if defined JAVA_HOME set "JAVA=%JAVA_HOME%\bin\java.exe"

rem FFDec écrit lui-même dans un sous-dossier « scripts ».
set "OUT=%ROOT%\Nanaca-Crash-Flash\decompiled"
if exist "%OUT%\scripts" rmdir /s /q "%OUT%\scripts"
"%JAVA%" -jar "%FFDEC%" -export script "%OUT%" "%ROOT%\nanaca-crash_v110.swf"
