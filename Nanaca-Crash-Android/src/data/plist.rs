//! Les plist d'Apple (XML) et les feuilles de sprites TexturePacker
//! (format 2) qu'elles décrivent.

use std::collections::HashMap;

use super::xml::{self, Element};

#[derive(Debug, Clone)]
pub enum Plist {
    Dict(HashMap<String, Plist>),
    /// Tableaux et nombres : présents, mais jamais lus par le jeu.
    Array,
    Str(String),
    Number,
    Bool(bool),
}

impl Plist {
    pub fn get(&self, key: &str) -> Option<&Plist> {
        match self {
            Plist::Dict(d) => d.get(key),
            _ => None,
        }
    }

    pub fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Some(Plist::Str(s)) => s,
            _ => "",
        }
    }
}

pub fn parse(text: &str) -> Option<Plist> {
    let root = xml::parse(text)?;
    value(root.children.first()?)
}

fn value(e: &Element) -> Option<Plist> {
    Some(match e.name.as_str() {
        "dict" => {
            let mut d = HashMap::new();
            let mut key = None;
            for c in &e.children {
                if c.name == "key" {
                    key = Some(c.text.clone());
                } else if let Some(k) = key.take() {
                    d.insert(k, value(c)?);
                }
            }
            Plist::Dict(d)
        }
        "array" => Plist::Array,
        "integer" | "real" => Plist::Number,
        "true" => Plist::Bool(true),
        "false" => Plist::Bool(false),
        _ => Plist::Str(e.text.clone()),
    })
}

/// Une image dans sa feuille, en pixels de la texture.
#[derive(Debug, Clone)]
pub struct Frame {
    /// La texture de la feuille (nom de fichier, dossier compris).
    pub texture: String,
    /// Rectangle dans la texture (largeur, hauteur avant rotation).
    pub rect: [f32; 4],
    /// Tournée de 90° dans la feuille (sens horaire).
    pub rotated: bool,
    /// Décalage du centre de la partie gardée par rapport au centre de
    /// l'image d'origine (y vers le haut).
    pub offset: (f32, f32),
    /// Taille de l'image d'origine (avant rognage).
    pub source: (f32, f32),
}

/// Les images d'une feuille ; `dir` est le dossier de la feuille (« hd/ »).
pub fn frames(text: &str, dir: &str) -> Vec<(String, Frame)> {
    let Some(root) = parse(text) else { return Vec::new() };
    let texture = format!("{dir}{}", root.get("metadata").map_or("", |m| m.str("textureFileName")));
    let Some(Plist::Dict(frames)) = root.get("frames") else { return Vec::new() };
    frames
        .iter()
        .map(|(name, f)| {
            let r = numbers(f.str("frame"));
            let o = numbers(f.str("offset"));
            let s = numbers(f.str("sourceSize"));
            let rotated = matches!(f.get("rotated"), Some(Plist::Bool(true)));
            let frame = Frame {
                texture: texture.clone(),
                rect: [r.first().copied().unwrap_or(0.0), r.get(1).copied().unwrap_or(0.0), r.get(2).copied().unwrap_or(0.0), r.get(3).copied().unwrap_or(0.0)],
                rotated,
                offset: (o.first().copied().unwrap_or(0.0), o.get(1).copied().unwrap_or(0.0)),
                source: (s.first().copied().unwrap_or(0.0), s.get(1).copied().unwrap_or(0.0)),
            };
            (name.clone(), frame)
        })
        .collect()
}

/// Les nombres d'une chaîne « {{1,2},{3,4}} ».
fn numbers(s: &str) -> Vec<f32> {
    s.split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames() {
        let text = r#"<?xml version="1.0"?><plist version="1.0"><dict><key>frames</key><dict>
            <key>a.png</key><dict><key>frame</key><string>{{1654,1006},{240,70}}</string>
            <key>offset</key><string>{-1,2}</string><key>rotated</key><true/>
            <key>sourceSize</key><string>{242,74}</string></dict></dict>
            <key>metadata</key><dict><key>textureFileName</key><string>s.png</string></dict></dict></plist>"#;
        let f = super::frames(text, "hd/");
        assert_eq!(f.len(), 1);
        assert_eq!(f[0].1.texture, "hd/s.png");
        assert_eq!(f[0].1.rect, [1654.0, 1006.0, 240.0, 70.0]);
        assert!(f[0].1.rotated);
        assert_eq!(f[0].1.offset, (-1.0, 2.0));
    }
}
