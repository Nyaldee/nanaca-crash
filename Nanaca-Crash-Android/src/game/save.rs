//! La sauvegarde : les clés de CCUserDefault du jeu (mêmes noms), rangées
//! dans Documents\My Games\Nanaca Crash\android_save.json (Windows),
//! ~/.local/share/Nanaca Crash/android_save.json (Linux) ou
//! saves\android_save.json à côté de l'exécutable (version portable), en
//! deux objets : « settings » pour les réglages, « stats » pour le reste
//! (record, statistiques, galerie). Un troisième, « assets », garde le
//! chemin de l'APK donné par le joueur et la marque de celui déjà vérifié.
//! Écrite à chaque changement, comme les SharedPreferences d'Android.

use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::data::json::{self, Json};

/// Le dossier de la sauvegarde, dans My Games (ou ~/.local/share).
const FOLDER: &str = "Nanaca Crash";
const FILE: &str = "android_save.json";

const SETTINGS: &str = "settings";
const STATS: &str = "stats";
/// L'APK original : chemin donné par le joueur (fichier ou dossier) et
/// marque (taille et date) de celui déjà vérifié.
const ASSETS: &str = "assets";
const PATH: &str = "path";
const CHECKED: &str = "checked";
/// Les clés des réglages ; les autres vont dans les statistiques.
const SETTING_KEYS: &[&str] = &["effect_volume", "sound_volume", "skin_selected", "speedmeter_state", "displaysetting"];

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Number(f64),
    Bool(bool),
    Text(String),
}

pub struct Save {
    values: BTreeMap<String, Value>,
    assets: BTreeMap<String, Value>,
    path: Option<PathBuf>,
}

