//! La sauvegarde : les deux SharedObject du jeu (« nanaca-crash/save » :
//! record ; « nanaca-crash/common » : réglages), et dans « assets » le
//! chemin du SWF donné par le joueur et la marque de celui déjà vérifié,
//! réunis dans Documents\My Games\Nanaca Crash\flash_save.json (Windows),
//! ~/.local/share/Nanaca Crash/flash_save.json (Linux) ou
//! saves\flash_save.json à côté de l'exécutable (version portable).
//!
//! ```json
//! {
//!   "assets": { "checked": "<taille>:<date>", "path": "" },
//!   "settings": { "qualitysetting": "MEDIUM", "displaysetting": "window", … },
//!   "stats": { "dataversion": 1.08, "datarecord": 137.29 }
//! }
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

/// Les réglages (le SharedObject « common » du jeu).
pub const SETTINGS: &str = "settings";
/// Le record (le SharedObject « save » du jeu).
pub const STATS: &str = "stats";
/// Le SWF original : chemin donné par le joueur (fichier ou dossier) et
/// marque (taille et date) de celui déjà vérifié.
const ASSETS: &str = "assets";
const PATH: &str = "path";
const CHECKED: &str = "checked";

/// Le dossier de la sauvegarde, dans My Games (ou ~/.local/share).
const FOLDER: &str = "Nanaca Crash";
const FILE: &str = "flash_save.json";

#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Number(f64),
    Text(String),
}

pub struct Save {
    /// Le nom du SharedObject, puis ses valeurs.
    data: BTreeMap<String, BTreeMap<String, Value>>,
    path: Option<PathBuf>,
}

impl Save {
    pub fn load() -> Save {
        let path = crate::platform::saves(FOLDER).map(|d| d.join(FILE));
        let data = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| parse(t.trim_start_matches('\u{feff}'))).unwrap_or_default();
        Save { data, path }
    }

    /// Le fichier de la sauvegarde, pour le joueur.
    pub fn file(&self) -> String {
        self.path.as_ref().map_or_else(String::new, |p| p.display().to_string())
    }

    /// Le chemin du SWF donné par le joueur, et la marque retenue.
    pub fn assets(&self) -> (&str, &str) {
        (self.text(ASSETS, PATH).unwrap_or(""), self.text(ASSETS, CHECKED).unwrap_or(""))
    }

    /// Retient la marque du SWF vérifié et l'écrit ; « path » reste présent
    /// (vide) pour que le joueur le trouve.
    pub fn set_assets_checked(&mut self, stamp: &str) {
        if self.get(ASSETS, PATH).is_some() && self.assets().1 == stamp {
            return;
        }
        let path = self.assets().0.to_string();
        self.set(ASSETS, PATH, Value::Text(path));
        self.set(ASSETS, CHECKED, Value::Text(stamp.to_string()));
        self.flush();
    }

    pub fn get(&self, object: &str, key: &str) -> Option<&Value> {
        self.data.get(object)?.get(key)
    }

    pub fn number(&self, object: &str, key: &str) -> Option<f64> {
        match self.get(object, key)? {
            Value::Number(n) => Some(*n),
            Value::Text(_) => None,
        }
    }

    pub fn text(&self, object: &str, key: &str) -> Option<&str> {
        match self.get(object, key)? {
            Value::Text(t) => Some(t),
            Value::Number(_) => None,
        }
    }

    pub fn set(&mut self, object: &str, key: &str, value: Value) {
        self.data.entry(object.to_string()).or_default().insert(key.to_string(), value);
    }

    /// flush : écrit la sauvegarde sur le disque.
    pub fn flush(&self) {
        let Some(path) = &self.path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let _ = std::fs::write(path, write(&self.data));
    }
}

