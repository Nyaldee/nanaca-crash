//! Le chargement du titre (ResourceLoader) : les armatures des
//! personnages et la feuille de leur skin, puis les sons, un par image ;
//! l'avancement (moitié armatures, moitié sons) est donné au titre.

use super::{Game, Logic, title};

/// Les armatures des personnages.
const ARMATURES: [&str; 9] = [
    "nanaca.ExportJson",
    "miki.ExportJson",
    "touko.ExportJson",
    "kiri.ExportJson",
    "misato.ExportJson",
    "youko.ExportJson",
    "youko_block.ExportJson",
    "sakuraba.ExportJson",
    "tomoki.ExportJson",
];

/// Les sons préchargés, dans l'ordre d'origine.
const EFFECTS: [&str; 55] = [
    "seogg/se01.ogg",
    "seogg/se02.ogg",
    "seogg/se03.ogg",
    "seogg/se04.ogg",
    "seogg/se05.ogg",
    "seogg/se06.ogg",
    "seogg/se07.ogg",
    "seogg/se08.ogg",
    "seogg/se10.ogg",
    "seogg/se11.ogg",
    "seogg/se12.ogg",
    "seogg/se13.ogg",
    "seogg/se14.ogg",
    "seogg/se15.ogg",
    "seogg/se16.ogg",
    "seogg/se17.ogg",
    "seogg/se18.ogg",
    "seogg/se19.ogg",
    "seogg/se_touko_sp.ogg",
    "seogg/se131317.ogg",
    "seogg/se141413.ogg",
    "seogg/se09x3.ogg",
    "voiceogg/kiri01.ogg",
    "voiceogg/kiri02.ogg",
    "voiceogg/kiri03.ogg",
    "voiceogg/kiri04.ogg",
    "voiceogg/kiri05.ogg",
    "voiceogg/miki01.ogg",
    "voiceogg/miki02.ogg",
    "voiceogg/miki03.ogg",
    "voiceogg/miki04.ogg",
    "voiceogg/miki05.ogg",
    "voiceogg/misato01.ogg",
    "voiceogg/misato02.ogg",
    "voiceogg/misato03.ogg",
    "voiceogg/nanaca01.ogg",
    "voiceogg/nanaca02.ogg",
    "voiceogg/nanaca03.ogg",
    "voiceogg/nanaca04.ogg",
    "voiceogg/nanaca05.ogg",
    "voiceogg/sakuraba01.ogg",
    "voiceogg/sakuraba02.ogg",
    "voiceogg/tomoki01.ogg",
    "voiceogg/tomoki02.ogg",
    "voiceogg/touko01.ogg",
    "voiceogg/touko02.ogg",
    "voiceogg/touko03.ogg",
    "voiceogg/touko04.ogg",
    "voiceogg/touko05.ogg",
    "voiceogg/touko06.ogg",
    "voiceogg/touko07.ogg",
    "voiceogg/touko07se.ogg",
    "voiceogg/youko01.ogg",
    "voiceogg/youko02.ogg",
    "voiceogg/youko03.ogg",
];

#[derive(Default)]
pub struct Loader {
    running: bool,
    /// reloadArmatures : les armatures seulement (changement de skin).
    reload: bool,
    armatures: usize,
    effects: usize,
    /// Avancement (0 à 1).
    pub progress: f32,
}

impl Game {
    /// Un nouveau chargement (créé avec chaque titre).
    pub(super) fn loader_start(&mut self) {
        self.loader = Loader { running: true, ..Loader::default() };
    }

    /// ResourceLoader::reloadArmatures : tout le cache d'images est vidé,
    /// les feuilles communes relues (sans l'image de Taichi), puis les
    /// armatures rechargées avec la feuille du nouveau skin.
    pub(super) fn loader_reload(&mut self) {
        self.assets.purge();
        self.assets.add_sprite_frames("gamesheet.plist");
        self.assets.add_sprite_frames("backgrounds.plist");
        self.assets.remove_frame("c_taichifly.png");
        self.loader = Loader { running: true, reload: true, ..Loader::default() };
    }

    pub(super) fn loader_progress(&self) -> f32 {
        self.loader.progress
    }

    /// Une image de chargement.
    pub(super) fn loader_step(&mut self) {
        if !self.loader.running {
            return;
        }
        let progress = if self.loader.armatures < ARMATURES.len() {
            let sheet = self.armature_sheet();
            self.assets.add_sprite_frames_with(&format!("{sheet}.plist"), Some(&format!("{sheet}.png")));
            self.assets.add_armature_file(ARMATURES[self.loader.armatures]);
            self.loader.armatures += 1;
            let done = self.loader.armatures as f32 / ARMATURES.len() as f32;
            // Le rechargement donne l'avancement des armatures tel quel.
            if self.loader.reload { done } else { done * 0.5 }
        } else if self.loader.effects < EFFECTS.len() && !self.loader.reload {
            let assets = self.assets.clone();
            self.audio.preload(&assets, EFFECTS[self.loader.effects]);
            self.loader.effects += 1;
            self.loader.effects as f32 / EFFECTS.len() as f32 * 0.5 + 0.5
        } else {
            self.loader.running = false;
            return;
        };
        self.loader.progress = progress;
        match self.logic {
            Logic::Title(_) => title::on_preload(self, (progress * 100.0).ceil() as i32),
            Logic::Option(_) => super::option::on_preload(self, progress),
            _ => {}
        }
    }
}
