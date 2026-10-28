"""Rectangles des os d'une armature à la première image du premier
mouvement (lecture d'un ExportJson). Usage : armbox.py fichier.ExportJson"""
import json
import sys

d = json.load(open(sys.argv[1], encoding="utf-8"))
tex = {t["name"]: t for t in d.get("texture_data", [])}
arm = d["armature_data"][0]
mov = d["animation_data"][0]["mov_data"][0]
frames = {b["name"]: b["frame_data"][0] for b in mov["mov_bone_data"]}
print("mouvement :", mov["name"])
for b in arm["bone_data"]:
    f = frames.get(b["name"])
    x = (f["x"] if f else 0) + b.get("x", 0)
    y = (f["y"] if f else 0) + b.get("y", 0)
    di = f["dI"] if f else None
    for i, disp in enumerate(b.get("display_data", [])):
        name = disp["name"].rsplit(".", 1)[0]
        t = tex.get(name, {})
        w, h = t.get("width", 0), t.get("height", 0)
        px, py = t.get("pX", 0.5), t.get("pY", 0.5)
        sk = (disp.get("skin_data") or [{}])[0]
        sx, sy = sk.get("x", 0), sk.get("y", 0)
        x0 = x + sx - px * w
        print(f"{b['name']:12} dI={di} d{i} {name:20} bone=({x:.0f},{y:.0f}) skin=({sx},{sy}) {w}x{h} p=({px},{py}) x:[{x0:.0f},{x0 + w:.0f}] parent={b.get('parent','')}")
