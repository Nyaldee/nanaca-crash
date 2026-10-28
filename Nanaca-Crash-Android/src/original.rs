//! L'APK original du joueur : cherché au chemin donné dans la sauvegarde,
//! puis à côté de l'exécutable (sous n'importe quel nom), et reconnu par son
//! CRC32 pour ne jouer que la bonne version ; une fois vérifié, il est
//! reconnu ensuite à sa taille et à sa date. Ses fichiers sont lus
//! directement dans le zip.

use std::collections::HashMap;
use std::path::PathBuf;

const CRC32: u32 = 0xBCBA_B1C3;

/// Un fichier du zip : où il commence, sa taille compressée et réelle, et
/// sa méthode (0 : stocké, 8 : deflate).
struct Entry {
    offset: usize,
    packed: usize,
    size: usize,
    method: u16,
}

pub struct Apk {
    data: Vec<u8>,
    entries: HashMap<String, Entry>,
}

impl Apk {
    /// L'APK et sa marque (taille et date) à retenir pour ne plus le
    /// vérifier, ou un message pour le joueur. `path` est le chemin donné
    /// par le joueur (fichier ou dossier), `known` la marque retenue.
    pub fn load(path: &str, known: &str) -> Result<(Apk, String), String> {
        const MISSING: &str = "The original APK of NANACA†CRASH!! 1.2.0 was not found.\n\nPut it next to this executable.";
        let candidates = find(path);
        if candidates.is_empty() {
            return Err(MISSING.to_string());
        }
        for path in &candidates {
            let Ok(data) = std::fs::read(path) else { continue };
            let stamp = stamp(path);
            if stamp == known || crc32(&data) == CRC32 {
                return Apk::open(data).map(|apk| (apk, stamp)).map_err(|e| format!("{}: {e}", path.display()));
            }
        }
        Err(format!("No APK found is the expected version (CRC32 {CRC32:08X}).\n\nThis port needs the original NANACA†CRASH!! 1.2.0."))
    }

    /// Lit le répertoire central du zip.
    fn open(data: Vec<u8>) -> Result<Apk, String> {
        let u16_at = |p: usize| data.get(p..p + 2).map_or(0, |b| u16::from_le_bytes([b[0], b[1]]));
        let u32_at = |p: usize| data.get(p..p + 4).map_or(0, |b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]));
        // La fin du répertoire, cherchée depuis la fin (commentaire possible).
        let end = (0..data.len().saturating_sub(21)).rev().take(0x10000 + 22).find(|&p| u32_at(p) == 0x0605_4B50).ok_or("not a zip file")?;
        let count = usize::from(u16_at(end + 10));
        let mut p = u32_at(end + 16) as usize;
        let mut entries = HashMap::with_capacity(count);
        for _ in 0..count {
            if u32_at(p) != 0x0201_4B50 {
                return Err("damaged zip directory".to_string());
            }
            let method = u16_at(p + 10);
            let packed = u32_at(p + 20) as usize;
            let size = u32_at(p + 24) as usize;
            let (name_len, extra_len, comment_len) = (usize::from(u16_at(p + 28)), usize::from(u16_at(p + 30)), usize::from(u16_at(p + 32)));
            let local = u32_at(p + 42) as usize;
            let name = String::from_utf8_lossy(data.get(p + 46..p + 46 + name_len).unwrap_or_default()).into_owned();
            // Les données suivent l'en-tête local, de longueurs propres.
            let offset = local + 30 + usize::from(u16_at(local + 26)) + usize::from(u16_at(local + 28));
            entries.insert(name, Entry { offset, packed, size, method });
            p += 46 + name_len + extra_len + comment_len;
        }
        Ok(Apk { data, entries })
    }

    pub fn contains(&self, name: &str) -> bool {
        self.entries.contains_key(name)
    }

    /// Le contenu d'un fichier (chemin complet dans le zip).
    pub fn read(&self, name: &str) -> Option<Vec<u8>> {
        let e = self.entries.get(name)?;
        let raw = self.data.get(e.offset..e.offset + e.packed)?;
        match e.method {
            0 => Some(raw.to_vec()),
            8 => miniz_oxide::inflate::decompress_to_vec_with_limit(raw, e.size).ok(),
            _ => None,
        }
    }

    /// Un fichier des ressources du jeu (dossier assets).
    pub fn asset(&self, name: &str) -> Option<Vec<u8>> {
        self.read(&format!("assets/{name}"))
    }

    /// Un fichier texte des ressources.
    pub fn text(&self, name: &str) -> Option<String> {
        self.asset(name).map(|b| String::from_utf8_lossy(&b).into_owned())
    }
}

/// La marque d'un fichier : sa taille et sa date de modification.
fn stamp(path: &PathBuf) -> String {
    let meta = std::fs::metadata(path).ok();
    let modified = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    format!("{}:{modified}", meta.map_or(0, |m| m.len()))
}

/// Les APK : celui donné par le joueur (ou ceux de son dossier ; un chemin
/// relatif part de l'exécutable), puis ceux à côté de l'exécutable ; en
/// debug, aussi à la racine du dépôt.
fn find(path: &str) -> Vec<PathBuf> {
    let exe = crate::platform::exe_dir();
    let (mut files, mut dirs) = (Vec::new(), Vec::new());
    if !path.is_empty() {
        let given = exe.as_ref().map_or_else(|| PathBuf::from(path), |e| e.join(path));
        if given.is_dir() { dirs.push(given) } else { files.push(given) }
    }
    dirs.extend(exe);
    if cfg!(debug_assertions) {
        dirs.push(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(".."));
    }
    files.extend(
        dirs.iter()
            .filter_map(|d| std::fs::read_dir(d).ok())
            .flatten()
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("apk"))),
    );
    files.retain(|p| p.is_file());
    files.dedup();
    files
}

pub fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, t) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        *t = c;
    }
    !data.iter().fold(!0u32, |c, &b| table[((c ^ u32::from(b)) & 0xFF) as usize] ^ (c >> 8))
}

#[cfg(test)]
mod tests {
    #[test]
    fn crc32() {
        assert_eq!(super::crc32(b"123456789"), 0xCBF4_3926);
    }

    /// L'APK du dépôt (s'il est là) s'ouvre et ses fichiers se lisent.
    #[test]
    fn apk() {
        let Ok((apk, _)) = super::Apk::load("", "") else { return };
        assert!(apk.text("skin.xml").is_some_and(|s| s.contains("way_to_school")));
        assert!(apk.asset("hd/gamesheet.png").is_some_and(|b| b.starts_with(b"\x89PNG")));
    }
}
