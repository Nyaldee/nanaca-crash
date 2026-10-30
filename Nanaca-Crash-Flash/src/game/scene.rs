//! La scène de jeu : s_main (clip 534) et ses effets, s_chara (clip 429)
//! qui porte Taichi, Nanaka et les personnages, s_back (clip 288) et les
//! calques du décor qui défilent.

use std::collections::HashMap;

use super::{ColorObject, Flow, Game};
use crate::display::Obj;
use crate::library::Id;

/// Les personnages de s_chara, dans l'ordre où moveobj les appelle.
const CHARA_NAMES: [&str; 7] = ["youko_mc", "miki_mc", "kiri_mc", "touko_mc", "tomoki_mc", "misato_mc", "sakuraba_mc"];

#[derive(Default)]
pub struct Scene {
    /// effectparam de s_main.
    effectparam: Option<f64>,
    /// count de s_main (frame 15) et de score_mc (fin de partie).
    count: f64,
    /// La variable scale de s_back (undefined au départ).
    back_scale: HashMap<Obj, f64>,
    /// Les variables des calques du décor (clips 277, 279, 282, 283, 287).
    layers: HashMap<Obj, Layer>,
    /// cfftype de cff_mc (clip 533).
    cfftype: HashMap<Obj, Option<f64>>,
}

/// unitwidth, mcwidth, et pour les arbres scale, size et z ; NaN tant que
/// le script du calque ne les a pas posés.
#[derive(Clone, Copy)]
struct Layer {
    unitwidth: f64,
    mcwidth: f64,
    scale: f64,
    size: f64,
    z: f64,
}

impl Default for Layer {
    fn default() -> Layer {
        Layer { unitwidth: f64::NAN, mcwidth: f64::NAN, scale: f64::NAN, size: f64::NAN, z: f64::NAN }
    }
}

impl Game {
    pub(super) fn scene_frame(&mut self, o: Obj, id: Option<Id>, frame: u16) -> bool {
        match id {
            Some(534) => self.main_script(o, frame),
            Some(429) => self.chara_scene_script(o, frame),
            Some(288) => self.back_script(o, frame),
            Some(277 | 279) => {
                self.stop(o);
                let unitwidth = self.unit_x(o, "unit2_mc") - self.unit_x(o, "unit1_mc");
                let mcwidth = self.child(o, "unit1_mc").map_or(f64::NAN, |u| self.d.width(u));
                let l = self.scene.layers.entry(o).or_default();
                l.unitwidth = unitwidth;
                l.mcwidth = mcwidth;
            }
            Some(282) => {
                self.stop(o);
                self.scene.layers.entry(o).or_default().unitwidth = 250.0;
            }
            Some(283) => {
                self.stop(o);
                let unitwidth = self.unit_x(o, "unit2_mc") - self.unit_x(o, "unit1_mc");
                let mcwidth = self.child(o, "unit1_mc").map_or(f64::NAN, |u| self.d.width(u));
                let l = self.scene.layers.entry(o).or_default();
                l.size = 0.5;
                l.z = 1.0;
                l.unitwidth = unitwidth;
                l.mcwidth = mcwidth;
            }
            Some(287) => {
                self.stop(o);
                let unitwidth = self.unit_x(o, "unit2_mc") - self.unit_x(o, "unit1_mc");
                self.scene.layers.entry(o).or_default().unitwidth = unitwidth;
            }
            Some(533) => {
                self.stop(o);
                for name in ["typea_mc", "typeb_mc", "typec_mc", "typed_mc"] {
                    if let Some(c) = self.child(o, name) {
                        self.d.set_visible(c, false);
                    }
                }
                let shown = match self.scene.cfftype.get(&o).copied().flatten() {
                    Some(super::control::MISATO) => Some("typea_mc"),
                    Some(super::control::TOUKO) => Some("typeb_mc"),
                    Some(super::control::KIRI) => Some("typec_mc"),
                    Some(super::control::MIKI) => Some("typed_mc"),
                    _ => None,
                };
                if let Some(c) = shown.and_then(|n| self.child(o, n)) {
                    self.d.set_visible(c, true);
                }
            }
            _ => return false,
        }
        true
    }

