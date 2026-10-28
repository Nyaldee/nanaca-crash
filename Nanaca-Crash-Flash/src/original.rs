//! Le SWF original du joueur : cherché au chemin donné dans la sauvegarde,
//! puis à côté de l'exécutable (sous n'importe quel nom), et reconnu par son
//! CRC32 pour ne jouer que la bonne version. Une fois vérifié, il est
//! reconnu ensuite à sa taille et à sa date.

use std::path::PathBuf;

const CRC32: u32 = 0x53A6_5665;

/// Les octets du SWF et sa marque (taille et date) à retenir pour ne plus
/// le vérifier, ou un message pour le joueur. `path` est le chemin donné
/// par le joueur (fichier ou dossier), `known` la marque retenue.
pub fn load(path: &str, known: &str) -> Result<(Vec<u8>, String), String> {
    let candidates = find(path);
    if candidates.is_empty() {
        return Err("The original SWF of NANACA†CRASH!! 1.10 was not found.\n\nPut it next to this executable.".to_string());
    }
    for path in &candidates {
        let Ok(data) = std::fs::read(path) else { continue };
        let stamp = stamp(path);
        if stamp == known || crc32(&data) == CRC32 {
            return Ok((data, stamp));
        }
    }
    Err(format!("No SWF found is the expected version (CRC32 {CRC32:08X}).\n\nThis port needs the original NANACA†CRASH!! 1.10."))
}

/// La marque d'un fichier : sa taille et sa date de modification.
fn stamp(path: &PathBuf) -> String {
    let meta = std::fs::metadata(path).ok();
    let modified = meta.as_ref().and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs());
    format!("{}:{modified}", meta.map_or(0, |m| m.len()))
}

/// Les SWF : celui donné par le joueur (ou ceux de son dossier ; un chemin
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
            .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("swf"))),
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
}
