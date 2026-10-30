"""Chaînes PIC du .so : pour chaque « DAT:décalage » (adresses Ghidra),
la chaîne en mot(DAT) + décalage. Usage : s.py so 1c10e4:1c0d94 ..."""
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


def at(addr):
    for v, o, s in segments:
        if v <= addr < v + s:
            return o + addr - v
    raise ValueError(hex(addr))


for arg in sys.argv[2:]:
    dat, off = (int(x, 16) for x in arg.split(":"))
    word, = struct.unpack_from("<I", data, at(dat - 0x10000))
    target = (word + off) & 0xFFFFFFFF
    p = at(target - 0x10000)
    end = data.index(b"\0", p)
    print(arg, ascii(data[p:end].decode("utf-8", "replace")), data[p:p + 8].hex())