    fn unit_x(&self, o: Obj, name: &str) -> f64 {
        self.child(o, name).map_or(f64::NAN, |u| self.d.x(u))
    }

    // --- s_main ----------------------------------------------------------------

    fn main_script(&mut self, m: Obj, frame: u16) {
        let child = |g: &Game, name: &str| g.child(m, name);
        match frame {
            1 => {
                self.stop(m);
                self.all_pause_play();
                if let Some(c) = child(self, "chara_mc") {
                    self.goto_and_stop(c, "stop");
                }
                if let Some(b) = child(self, "back_mc") {
                    self.goto_and_stop(b, "stop");
                    self.d.set_visible(b, true);
                }
                self.delete_enter_frame(m);
                self.set_volume(Some(m), 100.0);
            }
            2 => {
                self.stop(m);
                let special = child(self, "special_mc");
                if let Some(s) = special {
                    self.d.set_visible(s, false);
                }
                self.control.special = special;
                self.hud.menu = child(self, "menu_mc");
                self.control.contact = child(self, "contact_mc");
                if let Some(out) = child(self, "out_mc") {
                    self.d.set_visible(out, false);
                }
                if let Some(h) = child(self, "highscore_mc") {
                    let best = self.best();
                    self.hud.highscore_init.insert(h, best);
                }
                self.control.aerial = child(self, "aerial_mc");
                self.control.speed = child(self, "speed_mc");
                for name in ["chara_mc", "back_mc"] {
                    if let Some(c) = child(self, name) {
                        self.goto_and_play(c, "standby");
                    }
                }
            }
            3 => {
                for name in ["chara_mc", "back_mc"] {
                    if let Some(c) = child(self, name) {
                        self.goto_and_play(c, "start");
                    }
                }
                if let Some(menu) = child(self, "menu_mc") {
                    self.menu_game_start(menu);
                }
            }
            15 => {
                if let Some(b) = child(self, "back_mc") {
                    self.d.set_visible(b, false);
                }
                self.scene.count = 0.0;
                self.on_enter_frame(m, |g, this| {
                    g.scene.count += 1.0;
                    let count = g.scene.count;
                    if let Some(crash) = g.child(this, "crash_mc") {
                        g.d.set_alpha(crash, (g.control.i_power * 25.0 - 100.0 * count) / 5.0 + 100.0);
                        g.d.set_x(crash, -200.0 * count / 30.0 + 450.0);
                        if g.d.alpha(crash) < 100.0
                            && let Some(b) = g.child(this, "back_mc")
                        {
                            g.d.set_visible(b, true);
                        }
                        if g.d.alpha(crash) <= 0.0 {
                            g.d.set_visible(crash, false);
                        }
                    }
                    if let Some(c) = g.child(this, "chara_mc") {
                        g.d.set_x(c, 450.0 - 6.666666666666667 * count);
                    }
                    if count >= 30.0 { Flow::Delete } else { Flow::Keep }
                });
            }
            35 => {
                if let Some(b) = child(self, "bgm_mc") {
                    self.bgm_sound_start(b, 2);
                }
            }
            45 => {
                self.stop(m);
                self.on_enter_frame(m, |g, _| {
                    g.control_move();
                    Flow::Keep
                });
            }
            46 | 114 | 140 => {
                if frame == 114 {
                    self.stop(m);
                }
                if let Some(b) = child(self, "back_mc") {
                    self.d.set_visible(b, false);
                }
            }
            89 | 104 | 163 => {
                if let Some(b) = child(self, "back_mc") {
                    self.d.set_visible(b, true);
                }
            }
            93 | 113 | 139 | 207 => self.stop(m),
            151 => {
                if let Some(c) = child(self, "cff_mc") {
                    let param = self.scene.effectparam;
                    self.scene.cfftype.insert(c, param);
                }
            }
            _ => {}
        }
    }

    /// backeffect : un effet de fond (« combo », « cff »…).
    pub(super) fn main_back_effect(&mut self, main: Obj, effect: &str, param: Option<f64>) {
        self.scene.effectparam = param;
        self.goto_and_play(main, effect);
    }

