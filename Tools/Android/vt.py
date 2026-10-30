"""Table virtuelle : l'entrée de GOT donnée (adresses Ghidra) pointe vers la
vtable ; affiche les entrées de début à fin (décalages) avec leur symbole.
Usage : vt.py so got debut fin"""
import struct
import sys

data = open(sys.argv[1], "rb").read()
e_phoff, = struct.unpack_from("<I", data, 28)
e_shoff, = struct.unpack_from("<I", data, 32)
e_phentsize, e_phnum = struct.unpack_from("<HH", data, 42)
e_shentsize, e_shnum = struct.unpack_from("<HH", data, 46)
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


symbols = {}
for i in range(e_shnum):
    name, typ, _, _, off, size, link, _, _, entsize = struct.unpack_from("<10I", data, e_shoff + i * e_shentsize)
    if typ in (2, 11):
        _, _, _, _, stroff, _, _, _, _, _ = struct.unpack_from("<10I", data, e_shoff + link * e_shentsize)
        for k in range(size // 16):
            st_name, st_value, _, _, _, _ = struct.unpack_from("<IIIBBH", data, off + 16 * k)
            if st_value:
                end = data.index(b"\0", stroff + st_name)
                symbols[st_value & ~1] = data[stroff + st_name:end].decode()

start, stop = (int(x, 16) for x in sys.argv[3:5])
if sys.argv[2].startswith("_Z"):
    # Symbole de vtable : les entrées commencent après offset et typeinfo.
    table = next(a for a, n in symbols.items() if n == sys.argv[2])
else:
    table = word(int(sys.argv[2], 16) - 0x10000)
print(f"vtable @{table + 0x10000:08x}")
for off in range(start, stop + 1, 4):
    f = word(table + off)
    print(f"{off:#05x} {f + 0x10000:08x} {symbols.get(f & ~1, '?')}")
