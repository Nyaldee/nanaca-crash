"""Affiche le CRC32 de chaque fichier donné (pour vérifier un fichier
original du jeu).

Usage : python crc32.py <fichier> [<fichier>...]
"""

import sys
import zlib

for path in sys.argv[1:]:
    crc = 0
    with open(path, "rb") as f:
        while chunk := f.read(1 << 20):
            crc = zlib.crc32(chunk, crc)
    print(f"{crc:08X}  {path}")
