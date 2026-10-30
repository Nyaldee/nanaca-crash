//! Les ressources du jeu, lues dans l'APK à la demande : images des
//! feuilles de sprites, polices, textes. Comme le jeu sur un écran d'au
//! moins 720 pixels de haut, le dossier « hd » (images doublées) d'abord.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use crate::data::{fnt, plist};
use crate::original::Apk;

/// Points par pixel des images « hd » (facteur d'échelle du contenu).
pub const CONTENT_SCALE: f32 = 2.0;
/// Dossier des images.
const RESOURCES: &str = "hd/";

pub use fnt::Font;
pub use plist::Frame;

pub struct Assets {
    pub apk: Apk,
    frames: RefCell<HashMap<String, Rc<Frame>>>,
    fonts: RefCell<HashMap<String, Option<Rc<Font>>>>,
    sizes: RefCell<HashMap<String, Option<(f32, f32)>>>,
    pub(crate) armatures: RefCell<HashMap<String, Rc<crate::armature::ArmatureData>>>,
    /// Textures sans lissage (chemins résolus).
    aliased: RefCell<HashSet<String>>,
    /// Les feuilles déjà lues.
    loaded: RefCell<HashSet<String>>,
}

impl Assets {
    pub fn new(apk: Apk) -> Assets {
        Assets {
            apk,
            frames: RefCell::default(),
            fonts: RefCell::default(),
            sizes: RefCell::default(),
            armatures: RefCell::default(),
            aliased: RefCell::default(),
            loaded: RefCell::default(),
        }
    }

    /// setAliasTexParameters : la texture n'est plus lissée.
    pub fn set_alias(&self, path: &str) {
        self.aliased.borrow_mut().insert(path.to_string());
    }

    pub fn is_aliased(&self, path: &str) -> bool {
        self.aliased.borrow().contains(path)
    }

    /// Le chemin d'un fichier dans les ressources : d'abord « hd/ », puis
    /// la racine (CCFileUtils).
    pub fn resolve(&self, name: &str) -> Option<String> {
        [format!("{RESOURCES}{name}"), name.to_string()].into_iter().find(|p| self.apk.contains(&format!("assets/{p}")))
    }

    pub fn bytes(&self, path: &str) -> Option<Vec<u8>> {
        self.apk.asset(path)
    }

    /// Ajoute les images d'une feuille (CCSpriteFrameCache::addSpriteFramesWithFile).
    pub fn add_sprite_frames(&self, plist_name: &str) {
        self.add_sprite_frames_with(plist_name, None);
    }

    /// addSpriteFramesWithFile(plist, texture) : l'image donnée remplace
    /// celle des métadonnées (celle des feuilles des personnages,
    /// « merge-hd.png », n'existe pas).
    pub fn add_sprite_frames_with(&self, plist_name: &str, texture: Option<&str>) {
        // Une feuille déjà chargée ne l'est pas deux fois (m_pLoadedFileNames).
        if !self.loaded.borrow_mut().insert(plist_name.to_string()) {
            return;
        }
        let Some(path) = self.resolve(plist_name) else { return };
        let Some(text) = self.apk.text(&path) else { return };
        let dir = path.rfind('/').map_or("", |i| &path[..=i]);
        let texture = texture.and_then(|t| self.resolve(t));
        let mut frames = self.frames.borrow_mut();
        for (name, mut frame) in plist::frames(&text, dir) {
            if let Some(t) = &texture {
                frame.texture.clone_from(t);
            }
            frames.entry(name).or_insert_with(|| Rc::new(frame));
        }
    }

    /// removeSpriteFrames et CCArmatureDataManager::purge : plus aucune
    /// image ni armature en cache.
    pub fn purge(&self) {
        self.frames.borrow_mut().clear();
        self.loaded.borrow_mut().clear();
        self.armatures.borrow_mut().clear();
    }

    /// removeSpriteFrameByName : l'image, et l'oubli des feuilles chargées
    /// (elles seront relues à la prochaine demande).
    pub fn remove_frame(&self, name: &str) {
        self.frames.borrow_mut().remove(name);
        self.loaded.borrow_mut().clear();
    }

    pub fn frame(&self, name: &str) -> Option<Rc<Frame>> {
        self.frames.borrow().get(name).cloned()
    }

    /// Une police BMFont (son fichier .fnt).
    pub fn font(&self, name: &str) -> Option<Rc<Font>> {
        self.fonts
            .borrow_mut()
            .entry(name.to_string())
            .or_insert_with(|| {
                let path = self.resolve(name)?;
                let dir = path.rfind('/').map_or("", |i| &path[..=i]);
                Some(Rc::new(fnt::parse(&self.apk.text(&path)?, dir)))
            })
            .clone()
    }

    /// La taille en pixels d'une image (chemin résolu).
    pub fn image_size(&self, path: &str) -> Option<(f32, f32)> {
        *self.sizes.borrow_mut().entry(path.to_string()).or_insert_with(|| {
            let bytes = self.apk.asset(path)?;
            let (w, h) = image::ImageReader::new(std::io::Cursor::new(bytes)).with_guessed_format().ok()?.into_dimensions().ok()?;
            Some((w as f32, h as f32))
        })
    }
}
