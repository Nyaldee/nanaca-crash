# Sort de l'APK original (à la racine du dépôt) ce qu'il faut lire pour le
# portage : la bibliothèque C++ du jeu (pour Ghidra) et les fichiers texte
# (scripts Lua, XML, animations, feuilles d'images, polices). Tout va dans
# des dossiers ignorés par git ; rien n'est exécuté.
#
# Usage : powershell -File Tools\Android\extract.ps1

$root = Resolve-Path "$PSScriptRoot\..\.."
$apk = Get-ChildItem -LiteralPath $root -Filter *.apk | Select-Object -First 1
if (-not $apk) { Write-Error "Aucun APK à la racine du dépôt."; exit 1 }

$work = Join-Path $root "Tools\work\android"
$assets = Join-Path $root "Nanaca-Crash-Android\decompiled\assets"
New-Item -ItemType Directory -Force $work, $assets | Out-Null

Add-Type -AssemblyName System.IO.Compression.FileSystem
$zip = [System.IO.Compression.ZipFile]::OpenRead($apk.FullName)
try {
    foreach ($e in $zip.Entries) {
        $target = $null
        if ($e.FullName -eq "lib/armeabi-v7a/libcocos2dlua.so") {
            $target = Join-Path $work "libcocos2dlua.so"
        } elseif ($e.FullName -like "assets/*" -and $e.Name -match '\.(lua|xml|ExportJson|plist|fnt)$') {
            $target = Join-Path $assets ($e.FullName.Substring(7) -replace '/', '\')
        }
        if ($target) {
            New-Item -ItemType Directory -Force (Split-Path $target) | Out-Null
            [System.IO.Compression.ZipFileExtensions]::ExtractToFile($e, $target, $true)
        }
    }
} finally {
    $zip.Dispose()
}
Write-Output "Extrait de $($apk.Name)."
