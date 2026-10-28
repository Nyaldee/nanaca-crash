//! Les étiquettes en police système (CCLabelTTF). Le jeu demande « Arial »
//! ou « monospace » ; Android les dessinait en Roboto ou Droid Sans Mono,
//! complétées par Noto Sans CJK pour le japonais et le chinois : ce sont ces
//! polices, fournies avec le jeu (dossier fonts), qui sont dessinées ici en
//! blanc sur transparent, à la taille en pixels de l'écran « hd ». Lignes,
//! retours à la ligne et alignements suivent Cocos2dxBitmap.

use std::cell::Cell;
use std::sync::OnceLock;

use fontdue::{Font, FontSettings};

use crate::assets::CONTENT_SCALE;
use crate::render::Texture;

/// Une image de texte prête à afficher.
pub struct TextImage {
    pub width: u32,
    pub height: u32,
    /// RGBA prémultiplié (blanc, l'alpha porte la couverture).
    pub rgba: Vec<u8>,
    /// La texture, créée au premier dessin.
    pub texture: Cell<Option<Texture>>,
}

impl TextImage {
    /// Taille en points.
    pub fn size(&self) -> (f32, f32) {
        (self.width as f32 / CONTENT_SCALE, self.height as f32 / CONTENT_SCALE)
    }
}

/// Alignements : 0 gauche / haut, 1 centre, 2 droite / bas.
pub struct TextStyle<'a> {
    pub font: &'a str,
    /// Taille en points.
    pub size: f32,
    /// Boîte imposée en points (0 : à la taille du texte).
    pub dimensions: (f32, f32),
    pub h_align: u8,
    pub v_align: u8,
}

/// L'écriture des idéogrammes, selon la langue du jeu.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Cjk {
    Japanese,
    Traditional,
}

struct Fonts {
    roboto: Font,
    mono: Font,
    japanese: Font,
    traditional: Font,
}

fn fonts() -> &'static Fonts {
    static FONTS: OnceLock<Fonts> = OnceLock::new();
    FONTS.get_or_init(|| {
        let load = |bytes: &[u8]| Font::from_bytes(bytes, FontSettings::default()).expect("police fournie");
        Fonts {
            roboto: load(include_bytes!("../fonts/Roboto-Regular.ttf")),
            mono: load(include_bytes!("../fonts/DroidSansMono.ttf")),
            japanese: load(include_bytes!("../fonts/NotoSansJP-Regular-Nanaca.otf")),
            traditional: load(include_bytes!("../fonts/NotoSansTC-Regular-Nanaca.otf")),
        }
    })
}

thread_local! {
    static CJK: Cell<Cjk> = const { Cell::new(Cjk::Japanese) };
}

/// La police de repli des idéogrammes (langue du jeu).
pub fn set_cjk(cjk: Cjk) {
    CJK.with(|c| c.set(cjk));
}

pub fn render(text: &str, style: &TextStyle) -> TextImage {
    let f = fonts();
    let primary = if style.font.eq_ignore_ascii_case("monospace") { &f.mono } else { &f.roboto };
    let fallback = if CJK.with(Cell::get) == Cjk::Traditional { &f.traditional } else { &f.japanese };
    let px = style.size * CONTENT_SCALE;
    // La police de chaque caractère : la principale, sinon celle des
    // idéogrammes.
    let font_of = |c: char| if primary.lookup_glyph_index(c) != 0 || fallback.lookup_glyph_index(c) == 0 { primary } else { fallback };
    let advance = |c: char| font_of(c).metrics(c, px).advance_width;
    let measure = |s: &str| s.chars().map(advance).sum::<f32>();

    // Les lignes : coupées aux retours, puis à la largeur imposée (aux
    // espaces, ou entre deux caractères faute d'espace).
    let max_w = style.dimensions.0 * CONTENT_SCALE;
    let mut lines: Vec<String> = Vec::new();
    for raw in text.split('\n') {
        if max_w <= 0.0 || measure(raw) <= max_w {
            lines.push(raw.to_string());
            continue;
        }
        let mut line = String::new();
        for word in raw.split_inclusive(' ') {
            if measure(&line) + measure(word.trim_end()) <= max_w {
                line.push_str(word);
                continue;
            }
            if !line.is_empty() {
                lines.push(line.trim_end().to_string());
                line.clear();
            }
            for c in word.chars() {
                if !line.is_empty() && measure(&line) + advance(c) > max_w {
                    lines.push(line.trim_end().to_string());
                    line.clear();
                }
                line.push(c);
            }
        }
        lines.push(line.trim_end().to_string());
    }

    let m = primary.horizontal_line_metrics(px).expect("mesures de ligne");
    let (ascent, line_h) = (m.ascent.ceil(), (m.ascent - m.descent).ceil());
    let text_w = lines.iter().map(|l| measure(l)).fold(0.0f32, f32::max).ceil().max(1.0);
    let text_h = (line_h * lines.len() as f32).max(1.0);
    let w = if style.dimensions.0 > 0.0 { max_w.ceil() } else { text_w } as usize;
    let h = if style.dimensions.1 > 0.0 { (style.dimensions.1 * CONTENT_SCALE).ceil() } else { text_h } as usize;
    let top = match style.v_align {
        1 => (h as f32 - text_h) / 2.0,
        2 => h as f32 - text_h,
        _ => 0.0,
    };
    let mut cover = vec![0u8; w * h];
    for (i, line) in lines.iter().enumerate() {
        let free = w as f32 - measure(line);
        let mut x = match style.h_align {
            1 => free / 2.0,
            2 => free,
            _ => 0.0,
        };
        let baseline = top + ascent + line_h * i as f32;
        for c in line.chars() {
            let (gm, bitmap) = font_of(c).rasterize(c, px);
            let left = (x + gm.xmin as f32).round() as i32;
            let top_y = (baseline - (gm.ymin as f32 + gm.height as f32)).round() as i32;
            for gy in 0..gm.height {
                for gx in 0..gm.width {
                    let (px_x, px_y) = (left + gx as i32, top_y + gy as i32);
                    if px_x >= 0 && px_y >= 0 && (px_x as usize) < w && (px_y as usize) < h {
                        let p = &mut cover[px_y as usize * w + px_x as usize];
                        *p = (*p).max(bitmap[gy * gm.width + gx]);
                    }
                }
            }
            x += gm.advance_width;
        }
    }
    let rgba = cover.iter().flat_map(|&a| [a; 4]).collect();
    TextImage { width: w as u32, height: h as u32, rgba, texture: Cell::new(None) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fallback() {
        let style = TextStyle { font: "Arial", size: 12.0, dimensions: (0.0, 0.0), h_align: 0, v_align: 0 };
        let latin = render("SOUND", &style);
        assert!(latin.width > 10 && latin.rgba.iter().any(|&a| a > 0));
        // Des idéogrammes des textes du jeu sont dessinés (Noto).
        let ja = render("の", &style);
        assert!(ja.rgba.iter().any(|&a| a > 0));
    }

    #[test]
    fn wrap() {
        let style = TextStyle { font: "Arial", size: 12.0, dimensions: (60.0, 0.0), h_align: 1, v_align: 0 };
        let one = render("A", &style);
        let many = render("many words that do not fit on one line", &style);
        assert!(many.height > one.height * 2);
        assert_eq!(many.width, 120);
    }
}
