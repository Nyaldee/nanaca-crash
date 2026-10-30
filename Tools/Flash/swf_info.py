"""Résumé d'un fichier SWF : en-tête (version, taille de la scène, images
par seconde, nombre d'images), version d'ActionScript, et nombre de balises
de chaque sorte (formes, clips, images, sons, polices, code). Lecture seule.

Usage : python swf_info.py <fichier.swf>
"""

import struct
import sys
import zlib
from collections import Counter

TAGS = {
    0: "End", 1: "ShowFrame", 2: "DefineShape", 4: "PlaceObject", 5: "RemoveObject",
    6: "DefineBits", 7: "DefineButton", 8: "JPEGTables", 9: "SetBackgroundColor",
    10: "DefineFont", 11: "DefineText", 12: "DoAction", 13: "DefineFontInfo",
    14: "DefineSound", 15: "StartSound", 17: "DefineButtonSound", 18: "SoundStreamHead",
    19: "SoundStreamBlock", 20: "DefineBitsLossless", 21: "DefineBitsJPEG2",
    22: "DefineShape2", 24: "Protect", 26: "PlaceObject2", 28: "RemoveObject2",
    32: "DefineShape3", 33: "DefineText2", 34: "DefineButton2", 35: "DefineBitsJPEG3",
    36: "DefineBitsLossless2", 37: "DefineEditText", 39: "DefineSprite",
    43: "FrameLabel", 45: "SoundStreamHead2", 46: "DefineMorphShape", 48: "DefineFont2",
    56: "ExportAssets", 57: "ImportAssets", 58: "EnableDebugger", 59: "DoInitAction",
    62: "DefineFontInfo2", 64: "EnableDebugger2", 65: "ScriptLimits", 69: "FileAttributes",
    70: "PlaceObject3", 73: "DefineFontAlignZones", 74: "CSMTextSettings", 75: "DefineFont3",
    76: "SymbolClass", 77: "Metadata", 78: "DefineScalingGrid", 82: "DoABC",
    83: "DefineShape4", 84: "DefineMorphShape2", 86: "DefineSceneAndFrameLabelData",
    87: "DefineBinaryData", 88: "DefineFontName",
}


def tags(body, pos, end, counts, code_bytes, depth=0):
    while pos < end:
        head, = struct.unpack_from("<H", body, pos)
        code, length = head >> 6, head & 0x3F
        pos += 2
        if length == 0x3F:
            length, = struct.unpack_from("<I", body, pos)
            pos += 4
        counts[TAGS.get(code, f"Tag{code}")] += 1
        if code in (12, 59, 82, 72):
            code_bytes[TAGS.get(code, str(code))] += length
        if code == 39:  # DefineSprite : identifiant, nombre d'images, puis ses balises
            tags(body, pos + 4, pos + length, counts, code_bytes, depth + 1)
        pos += length
        if code == 0 and depth == 0:
            break


def main():
    data = open(sys.argv[1], "rb").read()
    sig, version, size = data[:3], data[3], struct.unpack_from("<I", data, 4)[0]
    body = data[:8] + (zlib.decompress(data[8:]) if sig == b"CWS" else data[8:])
    print(f"{sig.decode()} version {version}, {size} bytes uncompressed")
    nbits = body[8] >> 3
    rect_bytes = (5 + 4 * nbits + 7) // 8
    bits = int.from_bytes(body[8:8 + rect_bytes], "big")
    total = rect_bytes * 8
    vals = [(bits >> (total - 5 - nbits * (i + 1))) & ((1 << nbits) - 1) for i in range(4)]
    pos = 8 + rect_bytes
    rate = body[pos + 1] + body[pos] / 256
    frames, = struct.unpack_from("<H", body, pos + 2)
    print(f"stage {vals[1] / 20:g} x {vals[3] / 20:g}, {rate:g} fps, {frames} frames")
    counts, code_bytes = Counter(), Counter()
    tags(body, pos + 4, len(body), counts, code_bytes)
    print("ActionScript:", "3 (DoABC)" if code_bytes.get("DoABC") else "1/2 (DoAction)" if code_bytes else "none")
    for name, n in sorted(counts.items(), key=lambda kv: -kv[1]):
        print(f"  {name:<24} {n:>6}" + (f"  ({code_bytes[name]} bytes)" if name in code_bytes else ""))


if __name__ == "__main__":
    main()