/// La sauvegarde en JSON, lisible.
fn write(data: &BTreeMap<String, BTreeMap<String, Value>>) -> String {
    let mut out = String::from("{\n");
    for (i, (name, values)) in data.iter().enumerate() {
        out.push_str(&format!("  {}: {{\n", quote(name)));
        for (j, (k, v)) in values.iter().enumerate() {
            let v = match v {
                // Les nombres gardent toute leur précision ; JSON n'a ni NaN
                // ni infini.
                Value::Number(n) if n.is_finite() => format!("{n:?}"),
                Value::Number(_) => "null".into(),
                Value::Text(t) => quote(t),
            };
            out.push_str(&format!("    {}: {v}{}\n", quote(k), if j + 1 < values.len() { "," } else { "" }));
        }
        out.push_str(&format!("  }}{}\n", if i + 1 < data.len() { "," } else { "" }));
    }
    out.push_str("}\n");
    out
}

fn quote(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Lit la sauvegarde : un objet d'objets de nombres et de textes (le reste
/// est ignoré) ; None si le fichier n'est pas du JSON valide.
fn parse(text: &str) -> Option<BTreeMap<String, BTreeMap<String, Value>>> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    let mut data = BTreeMap::new();
    p.object(|p, name| {
        let mut values = BTreeMap::new();
        if p.peek() == Some(b'{') {
            p.object(|p, key| {
                if let Some(v) = p.value()? {
                    values.insert(key, v);
                }
                Some(())
            })?;
        } else {
            p.value()?;
        }
        data.insert(name, values);
        Some(())
    })?;
    Some(data)
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn peek(&mut self) -> Option<u8> {
        while self.s.get(self.i).is_some_and(u8::is_ascii_whitespace) {
            self.i += 1;
        }
        self.s.get(self.i).copied()
    }

    fn expect(&mut self, c: u8) -> Option<()> {
        (self.peek()? == c).then(|| self.i += 1)
    }

    /// Un objet : chaque clé et sa valeur passent à `f`.
    fn object(&mut self, mut f: impl FnMut(&mut Self, String) -> Option<()>) -> Option<()> {
        self.expect(b'{')?;
        if self.peek()? == b'}' {
            self.i += 1;
            return Some(());
        }
        loop {
            let key = self.string()?;
            self.expect(b':')?;
            f(self, key)?;
            match self.peek()? {
                b',' => self.i += 1,
                b'}' => {
                    self.i += 1;
                    return Some(());
                }
                _ => return None,
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        self.expect(b'"')?;
        let mut out = Vec::new();
        loop {
            let c = *self.s.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => return String::from_utf8(out).ok(),
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'u' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?)?;
                            out.extend_from_slice(c.to_string().as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                c => out.push(c),
            }
        }
    }

    /// Une valeur : nombre ou texte gardés, le reste lu et ignoré.
    fn value(&mut self) -> Option<Option<Value>> {
        match self.peek()? {
            b'"' => Some(Some(Value::Text(self.string()?))),
            b'{' => self.object(|p, _| p.value().map(|_| ())).map(|()| None),
            b't' | b'f' | b'n' => {
                while self.s.get(self.i).is_some_and(u8::is_ascii_alphabetic) {
                    self.i += 1;
                }
                Some(None)
            }
            _ => {
                let start = self.i;
                while self.s.get(self.i).is_some_and(|c| c.is_ascii_digit() || b"+-.eE".contains(c)) {
                    self.i += 1;
                }
                let n: f64 = std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()?;
                Some(Some(Value::Number(n)))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let mut data = BTreeMap::new();
        let mut common = BTreeMap::new();
        common.insert("qualitysetting".to_string(), Value::Text("HIGH \"Ω\"".into()));
        common.insert("soundsetting".to_string(), Value::Number(100.0));
        data.insert(SETTINGS.to_string(), common);
        let mut save = BTreeMap::new();
        save.insert("datarecord".to_string(), Value::Number(137.29123456789012));
        data.insert(STATS.to_string(), save);
        assert_eq!(parse(&write(&data)), Some(data));
        assert_eq!(parse("pas du json"), None);
    }
}
