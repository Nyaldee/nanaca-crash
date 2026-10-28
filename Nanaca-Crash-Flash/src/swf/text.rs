//! Polices (DefineFont3), textes fixes (DefineText) et champs de texte
//! (DefineEditText).

use std::collections::HashMap;

use super::reader::{Reader, TWIPS};
use super::shape;
use crate::library::{Align, EditText, Font, Glyph, PlacedGlyph, Text};

/// DefineFont3, après son identifiant. Ses coordonnées sont vingt fois plus
/// fines que celles des formes : une fois divisées, le carré « em » mesure
/// 1 024 unités.
pub fn font3(r: &mut Reader) -> Option<Font> {
    let flags = r.u8()?;
    let (has_layout, wide_offsets) = (flags & 0x80 != 0, flags & 0x08 != 0);
    let _language = r.u8()?;
    // Le nom de la police.
    let name_len = usize::from(r.u8()?);
    r.bytes(name_len)?;
    let count = usize::from(r.u16()?);
    // Les décalages des glyphes partent du début de leur table.
    let table = r.rest();
    let mut t = Reader::new(table);
    let offset = |t: &mut Reader| if wide_offsets { t.u32().map(|v| v as usize) } else { t.u16().map(usize::from) };
    let offsets = (0..=count).map(|_| offset(&mut t)).collect::<Option<Vec<_>>>()?;
    let mut glyphs = Vec::with_capacity(count);
    for i in 0..count {
        let data = table.get(offsets[i]..offsets[i + 1])?;
        glyphs.push(Glyph { segments: shape::glyph(&mut Reader::new(data))?, advance: 0.0 });
    }
    // La table des caractères, puis la mise en page.
    let mut r = Reader::new(table.get(offsets[count]..)?);
    let mut codes = HashMap::new();
    for i in 0..count {
        codes.insert(r.u16()?, i);
    }
    let (mut ascent, mut descent) = (0.0, 0.0);
    if has_layout {
        ascent = f32::from(r.u16()?) / TWIPS;
        descent = f32::from(r.u16()?) / TWIPS;
        let _leading = r.i16()?;
        for g in &mut glyphs {
            g.advance = f32::from(r.i16()?) / TWIPS;
        }
    }
    Some(Font { em: 1024.0, ascent, descent, glyphs, codes })
}

/// DefineText (`alpha` faux) ou DefineText2, après son identifiant.
pub fn text(r: &mut Reader, alpha: bool) -> Option<Text> {
    let bounds = r.rect()?;
    let matrix = r.matrix()?;
    let glyph_bits = u32::from(r.u8()?);
    let advance_bits = u32::from(r.u8()?);
    let mut glyphs = Vec::new();
    let (mut font, mut color, mut x, mut y, mut size) = (0, [0.0, 0.0, 0.0, 1.0], 0.0, 0.0, 0.0);
    loop {
        let flags = r.u8()?;
        if flags == 0 {
            break;
        }
        if flags & 8 != 0 {
            font = r.u16()?;
        }
        if flags & 4 != 0 {
            color = if alpha { r.rgba()? } else { r.rgb()? };
        }
        if flags & 1 != 0 {
            x = f32::from(r.i16()?) / TWIPS;
        }
        if flags & 2 != 0 {
            y = f32::from(r.i16()?) / TWIPS;
        }
        if flags & 8 != 0 {
            size = f32::from(r.u16()?) / TWIPS;
        }
        for _ in 0..r.u8()? {
            let index = r.ubits(glyph_bits)? as usize;
            let advance = r.sbits(advance_bits)? as f32 / TWIPS;
            glyphs.push(PlacedGlyph { font, index, x, y, size, color });
            x += advance;
        }
        r.align();
    }
    Some(Text { bounds, matrix, glyphs })
}

/// DefineEditText, après son identifiant.
pub fn edit_text(r: &mut Reader) -> Option<EditText> {
    let bounds = r.rect()?;
    let f1 = r.u8()?;
    let f2 = r.u8()?;
    let flag = |byte: u8, bit: u8| byte & (1 << bit) != 0;
    let font = if flag(f1, 0) { Some(r.u16()?) } else { None };
    if flag(f2, 7) {
        r.string()?;
    }
    let size = if flag(f1, 0) { f32::from(r.u16()?) / TWIPS } else { 12.0 };
    let color = if flag(f1, 2) { r.rgba()? } else { [0.0, 0.0, 0.0, 1.0] };
    if flag(f1, 1) {
        r.u16()?;
    }
    let (mut align, mut left_margin, mut right_margin, mut leading) = (Align::Left, 0.0, 0.0, 0.0);
    if flag(f2, 5) {
        align = match r.u8()? {
            1 => Align::Right,
            2 => Align::Center,
            3 => Align::Justify,
            _ => Align::Left,
        };
        left_margin = f32::from(r.u16()?) / TWIPS;
        right_margin = f32::from(r.u16()?) / TWIPS;
        let _indent = r.u16()?;
        leading = f32::from(r.i16()?) / TWIPS;
    }
    let variable = r.string()?;
    let text = if flag(f1, 7) { r.string()? } else { String::new() };
    Some(EditText {
        bounds,
        font,
        size,
        color,
        align,
        left_margin,
        right_margin,
        leading,
        variable,
        text,
        html: flag(f2, 1),
        word_wrap: flag(f1, 6),
    })
}
