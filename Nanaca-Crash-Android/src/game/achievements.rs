//! Les succès, qui ouvrent les éléments de la galerie. L'original les
//! confiait à Google Play Jeux (GalleryContents → GameCenter) et gardait la
//! liste des succès obtenus dans « achievement_cache » (« 0:id,id,… »). Ici,
//! le joueur est toujours « connecté » : les succès sont notés directement
//! dans ce même cache ; un succès progressif est obtenu quand sa valeur
//! atteint son objectif.

use super::Game;
use crate::data::xml;

/// Un élément de la galerie (gallery.xml).
#[derive(Clone)]
pub struct Content {
    pub id: i32,
    pub kind: i32,
    pub icon: String,
    pub title: String,
    pub earned_text: String,
    pub content_title: String,
    pub hint: String,
    pub thumb: String,
    pub gplus_id: String,
}

/// Les éléments de la galerie, dans la langue du jeu.
pub fn load(assets: &crate::assets::Assets, lang: &str) -> Vec<Content> {
    let file = match lang {
        "ja" => "gallery_ja.xml",
        "zh-tw" => "gallery_zh-tw.xml",
        _ => "gallery.xml",
    };
    let Some(root) = assets.apk.text(file).and_then(|t| xml::parse(&t)) else { return Vec::new() };
    root.children_named("content")
        .map(|c| {
            let text = |name: &str| c.child(name).map_or(String::new(), |e| e.text.trim().to_string());
            Content {
                id: text("id").parse().unwrap_or(-1),
                kind: text("type").parse().unwrap_or(0),
                icon: text("icon"),
                title: text("title"),
                earned_text: text("earned_text"),
                content_title: text("content_title"),
                hint: text("hint"),
                thumb: text("thumb"),
                gplus_id: text("gplus_id"),
            }
        })
        .collect()
}

const CACHE: &str = "achievement_cache";

impl Game {
    /// Les identifiants des succès obtenus.
    fn earned(&self) -> Vec<String> {
        let cache = self.save.string(CACHE, "");
        cache.strip_prefix("0:").map(|l| l.split(',').filter(|s| !s.is_empty()).map(String::from).collect()).unwrap_or_default()
    }

    /// Ce succès (numéro de galerie) est-il obtenu ?
    pub fn achieved(&self, index: i32) -> bool {
        let Some(c) = self.gallery.iter().find(|c| c.id == index) else { return false };
        c.gplus_id.len() > 1 && self.earned().contains(&c.gplus_id)
    }

    /// GalleryContents::unlockAchievement.
    pub fn unlock(&mut self, index: i32) {
        let Some(id) = self.gallery.iter().find(|c| c.id == index).map(|c| c.gplus_id.clone()) else { return };
        let mut earned = self.earned();
        if id.len() > 1 && !earned.contains(&id) {
            earned.push(id);
            self.save.set_string(CACHE, &format!("0:{}", earned.join(",")));
        }
    }

    /// GalleryContents::updateAchievement : un succès progressif, obtenu
    /// quand `value` atteint `goal`.
    pub fn update_achievement(&mut self, index: i32, steps: i32, value: f32, goal: f32) {
        if steps >= 1 && value >= goal {
            self.unlock(index);
        }
    }
}
