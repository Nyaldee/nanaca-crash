//! Lecture du SWF : son en-tête, puis ses balises, rangées dans la
//! bibliothèque. Seules les balises utilisées par le jeu sont lues ; le code
//! ActionScript est ignoré, la logique du jeu étant réécrite en Rust.

mod bitmap;
mod reader;
mod shape;
mod text;

use std::collections::HashMap;

use crate::library::{Button, ButtonRecord, Character, Codec, Command, Frame, Id, Library, Place, Sound, StartSound, Timeline};
use reader::Reader;
pub use shape::layer;

/// Lit tout le SWF.
pub fn load(data: &[u8]) -> Result<Library, String> {
    let body = match data.get(..3) {
        Some(b"FWS") => data.get(8..).ok_or("truncated SWF")?.to_vec(),
        Some(b"CWS") => miniz_oxide::inflate::decompress_to_vec_zlib(data.get(8..).ok_or("truncated SWF")?).map_err(|_| "unreadable SWF (zlib)")?,
        _ => return Err("not a SWF file".into()),
    };
    let mut r = Reader::new(&body);
    let unreadable = || "unreadable SWF header".to_string();
    let stage = r.rect().ok_or_else(unreadable)?;
    let frame_rate = r.fixed8().ok_or_else(unreadable)?;
    let _frame_count = r.u16().ok_or_else(unreadable)?;
    let mut lib = Library {
        characters: HashMap::new(),
        root: Timeline::default(),
        exports: HashMap::new(),
        stage: (stage.x_max - stage.x_min, stage.y_max - stage.y_min),
        frame_rate,
        background: [1.0; 4],
    };
    lib.root = tags(&mut lib, r.rest())?;
    Ok(lib)
}

/// Les balises d'une timeline (la principale ou celle d'un clip) : les
/// définitions vont dans la bibliothèque, les images sont rendues.
fn tags(lib: &mut Library, data: &[u8]) -> Result<Timeline, String> {
    let mut r = Reader::new(data);
    let mut timeline = Timeline::default();
    let mut frame = Frame::default();
    while !r.is_empty() {
        let (code, body) = tag(&mut r).ok_or("truncated SWF tag")?;
        let mut t = Reader::new(body);
        let bad = |what: &str| format!("unreadable {what} (tag {code})");
        match code {
            0 => break,
            1 => timeline.frames.push(std::mem::take(&mut frame)),
            9 => lib.background = t.rgb().ok_or_else(|| bad("background"))?,
            26 => frame.commands.push(Command::Place(place(&mut t).ok_or_else(|| bad("placement"))?)),
            28 => frame.commands.push(Command::Remove(t.u16().ok_or_else(|| bad("removal"))?)),
            43 => {
                let label = t.string().ok_or_else(|| bad("label"))?;
                timeline.labels.insert(label, timeline.frames.len() as u16 + 1);
            }
            15 => frame.sounds.push(start_sound(&mut t).ok_or_else(|| bad("sound start"))?),
            56 => {
                for _ in 0..t.u16().ok_or_else(|| bad("exports"))? {
                    let id = t.u16().ok_or_else(|| bad("exports"))?;
                    lib.exports.insert(t.string().ok_or_else(|| bad("exports"))?, id);
                }
            }
            39 => {
                let id = t.u16().ok_or_else(|| bad("sprite"))?;
                let _frames = t.u16().ok_or_else(|| bad("sprite"))?;
                let sprite = tags(lib, t.rest())?;
                lib.characters.insert(id, Character::Sprite(sprite));
            }
            _ => {
                if let Some((id, character)) = definition(code, &mut t) {
                    lib.characters.insert(id, character);
                } else if DEFINITIONS.contains(&code) {
                    return Err(bad("definition"));
                }
            }
        }
    }
    // Une dernière image sans ShowFrame.
    if !frame.commands.is_empty() || !frame.sounds.is_empty() {
        timeline.frames.push(frame);
    }
    Ok(timeline)
}

/// Les balises de définition lues par `definition`.
const DEFINITIONS: [u16; 12] = [2, 22, 32, 46, 35, 36, 75, 11, 33, 37, 34, 14];

/// Une balise : son code et son contenu.
fn tag<'a>(r: &mut Reader<'a>) -> Option<(u16, &'a [u8])> {
    let header = r.u16()?;
    let mut len = u32::from(header & 0x3F);
    if len == 0x3F {
        len = r.u32()?;
    }
    Some((header >> 6, r.bytes(len as usize)?))
}

