//! Le dessin de la liste d'affichage : par profondeur, matrices et
//! transformations de couleur composées le long de l'arbre, masques (un
//! objet avec une profondeur de masque cache ce qui sort de lui, pour les
//! profondeurs au-dessus jusqu'à celle-là), textes fixes et champs de
//! texte, états des boutons.

use crate::display::{Display, Kind, Obj};
use crate::geom::{ColorTransform, Matrix};
use crate::library::{Align, Character, EditText, Font, Id, Library};
use crate::render::{MeshKey, Renderer};

/// Marge d'un champ de texte autour de son texte, en pixels (comme Flash).
const GUTTER: f32 = 2.0;

pub fn draw(r: &mut Renderer, d: &Display) {
    let lib = d.lib.clone();
    object(r, d, &lib, d.root, Matrix::IDENTITY, ColorTransform::IDENTITY, false);
}

/// Un objet et ses descendants ; un masque se dessine même invisible.
fn object(r: &mut Renderer, d: &Display, lib: &Library, o: Obj, parent: Matrix, parent_cx: ColorTransform, mask: bool) {
    let Some(x) = d.get(o) else { return };
    if !x.visible && !mask {
        return;
    }
    let world = parent * x.matrix;
    let cx = parent_cx * x.color;
    match &x.kind {
        Kind::Clip(_) => children(r, d, lib, &d.children(o), world, cx, mask),
        Kind::Button(b) => {
            let state = b.state.flag();
            let shown: Vec<Obj> = b.children.iter().filter(|c| c.0 & state != 0).map(|c| c.1).collect();
            children(r, d, lib, &shown, world, cx, mask);
        }
        Kind::Graphic => {
            let Some(id) = x.character else { return };
            match lib.characters.get(&id) {
                Some(Character::Shape(_)) => r.draw(lib, MeshKey::Shape(id), world, cx, None),
                Some(Character::Morph(_)) => r.draw(lib, MeshKey::Morph(id, x.ratio), world, cx, None),
                Some(Character::Text(t)) => {
                    let m = world * t.matrix;
                    for g in &t.glyphs {
                        let Some(Character::Font(f)) = lib.characters.get(&g.font) else { continue };
                        let k = g.size / f.em;
                        r.draw(lib, MeshKey::Glyph(g.font, g.index), m * Matrix::translate(g.x, g.y) * Matrix::scale(k, k), cx, Some(g.color));
                    }
                }
                _ => {}
            }
        }
        Kind::Text(field) => {
            if let Some(Character::EditText(def)) = x.character.and_then(|id| lib.characters.get(&id)) {
                edit_text(r, lib, def, &field.text, field.autosize_right, world, cx);
            }
        }
    }
}

/// Des objets par profondeur ; un masque limite les profondeurs qu'il
/// couvre.
fn children(r: &mut Renderer, d: &Display, lib: &Library, list: &[Obj], world: Matrix, cx: ColorTransform, mask: bool) {
    let mut i = 0;
    while i < list.len() {
        let child = list[i];
        i += 1;
        let Some(x) = d.get(child) else { continue };
        if x.clip_depth == 0 || mask {
            object(r, d, lib, child, world, cx, mask);
            continue;
        }
        let limit = i32::from(x.clip_depth) + crate::display::TIMELINE_DEPTH;
        let masked: Vec<Obj> = list[i..].iter().copied().take_while(|&c| d.get(c).is_some_and(|m| m.depth <= limit)).collect();
        i += masked.len();
        r.mask_begin();
        object(r, d, lib, child, world, cx, true);
        r.mask_apply();
        for m in masked {
            object(r, d, lib, m, world, cx, false);
        }
        r.mask_remove_begin();
        object(r, d, lib, child, world, cx, true);
        r.mask_end();
    }
}

