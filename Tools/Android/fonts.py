"""Prépare les polices du portage Android dans Nanaca-Crash-Android/fonts.

Le jeu demande « Arial » et « monospace » : Android les affichait en Roboto
et en Droid Sans Mono, complétées par Noto Sans CJK pour le japonais et le
chinois traditionnel. Ce script prend ces polices téléchargées dans
Tools/work/fonts (Roboto 2.138 « android », Droid Sans Mono de l'AOSP,
Noto Sans JP et TC 2.004) et réduit les deux Noto aux caractères des
textes du jeu, lus dans l'APK.

Usage : python -I fonts.py <APK>
"""

import shutil
import sys
import zipfile
from pathlib import Path

from fontTools import subset

ROOT = Path(__file__).resolve().parents[2]
WORK = ROOT / "Tools" / "work" / "fonts"
OUT = ROOT / "Nanaca-Crash-Android" / "fonts"

# Les fichiers de textes de l'APK (toutes les langues).
TEXTS = ("string.xml", "string_ja.xml", "string_zh-tw.xml", "gallery.xml", "gallery_ja.xml", "gallery_zh-tw.xml")


def unzip(archive, member, target):
    with zipfile.ZipFile(WORK / archive) as z, z.open(member) as src, open(target, "wb") as dst:
        shutil.copyfileobj(src, dst)


def main():
    OUT.mkdir(parents=True, exist_ok=True)
    with zipfile.ZipFile(sys.argv[1]) as apk:
        chars = set()
        for name in TEXTS:
            chars |= set(apk.read(f"assets/{name}").decode("utf-8"))
    # ASCII imprimable en plus, pour les textes écrits dans le code.
    chars |= {chr(c) for c in range(0x20, 0x7F)}
    text = "".join(sorted(c for c in chars if c >= " "))

    unzip("roboto-android.zip", "Roboto-Regular.ttf", OUT / "Roboto-Regular.ttf")
    unzip("roboto-android.zip", "LICENSE", OUT / "LICENSE-Roboto.txt")
    shutil.copy(WORK / "DroidSansMono.ttf", OUT / "DroidSansMono.ttf")
    shutil.copy(WORK / "NOTICE-DroidSansMono.txt", OUT / "LICENSE-DroidSansMono.txt")
    for lang in ("JP", "TC"):
        source = WORK / f"NotoSans{lang}-Regular.otf"
        unzip(f"NotoSans{lang}.zip", f"NotoSans{lang}-Regular.otf", source)
        options = subset.Options()
        options.layout_features = ["*"]
        font = subset.load_font(str(source), options)
        sub = subset.Subsetter(options)
        sub.populate(text=text)
        sub.subset(font)
        subset.save_font(font, str(OUT / f"NotoSans{lang}-Regular-Nanaca.otf"), options)
    unzip("NotoSansJP.zip", "LICENSE", OUT / "LICENSE-NotoSansCJK.txt")
    print(f"{len(text)} caractères ; polices dans {OUT}")


if __name__ == "__main__":
    main()
