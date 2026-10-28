#!/usr/bin/env bash
# Compile les portages pour Linux en release et prépare les archives à
# partager :
#   target/Nanaca.Crash.Flash.Linux.tar.gz (nanaca_crash_flash)
#   target/Nanaca.Crash.Android.Linux.tar.gz (nanaca_crash_android)
#   target/Nanaca.Crash.All.Linux.tar.gz (les deux exécutables)
# Les archives ne contiennent que les exécutables : chaque joueur fournit
# son propre fichier original (voir README.md).
# Il faut Rust (rustup), pkg-config et la bibliothèque du son ALSA :
#   sudo apt install pkg-config libasound2-dev
# Sous Windows, build_linux.bat lance ce script dans WSL.

set -euo pipefail
cd "$(dirname "$0")"

# Lancé par build_linux.bat, bash n'est ni interactif ni un shell de
# connexion : il ne lit pas ~/.profile ni ~/.bashrc, où rustup ajoute
# ~/.cargo/bin au PATH.
if [ -f "$HOME/.cargo/env" ]; then
    . "$HOME/.cargo/env"
fi

if ! command -v cargo >/dev/null; then
    echo "Rust est introuvable : https://rustup.rs" >&2
    exit 1
fi
if ! pkg-config --exists alsa 2>/dev/null; then
    echo "La bibliothèque du son manque : sudo apt install pkg-config libasound2-dev" >&2
    exit 1
fi

# Un dossier à part : les fichiers de Windows restent dans target/release.
TARGET=target/linux
cargo build --release -p nanaca-crash-flash -p nanaca-crash-android --target-dir "$TARGET"

# Nom de l'archive, nom de l'exécutable.
for port in "Nanaca.Crash.Flash nanaca_crash_flash" "Nanaca.Crash.Android nanaca_crash_android"; do
    set -- $port
    name="$1.Linux.tar.gz"
    tar -czf "target/$name" -C "$TARGET/release" "$2"
    echo "target/$name est prêt."
done

# Les deux exécutables ensemble.
tar -czf target/Nanaca.Crash.All.Linux.tar.gz -C "$TARGET/release" nanaca_crash_flash nanaca_crash_android
echo "target/Nanaca.Crash.All.Linux.tar.gz est prêt."
