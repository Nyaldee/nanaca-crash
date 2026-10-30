"""Mots du .so aux adresses Ghidra données : entier, flottant, double.
Usage : f.py so 1cc454 ..."""
import struct
import sys

data = open(sys.argv[1], "rb").read()
e_phoff, = struct.unpack_from("<I", data, 28)
e_phentsize, e_phnum = struct.unpack_from("<HH", data, 42)
segments = []
for i in range(e_phnum):
    p_type, p_offset, p_vaddr, _, p_filesz, _, _, _ = struct.unpack_from("<8I", data, e_phoff + i * e_phentsize)
    if p_type == 1:
        segments.append((p_vaddr, p_offset, p_filesz))

for arg in sys.argv[2:]:
    addr = int(arg, 16) - 0x10000
    for v, o, s in segments:
        if v <= addr < v + s:
            p = o + addr - v
            i, = struct.unpack_from("<i", data, p)
            f, = struct.unpack_from("<f", data, p)
            d, = struct.unpack_from("<d", data, p)
            print(arg, i, f, d)