fn definition(code: u16, t: &mut Reader) -> Option<(Id, Character)> {
    let id = t.u16()?;
    let character = match code {
        2 | 22 | 32 => {
            let version = match code {
                2 => 1,
                22 => 2,
                _ => 3,
            };
            let bounds = t.rect()?;
            Character::Shape(crate::library::Shape { bounds, layers: shape::shape(t, version)? })
        }
        46 => Character::Morph(shape::morph(t)?),
        35 => Character::Bitmap(bitmap::jpeg3(t)?),
        36 => Character::Bitmap(bitmap::lossless2(t)?),
        75 => Character::Font(text::font3(t)?),
        11 | 33 => Character::Text(text::text(t, code == 33)?),
        37 => Character::EditText(text::edit_text(t)?),
        34 => Character::Button(button2(t)?),
        14 => Character::Sound(sound(t)?),
        _ => return None,
    };
    Some((id, character))
}

/// PlaceObject2.
fn place(t: &mut Reader) -> Option<Place> {
    let flags = t.u8()?;
    let depth = t.u16()?;
    let character = if flags & 0x02 != 0 { Some(t.u16()?) } else { None };
    let matrix = if flags & 0x04 != 0 { Some(t.matrix()?) } else { None };
    let color = if flags & 0x08 != 0 { Some(t.color_transform(true)?) } else { None };
    let ratio = if flags & 0x10 != 0 { Some(t.u16()?) } else { None };
    let name = if flags & 0x20 != 0 { Some(t.string()?) } else { None };
    let clip_depth = if flags & 0x40 != 0 { Some(t.u16()?) } else { None };
    // Les événements de clip (0x80) sont du code : réécrit en Rust.
    Some(Place { depth, character, is_move: flags & 0x01 != 0, matrix, color, ratio, name, clip_depth })
}

/// StartSound.
fn start_sound(t: &mut Reader) -> Option<StartSound> {
    let id = t.u16()?;
    let flags = t.u8()?;
    let in_point = if flags & 0x01 != 0 { Some(t.u32()?) } else { None };
    let out_point = if flags & 0x02 != 0 { Some(t.u32()?) } else { None };
    let loops = if flags & 0x04 != 0 { t.u16()? } else { 1 };
    let mut envelope = Vec::new();
    if flags & 0x08 != 0 {
        for _ in 0..t.u8()? {
            let pos = t.u32()?;
            let left = f32::from(t.u16()?) / 32768.0;
            let right = f32::from(t.u16()?) / 32768.0;
            envelope.push((pos, left, right));
        }
    }
    Some(StartSound { id, stop: flags & 0x20 != 0, no_multiple: flags & 0x10 != 0, loops, in_point, out_point, envelope })
}

/// DefineSound, après son identifiant.
fn sound(t: &mut Reader) -> Option<Sound> {
    let format = t.u8()?;
    let codec = match format >> 4 {
        0 | 3 => Codec::Pcm,
        1 => Codec::Adpcm,
        2 => Codec::Mp3,
        4..=6 => Codec::Nellymoser,
        other => Codec::Other(other),
    };
    let rate = match format >> 4 {
        5 => 8000,
        4 => 16000,
        _ => [5512, 11025, 22050, 44100][usize::from((format >> 2) & 3)],
    };
    let samples = t.u32()?;
    Some(Sound { codec, rate, sixteen_bits: format & 2 != 0, stereo: format & 1 != 0, samples, data: t.rest().to_vec() })
}

/// DefineButton2, après son identifiant. Ses actions sont du code : la
/// logique des boutons est réécrite en Rust.
fn button2(t: &mut Reader) -> Option<Button> {
    let _menu = t.u8()?;
    let _actions = t.u16()?;
    let mut records = Vec::new();
    loop {
        let flags = t.u8()?;
        if flags == 0 {
            break;
        }
        // Filtres et modes de fusion (Flash 8) : absents du jeu.
        if flags & 0x30 != 0 {
            return None;
        }
        let character = t.u16()?;
        let depth = t.u16()?;
        let matrix = t.matrix()?;
        let color = t.color_transform(true)?;
        records.push(ButtonRecord { states: flags & 0x0F, character, depth, matrix, color });
    }
    Some(Button { records })
}

#[cfg(test)]
mod tests {
    use crate::library::Character;

    /// Tout le SWF original se lit (s'il est à la racine du dépôt).
    #[test]
    fn original() {
        let Ok((data, _)) = crate::original::load("", "") else {
            eprintln!("SWF original absent : test sauté");
            return;
        };
        let lib = super::load(&data).unwrap();
        let count = |f: fn(&Character) -> bool| lib.characters.values().filter(|c| f(c)).count();
        assert_eq!(count(|c| matches!(c, Character::Shape(_))), 177);
        assert_eq!(count(|c| matches!(c, Character::Sprite(_))), 157);
        assert_eq!(count(|c| matches!(c, Character::Bitmap(_))), 44);
        assert_eq!(count(|c| matches!(c, Character::Sound(_))), 53);
        assert_eq!(count(|c| matches!(c, Character::Font(_))), 9);
        assert_eq!(count(|c| matches!(c, Character::Text(_))), 65);
        assert_eq!(count(|c| matches!(c, Character::Button(_))), 13);
        assert_eq!(lib.root.frames.len(), 18);
        assert_eq!((lib.stage, lib.frame_rate), ((700.0, 400.0), 30.0));
    }

