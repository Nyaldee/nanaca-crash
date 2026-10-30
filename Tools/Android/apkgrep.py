"""Cherche des noms dans les fichiers texte de l'APK (lecture seule).
Usage : apkgrep.py apk motif..."""
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as z:
    for info in z.infolist():
        if not info.filename.endswith((".plist", ".ExportJson", ".fnt", ".xml")):
            continue
        text = z.read(info).decode("utf-8", "replace")
        for word in sys.argv[2:]:
            if word in text:
                print(info.filename, word)
