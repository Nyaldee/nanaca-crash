"""Liste les symboles dynamiques d'une bibliothèque ELF (.so d'Android) :
nom, adresse et taille, filtrés par un motif. Lecture seule : rien n'est
exécuté.

Usage : python elf_symbols.py <fichier.so> [motif]
"""

import re
import struct
import sys


def main():
    data = open(sys.argv[1], "rb").read()
    pattern = re.compile(sys.argv[2]) if len(sys.argv) > 2 else None
    if data[:4] != b"\x7fELF" or data[4] != 1:
        sys.exit("ELF 32 bits attendu")
    shoff, = struct.unpack_from("<I", data, 0x20)
    shentsize, shnum = struct.unpack_from("<HH", data, 0x2E)
    sections = [struct.unpack_from("<IIIIIIIIII", data, shoff + i * shentsize) for i in range(shnum)]
    # La table des symboles dynamiques (type 11) et ses noms (sa section liée).
    for sec in sections:
        if sec[1] != 11:
            continue
        offset, size, link, entsize = sec[4], sec[5], sec[6], sec[9]
        strtab = sections[link][4]
        for i in range(size // entsize):
            name_off, value, sym_size, info = struct.unpack_from("<IIIB", data, offset + i * entsize)
            end = data.index(b"\0", strtab + name_off)
            name = data[strtab + name_off:end].decode("latin-1")
            if name and (pattern is None or pattern.search(name)) and info & 0xF == 2:
                print(f"{value:08X} {sym_size:6d} {name}")


if __name__ == "__main__":
    main()