    /// Les couleurs des images transparentes dépassent-elles leur alpha
    /// (sinon elles sont prémultipliées) ? cargo test alpha -- --ignored --nocapture
    #[test]
    #[ignore]
    fn alpha() {
        let (data, _) = crate::original::load("", "").unwrap();
        let lib = super::load(&data).unwrap();
        for (id, c) in &lib.characters {
            let Character::Bitmap(b) = c else { continue };
            let px: Vec<&[u8]> = b.rgba.chunks_exact(4).filter(|p| p[3] < 250).collect();
            if px.is_empty() {
                continue;
            }
            let over = px.iter().filter(|p| p[0].max(p[1]).max(p[2]) > p[3].saturating_add(8)).count();
            eprintln!("image {id} {}x{} : {} pixels transparents, {over} plus clairs que leur alpha", b.width, b.height, px.len());
        }
    }

    /// Écrit la structure des timelines dans Tools/work/structure.txt
    /// (référence pour écrire la logique) : cargo test structure -- --ignored
    #[test]
    #[ignore]
    fn structure() {
        use crate::library::{Command, Timeline};
        use std::fmt::Write;
        let (data, _) = crate::original::load("", "").unwrap();
        let lib = super::load(&data).unwrap();
        let kind = |id: u16| match lib.characters.get(&id) {
            Some(Character::Sprite(_)) => "clip",
            Some(Character::Button(_)) => "bouton",
            Some(Character::EditText(_)) => "champ",
            Some(Character::Text(_)) => "texte",
            Some(Character::Shape(_)) => "forme",
            Some(Character::Morph(_)) => "morph",
            Some(Character::Sound(_)) => "son",
            _ => "?",
        };
        let mut out = String::new();
        let timeline = |out: &mut String, title: String, t: &Timeline| {
            let _ = writeln!(out, "== {title} ({} images)", t.frames.len());
            let mut labels: Vec<_> = t.labels.iter().collect();
            labels.sort_by_key(|l| l.1);
            for (frame, def) in t.frames.iter().enumerate() {
                let mut line = String::new();
                for (name, f) in &labels {
                    if usize::from(**f) == frame + 1 {
                        let _ = write!(line, " [{name}]");
                    }
                }
                for c in &def.commands {
                    match c {
                        Command::Place(p) => {
                            let what = p.character.map_or(String::new(), |id| format!("{}{id}", kind(id)));
                            let name = p.name.as_deref().map_or(String::new(), |n| format!(" \"{n}\""));
                            let mask = p.clip_depth.map_or(String::new(), |d| format!(" masque→{d}"));
                            let _ = write!(line, " d{}{}{what}{name}{mask}", p.depth, if p.is_move { "~" } else { "+" });
                        }
                        Command::Remove(d) => {
                            let _ = write!(line, " -d{d}");
                        }
                    }
                }
                for s in &def.sounds {
                    let _ = write!(line, " son{}{}", s.id, if s.stop { "(stop)" } else { "" });
                }
                if !line.is_empty() {
                    let _ = writeln!(out, "  {}:{line}", frame + 1);
                }
            }
        };
        timeline(&mut out, "racine".into(), &lib.root);
        let mut ids: Vec<_> = lib.characters.keys().copied().collect();
        ids.sort_unstable();
        for id in ids {
            if let Some(Character::Sprite(t)) = lib.characters.get(&id) {
                let export = lib.exports.iter().find(|e| *e.1 == id).map_or(String::new(), |e| format!(" « {} »", e.0));
                timeline(&mut out, format!("clip {id}{export}"), t);
            }
            if let Some(Character::Button(b)) = lib.characters.get(&id) {
                let _ = writeln!(out, "== bouton {id}");
                for r in &b.records {
                    let _ = writeln!(out, "  états {:04b} d{} {}{}", r.states, r.depth, kind(r.character), r.character);
                }
            }
            if let Some(Character::EditText(t)) = lib.characters.get(&id) {
                let _ = writeln!(out, "== champ {id} variable « {} » texte « {} »", t.variable, t.text);
            }
        }
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../Tools/work/structure.txt");
        std::fs::write(path, out).unwrap();
    }
}
