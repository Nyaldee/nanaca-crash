//! Les formes du SWF : styles de remplissage et de trait, puis les
//! enregistrements de bords, regroupés par style.

use super::reader::{Reader, TWIPS};
use crate::geom::Matrix;
use crate::library::{Edge, Fill, Gradient, GradientKind, Layer, Line, Morph, Segment, Spread};

/// Côté du carré d'un dégradé : 32 768 vingtièmes de pixel, soit ±819,2
/// pixels autour de son centre.
const GRADIENT_HALF: f32 = 16384.0 / TWIPS;

/// Une forme de DefineShape, DefineShape2 ou DefineShape3 (`version` 1 à
/// 3), après son identifiant et son rectangle.
pub fn shape(r: &mut Reader, version: u8) -> Option<Vec<Layer>> {
    let (fills, lines) = styles(r, version)?;
    let groups = records(r, version, fills, lines)?;
    Some(groups.iter().map(|(fills, lines, edges)| layer(fills, lines, edges)).collect())
}

/// Les bords d'un glyphe de police (un seul remplissage implicite).
pub fn glyph(r: &mut Reader) -> Option<Vec<Segment>> {
    let groups = records(r, 1, Vec::new(), Vec::new())?;
    Some(groups.iter().flat_map(|(_, _, edges)| edges.iter().filter(|e| (e.fill0 != 0) != (e.fill1 != 0)).map(|e| e.segment)).collect())
}

/// Le calque d'un groupe de styles : chaque style avec ses bords.
pub fn layer(fills: &[Fill], lines: &[Line], edges: &[Edge]) -> Layer {
    let mut layer = Layer::default();
    for (i, fill) in fills.iter().enumerate() {
        let index = i as u16 + 1;
        // Un bord rempli des deux côtés par le même style n'en borde aucune
        // région.
        let segments: Vec<Segment> = edges.iter().filter(|e| (e.fill0 == index) != (e.fill1 == index)).map(|e| e.segment).collect();
        if !segments.is_empty() {
            layer.fills.push((fill.clone(), segments));
        }
    }
    for (i, line) in lines.iter().enumerate() {
        let index = i as u16 + 1;
        let segments: Vec<Segment> = edges.iter().filter(|e| e.line == index).map(|e| e.segment).collect();
        if !segments.is_empty() {
            layer.lines.push((*line, segments));
        }
    }
    layer
}

fn color(r: &mut Reader, version: u8) -> Option<[f32; 4]> {
    if version >= 3 { r.rgba() } else { r.rgb() }
}

/// Nombre d'éléments d'une liste de styles (sur 16 bits au-delà de 254).
fn count(r: &mut Reader, version: u8) -> Option<usize> {
    let n = r.u8()?;
    Some(if n == 0xFF && version >= 2 { usize::from(r.u16()?) } else { usize::from(n) })
}

fn styles(r: &mut Reader, version: u8) -> Option<(Vec<Fill>, Vec<Line>)> {
    let fills = (0..count(r, version)?).map(|_| fill(r, version)).collect::<Option<Vec<_>>>()?;
    let lines = (0..count(r, version)?)
        .map(|_| {
            let width = f32::from(r.u16()?) / TWIPS;
            Some(Line { width, color: color(r, version)? })
        })
        .collect::<Option<Vec<_>>>()?;
    Some((fills, lines))
}

fn fill(r: &mut Reader, version: u8) -> Option<Fill> {
    let kind = r.u8()?;
    Some(match kind {
        0x00 => Fill::Solid(color(r, version)?),
        0x10 | 0x12 | 0x13 => {
            let matrix = gradient_matrix(r.matrix()?);
            let flags = r.u8()?;
            let stops = (0..flags & 0x0F)
                .map(|_| {
                    let ratio = f32::from(r.u8()?) / 255.0;
                    Some((ratio, color(r, version)?))
                })
                .collect::<Option<Vec<_>>>()?;
            let kind = match kind {
                0x10 => GradientKind::Linear,
                0x12 => GradientKind::Radial,
                _ => GradientKind::Focal(r.fixed8()?),
            };
            Fill::Gradient(Gradient { kind, matrix, spread: spread(flags >> 6), stops })
        }
        0x40..=0x43 => {
            let id = r.u16()?;
            let matrix = bitmap_matrix(r.matrix()?);
            Fill::Bitmap { id, matrix, repeat: kind & 1 == 0, smooth: kind & 2 == 0 }
        }
        _ => return None,
    })
}

fn spread(mode: u8) -> Spread {
    match mode {
        1 => Spread::Reflect,
        2 => Spread::Repeat,
        _ => Spread::Pad,
    }
}

/// La matrice d'un dégradé, du carré -1..1 vers la forme en pixels.
fn gradient_matrix(m: Matrix) -> Matrix {
    Matrix { a: m.a * GRADIENT_HALF, b: m.b * GRADIENT_HALF, c: m.c * GRADIENT_HALF, d: m.d * GRADIENT_HALF, ..m }
}

/// La matrice d'une image, de ses pixels vers la forme en pixels (le SWF
/// compte un pixel d'image pour un vingtième de pixel).
fn bitmap_matrix(m: Matrix) -> Matrix {
    Matrix { a: m.a / TWIPS, b: m.b / TWIPS, c: m.c / TWIPS, d: m.d / TWIPS, ..m }
}