    /// moveobj de s_main : les personnages et le décor suivent Taichi.
    pub(super) fn main_move(&mut self, px: f64, py: f64, r: f64, scale: f64, changescale: bool) {
        let Some(main) = self.control.main else { return };
        if let Some(c) = self.child(main, "chara_mc") {
            self.chara_scene_move(c, px, py, r, scale, changescale);
        }
        if let Some(b) = self.child(main, "back_mc") {
            self.back_move(b, -px * 150.0, scale, changescale);
        }
    }

    /// changecolor : la teinte du Taichi de l'indicateur de hauteur.
    pub(super) fn main_change_color(&mut self, color: &ColorObject) {
        let Some(main) = self.control.main else { return };
        if let Some(t) = self.child(main, "out_mc").and_then(|o| self.child(o, "taichi_mc")) {
            self.set_color_transform(t, color);
        }
    }

    /// viewscore : hauteur, distance, record, vitesse, personnages.
    pub(super) fn main_view_score(&mut self, px: f64, py: f64, breakscore: bool, vx: f64, vy: f64) {
        let Some(main) = self.control.main else { return };
        if let Some(out) = self.child(main, "out_mc") {
            self.out_view(out, py);
        }
        let text = self.child(main, "score_mc").and_then(|s| self.score_view(s, px, breakscore));
        if let Some(h) = self.child(main, "highscore_mc") {
            self.highscore_view(h, text);
        }
        if let Some(s) = self.child(main, "speed_mc") {
            self.speed_view_in(s, vx, vy);
        }
        if let Some(s) = self.child(main, "specialchara_mc") {
            self.specialchara_view(s);
        }
    }

    /// gameover : le menu s'ouvre, le score grossit, le son baisse.
    pub(super) fn main_game_over(&mut self) {
        let Some(main) = self.control.main else { return };
        if let Some(menu) = self.child(main, "menu_mc") {
            self.menu_game_over(menu);
        }
        if let Some(score) = self.child(main, "score_mc") {
            self.scene.count = 0.0;
            self.on_enter_frame(score, |g, this| {
                g.scene.count += 1.0;
                let count = g.scene.count;
                g.d.set_xscale(this, 100.0 + 100.0 * count / 10.0);
                g.d.set_yscale(this, 100.0 + 100.0 * count / 10.0);
                g.d.set_x(this, -87.5 * count / 10.0 + 687.5);
                g.d.set_y(this, 159.4 * count / 10.0 + 25.6);
                if count >= 10.0 { Flow::Delete } else { Flow::Keep }
            });
        }
        self.on_enter_frame(main, |g, this| {
            let v = (g.volume(Some(this)) - 1.0).max(0.0);
            g.set_volume(Some(this), v);
            if g.volume(Some(this)) <= 0.0 { Flow::Delete } else { Flow::Keep }
        });
    }

    // --- s_chara -------------------------------------------------------------------