impl Save {
    /// Lit la sauvegarde (vide si absente ou illisible).
    pub fn load() -> Save {
        let path = crate::platform::saves(FOLDER).map(|d| d.join(FILE));
        let (mut values, mut assets) = (BTreeMap::new(), BTreeMap::new());
        if let Some(Json::Object(root)) = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| json::parse(t.trim_start_matches('\u{feff}'))) {
            for group in [SETTINGS, STATS, ASSETS] {
                if let Some(Json::Object(o)) = root.get(group) {
                    for (k, v) in o {
                        let v = match v {
                            Json::Number(n) => Value::Number(*n),
                            Json::Bool(b) => Value::Bool(*b),
                            Json::Str(s) => Value::Text(s.clone()),
                            _ => continue,
                        };
                        if group == ASSETS { &mut assets } else { &mut values }.insert(k.clone(), v);
                    }
                }
            }
        }
        Save { values, assets, path }
    }

    /// La sauvegarde lue, gardée en mémoire seulement (tests).
    pub fn memory() -> Save {
        Save { path: None, ..Save::load() }
    }

    /// Le fichier de la sauvegarde, pour le joueur.
    pub fn file(&self) -> String {
        self.path.as_ref().map_or_else(String::new, |p| p.display().to_string())
    }

    /// Le chemin de l'APK donné par le joueur, et la marque retenue.
    pub fn assets(&self) -> (&str, &str) {
        let get = |k| match self.assets.get(k) {
            Some(Value::Text(s)) => s.as_str(),
            _ => "",
        };
        (get(PATH), get(CHECKED))
    }

    /// Retient la marque de l'APK vérifié ; « path » reste présent (vide)
    /// pour que le joueur le trouve.
    pub fn set_assets_checked(&mut self, stamp: &str) {
        if self.assets.contains_key(PATH) && self.assets().1 == stamp {
            return;
        }
        let path = self.assets().0.to_string();
        self.assets.insert(PATH.to_string(), Value::Text(path));
        self.assets.insert(CHECKED.to_string(), Value::Text(stamp.to_string()));
        self.write();
    }

    pub fn float(&self, key: &str, default: f32) -> f32 {
        match self.values.get(key) {
            Some(Value::Number(n)) => *n as f32,
            _ => default,
        }
    }

    pub fn int(&self, key: &str, default: i32) -> i32 {
        match self.values.get(key) {
            Some(Value::Number(n)) => *n as i32,
            _ => default,
        }
    }

    pub fn bool(&self, key: &str, default: bool) -> bool {
        match self.values.get(key) {
            Some(Value::Bool(b)) => *b,
            _ => default,
        }
    }

    pub fn string(&self, key: &str, default: &str) -> String {
        match self.values.get(key) {
            Some(Value::Text(s)) => s.clone(),
            _ => default.to_string(),
        }
    }

    pub fn set_float(&mut self, key: &str, v: f32) {
        self.set(key, Value::Number(f64::from(v)));
    }

    pub fn set_int(&mut self, key: &str, v: i32) {
        self.set(key, Value::Number(f64::from(v)));
    }

    pub fn set_bool(&mut self, key: &str, v: bool) {
        self.set(key, Value::Bool(v));
    }

    pub fn set_string(&mut self, key: &str, v: &str) {
        self.set(key, Value::Text(v.to_string()));
    }

    fn set(&mut self, key: &str, v: Value) {
        if self.values.get(key) == Some(&v) {
            return;
        }
        self.values.insert(key.to_string(), v);
        self.write();
    }

    fn write(&self) {
        let Some(path) = &self.path else { return };
        let mut out = String::from("{\n");
        for group in [ASSETS, SETTINGS, STATS] {
            let entries: Vec<_> = match group {
                ASSETS => self.assets.iter().collect(),
                _ => self.values.iter().filter(|(k, _)| SETTING_KEYS.contains(&k.as_str()) == (group == SETTINGS)).collect(),
            };
            if group == ASSETS && entries.is_empty() {
                continue;
            }
            out.push_str(&format!("  \"{group}\": {{"));
            for (j, (k, v)) in entries.iter().enumerate() {
                let value = match v {
                    Value::Number(n) => {
                        if n.fract() == 0.0 && n.abs() < 1e15 { format!("{}", *n as i64) } else { format!("{n}") }
                    }
                    Value::Bool(b) => b.to_string(),
                    Value::Text(s) => quote(s),
                };
                out.push_str(&format!("{}\n    {}: {value}", if j > 0 { "," } else { "" }, quote(k)));
            }
            out.push_str(if entries.is_empty() { "}" } else { "\n  }" });
            out.push_str(if group != STATS { ",\n" } else { "\n" });
        }
        out.push_str("}\n");
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // Écrit à côté puis remplace : jamais de fichier à moitié écrit.
        let tmp = path.with_extension("json.tmp");
        if std::fs::write(&tmp, out).is_ok() {
            let _ = std::fs::rename(&tmp, path);
        }
    }
}

fn quote(s: &str) -> String {
    let mut q = String::from("\"");
    for c in s.chars() {
        match c {
            '"' => q.push_str("\\\""),
            '\\' => q.push_str("\\\\"),
            '\n' => q.push_str("\\n"),
            c if (c as u32) < 0x20 => q.push_str(&format!("\\u{:04x}", c as u32)),
            c => q.push(c),
        }
    }
    q.push('"');
    q
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../Tools/work").join(format!("save-test-{}", std::process::id()));
        let path = dir.join(FILE);
        let mut s = Save { values: BTreeMap::new(), assets: BTreeMap::new(), path: Some(path.clone()) };
        s.set_assets_checked("1:2");
        s.set_float("best_record", 1234.5);
        s.set_float("effect_volume", 0.5);
        s.set_bool("is_first_game", false);
        s.set_string("achievement_cache", "a\"b");
        let text = std::fs::read_to_string(&path).unwrap();
        let root = json::parse(&text).unwrap();
        assert!(matches!(root.get(SETTINGS).get("effect_volume"), Json::Number(n) if *n == 0.5));
        assert!(matches!(root.get(STATS).get("best_record"), Json::Number(n) if *n == 1234.5));
        assert_eq!(root.get(STATS).str("achievement_cache"), "a\"b");
        assert_eq!(root.get(ASSETS).str(CHECKED), "1:2");
        assert_eq!(root.get(ASSETS).str(PATH), "");
        let _ = std::fs::remove_dir_all(dir);
    }
}