/// Les enregistrements d'une forme : ses bords, en groupes qui ont chacun
/// leurs styles (un nouveau groupe à chaque changement de liste de styles).
fn records(r: &mut Reader, version: u8, fills: Vec<Fill>, lines: Vec<Line>) -> Option<Vec<(Vec<Fill>, Vec<Line>, Vec<Edge>)>> {
    r.align();
    let mut fill_bits = r.ubits(4)?;
    let mut line_bits = r.ubits(4)?;
    let mut groups = vec![(fills, lines, Vec::new())];
    // Position du crayon en vingtièmes de pixel, et styles en cours.
    let (mut x, mut y) = (0i32, 0i32);
    let (mut fill0, mut fill1, mut line) = (0u16, 0u16, 0u16);
    let point = |x: i32, y: i32| [x as f32 / TWIPS, y as f32 / TWIPS];
    loop {
        if r.flag()? {
            // Bord.
            let start = point(x, y);
            let segment = if r.flag()? {
                let n = r.ubits(4)? + 2;
                if r.flag()? {
                    x += r.sbits(n)?;
                    y += r.sbits(n)?;
                } else if r.flag()? {
                    y += r.sbits(n)?;
                } else {
                    x += r.sbits(n)?;
                }
                Segment::Line(start, point(x, y))
            } else {
                let n = r.ubits(4)? + 2;
                let (cx, cy) = (x + r.sbits(n)?, y + r.sbits(n)?);
                x = cx + r.sbits(n)?;
                y = cy + r.sbits(n)?;
                Segment::Curve(start, point(cx, cy), point(x, y))
            };
            groups.last_mut()?.2.push(Edge { fill0, fill1, line, segment });
            continue;
        }
        let flags = r.ubits(5)?;
        if flags == 0 {
            break;
        }
        if flags & 1 != 0 {
            let n = r.ubits(5)?;
            x = r.sbits(n)?;
            y = r.sbits(n)?;
        }
        if flags & 2 != 0 {
            fill0 = r.ubits(fill_bits)? as u16;
        }
        if flags & 4 != 0 {
            fill1 = r.ubits(fill_bits)? as u16;
        }
        if flags & 8 != 0 {
            line = r.ubits(line_bits)? as u16;
        }
        if flags & 16 != 0 && version >= 2 {
            let (fills, lines) = styles(r, version)?;
            groups.push((fills, lines, Vec::new()));
            r.align();
            fill_bits = r.ubits(4)?;
            line_bits = r.ubits(4)?;
        }
    }
    Some(groups)
}

/// DefineMorphShape, après son identifiant.
pub fn morph(r: &mut Reader) -> Option<Morph> {
    let start_bounds = r.rect()?;
    let end_bounds = r.rect()?;
    let _end_offset = r.u32()?;
    let fills = (0..count(r, 2)?).map(|_| morph_fill(r)).collect::<Option<Vec<_>>>()?;
    let lines = (0..count(r, 2)?)
        .map(|_| {
            let (w0, w1) = (f32::from(r.u16()?) / TWIPS, f32::from(r.u16()?) / TWIPS);
            let (c0, c1) = (r.rgba()?, r.rgba()?);
            Some((Line { width: w0, color: c0 }, Line { width: w1, color: c1 }))
        })
        .collect::<Option<Vec<_>>>()?;
    let start = records(r, 1, Vec::new(), Vec::new())?.into_iter().flat_map(|g| g.2).collect();
    let end = records(r, 1, Vec::new(), Vec::new())?.into_iter().flat_map(|g| g.2).collect();
    Some(Morph { start_bounds, end_bounds, fills, lines, start, end })
}

fn morph_fill(r: &mut Reader) -> Option<(Fill, Fill)> {
    let kind = r.u8()?;
    Some(match kind {
        0x00 => (Fill::Solid(r.rgba()?), Fill::Solid(r.rgba()?)),
        0x10 | 0x12 => {
            let (m0, m1) = (gradient_matrix(r.matrix()?), gradient_matrix(r.matrix()?));
            let mut s0 = Vec::new();
            let mut s1 = Vec::new();
            for _ in 0..r.u8()? & 0x0F {
                s0.push((f32::from(r.u8()?) / 255.0, r.rgba()?));
                s1.push((f32::from(r.u8()?) / 255.0, r.rgba()?));
            }
            let kind = if kind == 0x10 { GradientKind::Linear } else { GradientKind::Radial };
            let g = |matrix, stops| Fill::Gradient(Gradient { kind, matrix, spread: Spread::Pad, stops });
            (g(m0, s0), g(m1, s1))
        }
        0x40..=0x43 => {
            let id = r.u16()?;
            let (repeat, smooth) = (kind & 1 == 0, kind & 2 == 0);
            let (m0, m1) = (bitmap_matrix(r.matrix()?), bitmap_matrix(r.matrix()?));
            (Fill::Bitmap { id, matrix: m0, repeat, smooth }, Fill::Bitmap { id, matrix: m1, repeat, smooth })
        }
        _ => return None,
    })
}