/// Un champ de texte : son texte mis en page avec sa police, ligne par
/// ligne. Un champ HTML dont le code a remplacé le texte garde le format de
/// son texte d'origine ; `autosize_right` le cale à droite.
fn edit_text(r: &mut Renderer, lib: &Library, def: &EditText, text: &str, autosize_right: bool, world: Matrix, cx: ColorTransform) {
    let Some(font_id) = def.font else { return };
    let Some(Character::Font(font)) = lib.characters.get(&font_id) else { return };
    let (text, mut align, color) = if def.html && text.contains('<') {
        strip_html(text, def)
    } else if def.html {
        let (_, align, color) = strip_html(&def.text, def);
        (text.to_string(), align, color)
    } else {
        (text.to_string(), def.align, def.color)
    };
    if autosize_right {
        align = Align::Right;
    }
    let k = def.size / font.em;
    let width = def.bounds.x_max - def.bounds.x_min - 2.0 * GUTTER - def.left_margin - def.right_margin;
    let line_height = (font.ascent + font.descent) * k + def.leading;
    let mut y = def.bounds.y_min + GUTTER + font.ascent * k;
    for paragraph in text.split(['\r', '\n']) {
        for line in wrap(font, k, paragraph, if def.word_wrap { width } else { f32::MAX }) {
            let glyphs: Vec<(usize, f32)> = line.chars().filter_map(|ch| font.codes.get(&(ch as u16)).map(|&i| (i, font.glyphs[i].advance * k))).collect();
            let line_width: f32 = glyphs.iter().map(|g| g.1).sum();
            let mut x = def.bounds.x_min
                + GUTTER
                + def.left_margin
                + match align {
                    Align::Right => width - line_width,
                    Align::Center => (width - line_width) / 2.0,
                    _ => 0.0,
                };
            for (index, advance) in glyphs {
                glyph(r, lib, font_id, index, world * Matrix::translate(x, y) * Matrix::scale(k, k), cx, color);
                x += advance;
            }
            y += line_height;
        }
    }
}

fn glyph(r: &mut Renderer, lib: &Library, font: Id, index: usize, world: Matrix, cx: ColorTransform, color: [f32; 4]) {
    r.draw(lib, MeshKey::Glyph(font, index), world, cx, Some(color));
}

/// Les lignes d'un paragraphe coupées aux espaces pour tenir dans `width`.
fn wrap<'a>(font: &Font, k: f32, paragraph: &'a str, width: f32) -> Vec<&'a str> {
    let advance = |s: &str| s.chars().filter_map(|ch| font.codes.get(&(ch as u16))).map(|&i| font.glyphs[i].advance * k).sum::<f32>();
    let mut lines = Vec::new();
    let mut start = 0;
    let mut last_space = None;
    for (i, ch) in paragraph.char_indices() {
        if ch == ' ' {
            last_space = Some(i);
        }
        if advance(&paragraph[start..i + ch.len_utf8()]) > width
            && let Some(s) = last_space.filter(|&s| s > start)
        {
            lines.push(&paragraph[start..s]);
            start = s + 1;
            last_space = None;
        }
    }
    lines.push(&paragraph[start..]);
    lines
}

/// Le texte d'un champ HTML : balises retirées (alignement et couleur
/// gardés), entités décodées.
fn strip_html(html: &str, def: &EditText) -> (String, Align, [f32; 4]) {
    let mut align = def.align;
    let mut color = def.color;
    let mut out = String::new();
    let mut rest = html;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('>') else { break };
        let tag = &rest[start + 1..start + end];
        let lower = tag.to_ascii_lowercase();
        if lower == "p" || lower.starts_with("p ") {
            for (key, a) in [("align=\"right\"", Align::Right), ("align=\"center\"", Align::Center), ("align=\"left\"", Align::Left)] {
                if lower.contains(key) {
                    align = a;
                }
            }
        }
        if lower == "/p" || lower == "br" {
            out.push('\n');
        }
        if let Some(i) = lower.find("color=\"#")
            && let Some(hex) = tag.get(i + 8..i + 14)
            && let Ok(v) = u32::from_str_radix(hex, 16)
        {
            color = [((v >> 16) & 255) as f32 / 255.0, ((v >> 8) & 255) as f32 / 255.0, (v & 255) as f32 / 255.0, 1.0];
        }
        rest = &rest[start + end + 1..];
    }
    out.push_str(rest);
    let out = out.trim_end_matches('\n').replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&");
    (out, align, color)
}
