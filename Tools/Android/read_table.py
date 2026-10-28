"""Lit un tableau de chaînes du .so : l'entrée de GOT donnée pointe vers
un tableau de N pointeurs de chaînes. Usage : read_table.py so got n"""
import struct
import sys

data = open(sys.argv[1], "rb").read()
# Adresses de Ghidra : le .so y est chargé à 0x10000.
got, count = int(sys.argv[2], 16) - 0x10000, int(sys.argv[3])
e_phoff, = struct.unpack_from("<I", data, 28)
e_phentsize, e_phnum = struct.unpack_from("<HH", data, 42)
segments = []
for i in range(e_phnum):
    p_type, p_offset, p_vaddr, _, p_filesz, _, _, _ = struct.unpack_from("<8I", data, e_phoff + i * e_phentsize)
    if p_type == 1:
        segments.append((p_vaddr, p_offset, p_filesz))


def word(addr):
    for v, o, s in segments:
        if v <= addr < v + s:
            return struct.unpack_from("<I", data, o + addr - v)[0]
    raise ValueError(hex(addr))


def cstr(addr):
    for v, o, s in segments:
        if v <= addr < v + s:
            end = data.index(b"\0", o + addr - v)
            return data[o + addr - v:end].decode("utf-8", "replace")
    return f"?{addr:x}"


table = word(got)
print(f"tableau @{table:08x}")
for i in range(count):
    print(i, cstr(word(table + 4 * i)))
