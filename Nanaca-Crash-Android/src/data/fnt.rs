//! Les polices BMFont (format texte d'AngelCode).

use std::collections::HashMap;

#[derive(Debug, Clone, Copy, Default)]
pub struct Glyph {
    /// Rectangle dans la page, en pixels.
    pub rect: [f32; 4],
    pub xoffset: f32,
    pub yoffset: f32,
    pub xadvance: f32,
}

#[derive(Debug, Default)]
pub struct Font {
    /// L'image de la page (dossier compris).
    pub page: String,
    pub line_height: f32,
    pub base: f32,
    pub glyphs: HashMap<char, Glyph>,
    pub kerning: HashMap<(char, char), f32>,
}

/// Une police ; `dir` est son dossier (« hd/ »).
pub fn parse(text: &str, dir: &str) -> Font {
    let mut font = Font::default();
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(kind) = words.next() else { continue };
        let fields: HashMap<&str, &str> = words.filter_map(|w| w.split_once('=')).collect();
        let num = |k: &str| fields.get(k).and_then(|v| v.parse::<f32>().ok()).unwrap_or(0.0);
        let ch = |k: &str| fields.get(k).and_then(|v| v.parse::<u32>().ok()).and_then(char::from_u32);
        match kind {
            "common" => {
                font.line_height = num("lineHeight");
                font.base = num("base");
            }
            "page" if fields.get("id") == Some(&"0") => {
                font.page = format!("{dir}{}", fields.get("file").map_or("", |f| f.trim_matches('"')));
            }
            "char" => {
                if let Some(c) = ch("id") {
                    font.glyphs.insert(c, Glyph { rect: [num("x"), num("y"), num("width"), num("height")], xoffset: num("xoffset"), yoffset: num("yoffset"), xadvance: num("xadvance") });
                }
            }
            "kerning" => {
                if let (Some(a), Some(b)) = (ch("first"), ch("second")) {
                    font.kerning.insert((a, b), num("amount"));
                }
            }
            _ => {}
        }
    }
    font
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let f = super::parse("common lineHeight=64 base=50 scaleW=256\npage id=0 file=\"x_0.png\"\nchar id=65 x=1 y=2 width=3 height=4 xoffset=5 yoffset=6 xadvance=7 page=0\nkerning first=65 second=65 amount=-2", "hd/");
        assert_eq!(f.page, "hd/x_0.png");
        assert_eq!(f.line_height, 64.0);
        assert_eq!(f.glyphs[&'A'].xadvance, 7.0);
        assert_eq!(f.kerning[&('A', 'A')], -2.0);
    }
}