    fn chara_scene_script(&mut self, c: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(c);
                let (sscale, escale, sx, ex, sy, ey, frames) = (7000.0, 100.0, -84.75, -100.0, -300.0, -200.0, 15.0);
                self.d.set_xscale(c, sscale);
                self.d.set_yscale(c, sscale);
                self.d.set_x(c, 350.0 - sx * sscale / 100.0);
                self.d.set_y(c, 200.0 - sy * sscale / 100.0);
                let mut count: f64 = 0.0;
                self.on_enter_frame(c, move |g, this| {
                    let dt = count * count / frames / frames;
                    let scale = 1.0 / ((1.0 / escale - 1.0 / sscale) * dt + 1.0 / sscale);
                    let px = (ex - sx) * dt + sx;
                    let py = (ey - sy) * dt + sy;
                    g.d.set_xscale(this, scale);
                    g.d.set_yscale(this, scale);
                    g.d.set_x(this, 350.0 - px * scale / 100.0);
                    g.d.set_y(this, 200.0 - py * scale / 100.0);
                    count += 1.0;
                    if count > frames {
                        if let Some(main) = g.control.main {
                            g.goto_and_play(main, "standby");
                        }
                        return Flow::Delete;
                    }
                    Flow::Keep
                });
            }
            2 | 45 => self.stop(c),
            15 => {
                self.control.taichi = self.child(c, "taichi_mc");
                self.control.nanakaobj = self.child(c, "nanaka_mc");
                if let Some(n) = self.control.nanakaobj {
                    let a = -self.control.i_angle;
                    self.d.set_rotation(n, a);
                }
            }
            44 => {
                use super::control::{KIRI, MIKI, MISATO, SAKURABA, TOMOKI, TOUKO, YOUKO};
                for (name, code) in [("youko_mc", YOUKO), ("miki_mc", MIKI), ("kiri_mc", KIRI), ("touko_mc", TOUKO), ("tomoki_mc", TOMOKI), ("misato_mc", MISATO), ("sakuraba_mc", SAKURABA)] {
                    if let Some(ch) = self.child(c, name) {
                        self.init_chara(ch, code);
                    }
                }
            }
            _ => {}
        }
    }

    fn chara_scene_move(&mut self, c: Obj, px: f64, py: f64, r: f64, scale: f64, changescale: bool) {
        let xmin = -25000.0 / scale;
        let xmax = 45000.0 / scale;
        if let Some(t) = self.child(c, "taichi_mc") {
            self.taichi_move(t, py, r, scale);
        }
        if changescale {
            self.d.set_xscale(c, scale);
            self.d.set_yscale(c, scale);
        }
        if let Some(n) = self.child(c, "nanaka_mc") {
            self.nanaka_move(n, px, xmin);
        }
        for name in CHARA_NAMES {
            if let Some(ch) = self.child(c, name) {
                self.character_move(ch, px, py, scale, xmin, xmax);
            }
        }
    }

    // --- s_back et les calques du décor ----------------------------------------------

    fn back_script(&mut self, b: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(b);
                let (sscale, escale) = (7000.0, 100.0);
                let sx = -84.75 - 100.0 / (sscale / 100.0);
                let ex = -200.0;
                let sy = 300.0 + -200.0 / (sscale / 100.0);
                let ey = 0.0;
                let frames = 15.0;
                let scale = self.scene.back_scale.get(&b).copied().unwrap_or(f64::NAN);
                self.back_layers(b, -sx, sy / 150.0, sscale, scale, true, true);
                let mut count: f64 = 0.0;
                self.on_enter_frame(b, move |g, this| {
                    let dt = count * count / frames / frames;
                    let px = (ex - sx) * dt + sx;
                    let py = (ey - sy) * dt + sy;
                    let scale = 1.0 / ((1.0 / escale - 1.0 / sscale) * dt + 1.0 / sscale);
                    g.scene.back_scale.insert(this, scale);
                    g.back_layers(this, -px, py / 150.0, scale, scale, true, false);
                    count += 1.0;
                    if count > frames { Flow::Delete } else { Flow::Keep }
                });
            }
            2 | 16 => self.stop(b),
            15 => {
                self.stop(b);
                let mut count: f64 = 0.0;
                self.on_enter_frame(b, move |g, this| {
                    count += 1.0;
                    let px = -200.0 + 6.666666666666667 * count;
                    let scale = g.scene.back_scale.get(&this).copied().unwrap_or(f64::NAN);
                    g.back_layers(this, -px, 0.0, scale, scale, true, false);
                    if count == 30.0 {
                        g.play(this);
                        return Flow::Delete;
                    }
                    Flow::Keep
                });
            }
            _ => {}
        }
    }

    /// Les arbres et la rambarde (et au départ, les nuages).
    #[allow(clippy::too_many_arguments)]
    fn back_layers(&mut self, b: Obj, ppx: f64, py: f64, tree_scale: f64, rail_scale: f64, change: bool, clouds: bool) {
        if let Some(t) = self.child(b, "tree_mc") {
            self.tree_move(t, ppx, py, tree_scale, change);
        }
        if let Some(r) = self.child(b, "rail_mc") {
            self.rail_move(r, ppx, py, rail_scale, change);
        }
        if clouds {
            for name in ["cloud1_mc", "cloud2_mc"] {
                if let Some(c) = self.child(b, name) {
                    self.cloud_move(c, ppx);
                }
            }
        }
    }

    /// moveobj de s_back.
    fn back_move(&mut self, b: Obj, ppx: f64, scale: f64, change: bool) {
        if let Some(t) = self.child(b, "tree_mc") {
            self.tree_move(t, ppx, 0.0, scale, change);
        }
        if let Some(r) = self.child(b, "rail_mc") {
            self.rail_move(r, ppx, 0.0, scale, change);
        }
        if let Some(m) = self.child(b, "mount_mc") {
            let l = self.scene.layers.get(&m).copied().unwrap_or_default();
            self.d.set_x(m, (ppx / 300.0).floor() % l.unitwidth);
            self.d.set_visible(m, scale < 40.0);
        }
        for name in ["cloud1_mc", "cloud2_mc"] {
            if let Some(c) = self.child(b, name) {
                self.cloud_move(c, ppx);
            }
        }
    }

    /// Les nuages (clip 277 lents, 279 plus rapides).
    fn cloud_move(&mut self, c: Obj, ppx: f64) {
        let l = self.scene.layers.get(&c).copied().unwrap_or_default();
        let speed = if self.d.get(c).and_then(|x| x.character) == Some(279) { 200.0 } else { 300.0 };
        self.d.set_x(c, (ppx / speed).floor() % l.unitwidth);
        let x = self.d.x(c);
        let (right, left) = (450.0 - x, -250.0 - x - l.mcwidth);
        if let Some(u) = self.child(c, "unit1_mc") {
            let v = self.d.x(u) > left;
            self.d.set_visible(u, v);
        }
        if let Some(u) = self.child(c, "unit2_mc") {
            let v = self.d.x(u) < right;
            self.d.set_visible(u, v);
        }
    }

    /// Les arbres (clip 283), en perspective.
    fn tree_move(&mut self, t: Obj, ppx: f64, py: f64, mainscale: f64, change: bool) {
        let mut l = self.scene.layers.get(&t).copied().unwrap_or_default();
        if change {
            l.scale = 100.0 / (100.0 / mainscale + l.z) / l.size;
            self.scene.layers.entry(t).or_default().scale = l.scale;
            self.d.set_yscale(t, l.scale);
            self.d.set_xscale(t, l.scale);
        }
        self.d.set_x(t, ppx / (1.0 + l.z) % l.unitwidth / 100.0 * l.scale);
        self.d.set_y(t, py * l.size * 150.0 * (l.scale / 100.0));
        let x = self.d.x(t);
        let right = (450.0 - x) * 100.0 / l.scale;
        let left = (-250.0 - x) * 100.0 / l.scale - l.mcwidth;
        for (name, show) in [("unit1_mc", 0), ("unit2_mc", 0), ("unit3_mc", 2), ("unit4_mc", 1), ("unit5_mc", 1), ("unit6_mc", 1)] {
            if let Some(u) = self.child(t, name) {
                let ux = self.d.x(u);
                let v = match show {
                    0 => ux > left,
                    1 => ux < right,
                    _ => true,
                };
                self.d.set_visible(u, v);
            }
        }
    }

    /// La rambarde (clip 287).
    fn rail_move(&mut self, r: Obj, ppx: f64, py: f64, scale: f64, change: bool) {
        let l = self.scene.layers.get(&r).copied().unwrap_or_default();
        if change {
            self.d.set_yscale(r, scale);
            self.d.set_xscale(r, scale);
        }
        self.d.set_x(r, ppx % l.unitwidth * scale / 100.0);
        self.d.set_y(r, py * 150.0 * scale / 100.0);
        let x = self.d.x(r);
        let right = (450.0 - x) * 100.0 / scale;
        let left = (-250.0 - x) * 100.0 / scale - l.unitwidth;
        for (name, show_left) in [("unit1_mc", true), ("unit2_mc", true), ("unit4_mc", false), ("unit5_mc", false), ("unit6_mc", false)] {
            if let Some(u) = self.child(r, name) {
                let ux = self.d.x(u);
                let v = if show_left { ux > left } else { ux < right };
                self.d.set_visible(u, v);
            }
        }
    }

    pub(super) fn scene_unload(&mut self, o: Obj) {
        self.scene.back_scale.remove(&o);
        self.scene.layers.remove(&o);
        self.scene.cfftype.remove(&o);
    }
}
