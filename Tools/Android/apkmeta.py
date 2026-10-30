"""Affiche la fin (métadonnées) d'un fichier texte de l'APK et liste les
fichiers dont le nom contient un motif. Usage : apkmeta.py apk fichier motif"""
import sys
import zipfile

with zipfile.ZipFile(sys.argv[1]) as z:
    text = z.read(sys.argv[2]).decode("utf-8", "replace")
    print(text[-700:])
    for info in z.infolist():
        if sys.argv[3] in info.filename:
            print(info.filename, info.file_size)
