//! L'interface de la partie : le menu (clip 465), la jauge de lancement
//! (514), l'attaque aérienne (438), le chrono des attaques spéciales (479),
//! la vitesse (506), la distance (497), le record (493), la hauteur (433),
//! les visages des personnages qui approchent (476) et les personnages
//! disponibles pour les attaques spéciales (489).

use std::collections::HashMap;

use super::as2::{meters, number_to_string, round};
use super::control::{KIRI, MIKI, MISATO, SAKURABA, TOMOKI, TOUKO, YOUKO};
use super::{Flow, Game};
use crate::display::Obj;
use crate::library::Id;

/// Les boutons que le menu montre et cache (sans menu_btn).
const MENU_BUTTONS: [&str; 6] = ["pauser", "title_btn", "meter_btn", "againr", "quality_btn", "sound_btn"];

#[derive(Default)]
pub struct Hud {
    /// menu_mc (la variable btn de la jauge, de l'aérien et du chrono).
    pub menu: Option<Obj>,
    /// init de highscore_mc.
    pub highscore_init: HashMap<Obj, String>,
    m: Menu,
    meter: Meter,
    aerial: Aerial,
    special: Special,
    speed: Speed,
    score: HashMap<Obj, (bool, Option<f64>)>,
    out: HashMap<Obj, f64>,
    /// basex et pointy de chaque visage.
    faces: HashMap<Obj, (f64, f64)>,
}

#[derive(Default)]
struct Menu {
    roll: bool,
    openstatus: bool,
    hideenable: bool,
    againr: Option<Obj>,
    pauser: Option<Obj>,
    /// La variable roll posée sur chaque bouton.
    rolls: HashMap<Obj, bool>,
}

#[derive(Default)]
struct Meter {
    starttime: f64,
}

#[derive(Default)]
struct Aerial {
    count: f64,
    initx: f64,
    targetdagger: Option<Obj>,
    upper: bool,
    uppercount: f64,
    downercount: f64,
    downerint: f64,
    aerialenabled: bool,
    active: bool,
}

#[derive(Default)]
struct Special {
    status: f64,
    timer: f64,
}

#[derive(Default)]
struct Speed {
    vval: Option<f64>,
    hval: Option<f64>,
    blindmode: bool,
}

impl Game {
    pub(super) fn hud_frame(&mut self, o: Obj, id: Option<Id>, frame: u16) -> bool {
        match id {
            Some(465) => match frame {
                1 => self.menu_init(o),
                _ => self.stop(o),
            },
            Some(514) => self.meter_script(o, frame),
            Some(438) => self.aerial_script(o, frame),
            Some(479) => self.special_script(o, frame),
            Some(506) => {
                self.stop(o);
                for name in ["score_txt"] {
                    if let Some(t) = self.child(o, name) {
                        self.d.set_autosize_right(t);
                    }
                }
                self.hud.speed = Speed::default();
                for name in ["hscore_txt", "vscore_txt"] {
                    if let Some(t) = self.child(o, name) {
                        self.d.set_visible(t, false);
                    }
                }
                self.speed_mode_set(o);
                self.speed_view_in(o, 0.0, 0.0);
            }
            Some(497) => match frame {
                1 => {
                    self.stop(o);
                    if let Some(t) = self.child(o, "score_txt") {
                        self.d.set_autosize_right(t);
                    }
                    self.hud.score.insert(o, (false, None));
                    self.score_view(o, 0.0, false);
                }
                _ => self.stop(o),
            },
            Some(493) => {
                self.stop(o);
                if let Some(t) = self.child(o, "score_txt") {
                    self.d.set_autosize_right(t);
                    let init = self.hud.highscore_init.get(&o).cloned().unwrap_or_else(|| "undefined".into());
                    self.d.set_text(t, &init);
                }
            }
            Some(433) => {
                self.stop(o);
                if let Some(t) = self.child(o, "height_txt") {
                    self.d.set_autosize_right(t);
                }
                self.hud.out.insert(o, 0.0);
            }
            Some(476) => self.stop(o),
            Some(489) => self.specialchara_view(o),
            _ => return false,
        }
        true
    }

    pub(super) fn hud_button(&mut self, id: Id, this: Obj) -> bool {
        match id {
            441 => {
                if self.hud.m.openstatus {
                    self.hud.m.openstatus = false;
                    self.menu_show(this, false);
                } else {
                    self.menu_view(this);
                }
            }
            444 | 463 => {
                if let Some(main) = self.parent(this) {
                    self.all_pause_stop();
                    if let Some(changer) = self.root.changer {
                        self.change_scene(changer, if id == 444 { "again" } else { "gotitle" }, Some(main));
                    }
                }
            }
            448 => {
                let label = if self.all_pause_toggle() { "pause" } else { "normal" };
                self.native_goto_and_stop(this, label);
            }
            452 => {
                if let Some(s) = self.parent(this).and_then(|m| self.child(m, "speed_mc")) {
                    self.control.metermode = (self.control.metermode + 1.0) % 3.0;
                    self.local_set();
                    self.speed_mode_set(s);
                }
            }
            456 => {
                self.cycle_quality();
                self.local_set();
                self.quality_text(this);
            }
            460 => {
                let bgm = self.parent(this).and_then(|m| self.child(m, "bgm_mc"));
                if self.volume(None) == 0.0 {
                    self.set_volume(None, 100.0);
                    self.local_set();
                    if let Some(b) = bgm {
                        self.bgm_sound_restart(b);
                    }
                } else {
                    self.set_volume(None, 0.0);
                    self.local_set();
                    if let Some(b) = bgm {
                        self.bgm_sound_stop(b);
                    }
                }
                self.sound_text(this);
            }
            _ => return false,
        }
        true
    }

    // --- Menu ---------------------------------------------------------------------

    /// Un bouton du menu par son nom ou sa variable (pauser_btn, againr_btn).
    fn menu_button(&self, menu: Obj, name: &str) -> Option<Obj> {
        match name {
            "pauser" => self.hud.m.pauser,
            "againr" => self.hud.m.againr,
            _ => self.child(menu, name),
        }
    }

    fn menu_init(&mut self, menu: Obj) {
        self.stop(menu);
        self.hud.m = Menu::default();
        for name in ["menu_btn", "pause_btn", "paused_btn", "title_btn", "meter_btn", "again_btn", "againd_btn", "quality_btn", "sound_btn"] {
            let Some(b) = self.child(menu, name) else { continue };
            let viewsub: Option<fn(&mut Game, Obj)> = match name {
                "quality_btn" => Some(Game::quality_text),
                "sound_btn" => Some(Game::sound_text),
                _ => None,
            };
            let events = self.events(b);
            events.roll_over = Some(Box::new(move |g, this| {
                g.hud.m.rolls.insert(this, true);
                if let Some(f) = viewsub {
                    f(g, menu);
                }
                g.menu_view(menu);
                Flow::Keep
            }));
            let rollout = move |g: &mut Game, this: Obj| {
                g.hud.m.rolls.insert(this, false);
                g.menu_hide(menu);
                Flow::Keep
            };
            events.roll_out = Some(Box::new(rollout));
            events.release_outside = Some(Box::new(rollout));
        }
        self.hud.m.hideenable = true;
        let again = self.child(menu, "again_btn");
        let againd = self.child(menu, "againd_btn");
        let pause = self.child(menu, "pause_btn");
        let paused = self.child(menu, "paused_btn");
        self.hud.m.againr = againd;
        if let Some(a) = againd {
            self.stop(a);
            self.set_hand_cursor(a, false);
        }
        if let Some(a) = again {
            self.d.set_visible(a, false);
            self.set_enabled(a, false);
        }
        self.hud.m.pauser = paused;
        if let Some(p) = paused {
            self.stop(p);
            self.set_hand_cursor(p, false);
        }
        if let Some(p) = pause {
            self.d.set_visible(p, false);
            self.set_enabled(p, false);
        }
        self.menu_hide(menu);
    }

    /// Le survol d'au moins un bouton du menu.
    fn menu_roll(&self, menu: Obj) -> bool {
        ["menu_btn", "meter_btn", "pauser", "title_btn", "againr", "quality_btn", "sound_btn"]
            .iter()
            .filter_map(|n| self.menu_button(menu, n))
            .any(|b| self.hud.m.rolls.get(&b).copied().unwrap_or(false))
    }

    fn menu_show(&mut self, menu: Obj, v: bool) {
        for name in MENU_BUTTONS {
            if let Some(b) = self.menu_button(menu, name) {
                self.d.set_visible(b, v);
                self.set_enabled(b, v);
            }
        }
    }

    /// viewbtn.
    fn menu_view(&mut self, menu: Obj) {
        self.hud.m.roll = self.menu_roll(menu);
        self.hud.m.openstatus = true;
        self.menu_show(menu, true);
    }

    /// hidebtn.
    fn menu_hide(&mut self, menu: Obj) {
        self.hud.m.roll = self.menu_roll(menu);
        if !self.hud.m.roll && self.hud.m.hideenable {
            self.hud.m.openstatus = false;
            self.menu_show(menu, false);
        }
    }

    /// Le bouton PAUSE sert (de la fin du lancement à la fin de la partie).
    pub(super) fn pause_available(&self, menu: Obj) -> bool {
        self.hud.m.pauser.is_some() && self.hud.m.pauser == self.child(menu, "pause_btn")
    }

    /// btn.roll : la souris est sur le menu.
    fn menu_rolled(&self) -> bool {
        self.hud.menu.is_some() && self.hud.m.roll
    }

    pub(super) fn menu_game_start(&mut self, menu: Obj) {
        let swap = |g: &mut Game, live: &str, dead: &str| {
            let (l, d) = (g.child(menu, live), g.child(menu, dead));
            if let Some(l) = l {
                g.hud.m.rolls.insert(l, false);
                let (vis, en) = d.map_or((false, true), |d| (g.d.visible(d), g.enabled(d)));
                g.d.set_visible(l, vis);
                g.set_enabled(l, en);
            }
            if let Some(d) = d {
                g.d.set_visible(d, false);
                g.set_enabled(d, false);
            }
            l
        };
        self.hud.m.againr = swap(self, "again_btn", "againd_btn");
        self.hud.m.pauser = swap(self, "pause_btn", "paused_btn");
    }

    pub(super) fn menu_game_over(&mut self, menu: Obj) {
        let (paused, pause) = (self.child(menu, "paused_btn"), self.child(menu, "pause_btn"));
        if let Some(p) = paused {
            self.hud.m.rolls.insert(p, false);
            let (vis, en) = pause.map_or((false, true), |q| (self.d.visible(q), self.enabled(q)));
            self.d.set_visible(p, vis);
            self.set_enabled(p, en);
        }
        self.hud.m.pauser = paused;
        if let Some(q) = pause {
            self.d.set_visible(q, false);
            self.set_enabled(q, false);
        }
        self.hud.m.hideenable = false;
        self.menu_view(menu);
    }

    /// qualitytext : le texte du bouton de qualité.
    fn quality_text(&mut self, menu: Obj) {
        let text = match self.quality {
            "HIGH" => "HIGH > MEDIUM",
            "MEDIUM" => "MEDIUM > LOW",
            "LOW" => "LOW > HIGH",
            _ => "> HIGH",
        };
        self.set_var(menu, "qtxt", text.into());
    }

    /// soundtext : le texte du bouton du son.
    fn sound_text(&mut self, menu: Obj) {
        let text = if self.volume(None) == 0.0 { "OFF > ON" } else { "ON > OFF" };
        self.set_var(menu, "stxt", text.into());
    }

    // --- Jauge de lancement --------------------------------------------------------------

    fn meter_script(&mut self, m: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(m);
                self.interval = None;
                self.delete_mouse_up(m);
                self.delete_mouse_down(m);
                if let Some(r) = self.child(m, "release_mc") {
                    self.d.set_visible(r, false);
                }
                if let Some(m2) = self.child(m, "meter2_mc") {
                    self.d.set_xscale(m2, 0.0);
                    self.d.set_yscale(m2, 0.0);
                }
                self.hud.meter.starttime = self.time;
                self.interval = Some((m, Box::new(|g, this| {
                    // meter1 : l'angle va et vient.
                    let val = round((g.time - g.hud.meter.starttime) * 0.18 % 180.0);
                    if let Some(m1) = g.child(this, "meter1_mc") {
                        g.d.set_rotation(m1, -val.min(180.0 - val));
                    }
                    Flow::Keep
                })));
                self.on_mouse_down(m, |g, this| {
                    if g.menu_rolled() {
                        return Flow::Keep;
                    }
                    g.interval = None;
                    g.delete_mouse_down(this);
                    if let Some(r) = g.child(this, "release_mc") {
                        g.d.set_visible(r, true);
                    }
                    if let Some(p) = g.child(this, "press_mc") {
                        g.d.set_visible(p, false);
                    }
                    g.hud.meter.starttime = g.time;
                    g.interval = Some((this, Box::new(|g, this| {
                        // meter2 : la puissance grandit et rétrécit.
                        let val = round((g.time - g.hud.meter.starttime) * 0.3 % 200.0);
                        if let Some(m2) = g.child(this, "meter2_mc") {
                            g.d.set_xscale(m2, val.min(200.0 - val));
                            let xs = g.d.xscale(m2);
                            g.d.set_yscale(m2, xs);
                        }
                        Flow::Keep
                    })));
                    g.on_mouse_up(this, |g, this| {
                        g.interval = None;
                        g.delete_mouse_down(this);
                        g.control.i_angle = -g.child(this, "meter1_mc").map_or(f64::NAN, |m1| g.d.rotation(m1));
                        g.control.i_power = g.child(this, "meter2_mc").map_or(f64::NAN, |m2| g.d.xscale(m2));
                        g.game_start();
                        for name in ["press_mc", "release_mc"] {
                            if let Some(c) = g.child(this, name) {
                                g.d.set_visible(c, false);
                            }
                        }
                        g.play(this);
                        Flow::Delete
                    });
                    Flow::Delete
                });
            }
            2 => {
                let angle = format!("{}°", number_to_string(self.control.i_angle));
                let power = format!("{}%", number_to_string(self.control.i_power));
                self.set_var(m, "angle", angle);
                self.set_var(m, "power", power);
            }
            5 => self.stop(m),
            _ => {}
        }
    }

    // --- Attaque aérienne -------------------------------------------------------------------

    fn aerial_script(&mut self, a: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(a);
                self.hud.aerial = Aerial { initx: self.d.x(a), uppercount: 3.0, downercount: 100.0, active: true, ..Aerial::default() };
                for name in ["dr1_mc", "dr2_mc", "dr3_mc", "db_mc"] {
                    if let Some(c) = self.child(a, name) {
                        self.stop(c);
                    }
                }
                self.on_mouse_down(a, |g, this| {
                    let h = &g.hud.aerial;
                    if h.aerialenabled && !g.menu_rolled() && !g.paused(this) {
                        g.hud.aerial.aerialenabled = false;
                        let upper = g.hud.aerial.upper;
                        if upper {
                            let n = g.hud.aerial.uppercount;
                            g.hud.aerial.uppercount -= 1.0;
                            let name = match n as i64 {
                                1 => Some("dr1_mc"),
                                2 => Some("dr2_mc"),
                                3 => Some("dr3_mc"),
                                _ => None,
                            };
                            if let Some(name) = name {
                                g.hud.aerial.targetdagger = g.child(this, name);
                            }
                        } else {
                            g.hud.aerial.downercount = 0.0;
                            g.hud.aerial.targetdagger = g.child(this, "db_mc");
                        }
                        g.hud.aerial.active = false;
                        g.goto_and_play(this, "aerial");
                        g.aerial_crash(upper);
                    }
                    Flow::Keep
                });
                self.aerial_view_in(a, false, false, 0.0);
            }
            2 => {
                if let Some(t) = self.hud.aerial.targetdagger {
                    self.play(t);
                }
            }
            16 => {
                self.stop(a);
                if let Some(t) = self.hud.aerial.targetdagger {
                    self.d.set_visible(t, false);
                    self.goto_and_stop(t, 1);
                }
                self.hud.aerial.count = 0.0;
                self.on_enter_frame(a, |g, this| {
                    g.hud.aerial.count += 1.0;
                    let count = g.hud.aerial.count;
                    let xs = g.d.xscale(this);
                    g.d.set_x(this, -count / 10.0 * 250.0 / 100.0 * xs + g.hud.aerial.initx);
                    if count >= 10.0 {
                        g.play(this);
                        return Flow::Delete;
                    }
                    Flow::Keep
                });
            }
            17 => {
                let upper = self.hud.aerial.upper;
                for (name, v) in [("crashr_mc", upper), ("crashb_mc", !upper)] {
                    if let Some(c) = self.child(a, name) {
                        self.d.set_visible(c, v);
                    }
                }
            }
            49 => {
                self.stop(a);
                self.hud.aerial.count = 0.0;
                self.on_enter_frame(a, |g, this| {
                    g.hud.aerial.count += 1.0;
                    let count = g.hud.aerial.count;
                    let xs = g.d.xscale(this);
                    g.d.set_x(this, (count / 10.0 - 1.0) * 250.0 * xs / 100.0 + g.hud.aerial.initx);
                    if count >= 10.0 {
                        g.hud.aerial.active = true;
                        return Flow::Delete;
                    }
                    Flow::Keep
                });
            }
            _ => {}
        }
    }

    /// aerial.view, appelé à chaque image du vol.
    pub(super) fn aerial_view(&mut self, status: bool, fall: bool, interval: f64) {
        if let Some(a) = self.control.aerial {
            self.aerial_view_in(a, status, fall, interval);
        }
    }

    fn aerial_view_in(&mut self, a: Obj, status: bool, fall: bool, interval: f64) {
        if status {
            self.hud.aerial.downercount = 100f64.min(self.hud.aerial.downercount + interval * 100.0 / 3.0);
        }
        let n = self.hud.aerial.downercount.floor();
        let text = self.child(a, "downer_txt");
        if self.hud.aerial.downerint < n {
            self.hud.aerial.downerint = n;
            if let Some(t) = text {
                self.d.set_text(t, &format!("{}%", number_to_string(n)));
            }
            if n == 100.0 {
                if let Some(db) = self.child(a, "db_mc") {
                    self.d.set_visible(db, true);
                }
                if let Some(t) = text {
                    self.set_rgb(t, 255);
                }
            }
        } else if self.hud.aerial.downerint > n {
            self.hud.aerial.downerint = n;
            if let Some(t) = text {
                self.d.set_text(t, &format!("{}%", number_to_string(n)));
                self.set_rgb(t, 5_592_405);
            }
        }
        let h = &self.hud.aerial;
        let enabled = status && h.active && (fall && h.uppercount > 0.0 || !fall && h.downerint == 100.0);
        self.hud.aerial.aerialenabled = enabled;
        if let Some(t) = self.child(a, "textd_mc") {
            self.d.set_visible(t, !enabled);
        }
        if enabled {
            self.hud.aerial.upper = fall;
        }
        for (name, v) in [("textr_mc", enabled && fall), ("textb_mc", enabled && !fall)] {
            if let Some(c) = self.child(a, name) {
                self.d.set_visible(c, v);
            }
        }
    }

    // --- Chrono des attaques spéciales ---------------------------------------------------------

    fn special_script(&mut self, s: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(s);
                self.d.set_visible(s, false);
                self.hud.special.status = 0.0;
            }
            31 => self.stop(s),
            32 => self.hud.special.status = 0.0,
            41 | 61 => {
                self.stop(s);
                self.d.set_visible(s, false);
                if frame == 61 {
                    self.hud.special.status = 1.0;
                }
            }
            _ => {}
        }
    }

    /// special.setstart : le joueur a 0,7 s pour cliquer.
    pub(super) fn special_start(&mut self) {
        let Some(s) = self.control.special else { return };
        self.hud.special.status = -1.0;
        self.d.set_visible(s, true);
        self.hud.special.timer = self.time;
        self.on_enter_frame(s, |g, this| {
            if g.time - g.hud.special.timer > 700.0 {
                g.delete_mouse_down(this);
                g.d.set_visible(this, true);
                g.goto_and_play(this, "ng");
                return Flow::Delete;
            }
            Flow::Keep
        });
        self.on_mouse_down(s, |g, this| {
            if !g.menu_rolled() && !g.paused(this) {
                g.d.set_visible(this, true);
                g.delete_enter_frame(this);
                g.goto_and_play(this, "ok");
                return Flow::Delete;
            }
            Flow::Keep
        });
        self.goto_and_play(s, "wait");
    }

    /// special.getstatus : -1 en attente, 0 raté, 1 réussi.
    pub(super) fn special_status(&self) -> f64 {
        self.hud.special.status
    }

    // --- Vitesse ---------------------------------------------------------------------------

    pub(super) fn speed_set_blind(&mut self, blind: bool) {
        self.hud.speed.blindmode = blind;
        if let Some(s) = self.control.speed {
            self.speed_mode_set(s);
        }
    }

    pub(super) fn speed_view(&mut self, vx: f64, vy: f64) {
        if let Some(s) = self.control.speed {
            self.speed_view_in(s, vx, vy);
        }
    }

    /// modeset : ce que montre le compteur selon le mode choisi.
    fn speed_mode_set(&mut self, s: Obj) {
        let b = self.hud.speed.blindmode;
        let shown: [bool; 7] = match self.control.metermode as i64 {
            0 => [b, false, !b, false, true, false, false],
            1 => [b, false, false, !b, false, true, false],
            2 => [b, !b, false, false, false, false, !b],
            _ => return,
        };
        for (name, v) in ["blind_txt", "score_txt", "hscore_txt", "vscore_txt", "rightarrow_mc", "uparrow_mc", "movearrow_mc"].iter().zip(shown) {
            if let Some(c) = self.child(s, name) {
                self.d.set_visible(c, v);
            }
        }
    }

    pub(super) fn speed_view_in(&mut self, s: Obj, h: f64, v: f64) {
        let sp = &self.hud.speed;
        // L'original compare hval à la vitesse verticale.
        if sp.vval != Some(v) || sp.hval != Some(v) {
            if let Some(a) = self.child(s, "movearrow_mc") {
                self.d.set_rotation(a, (-v).atan2(h) * 180.0 / std::f64::consts::PI);
            }
            if let Some(t) = self.child(s, "score_txt") {
                self.d.set_text(t, &meters((v * v + h * h).sqrt()));
            }
        }
        if self.hud.speed.vval != Some(v) {
            let old = self.hud.speed.vval.unwrap_or(0.0);
            if let Some(a) = self.child(s, "uparrow_mc") {
                if old < 0.0 && v >= 0.0 {
                    self.d.set_rotation(a, -90.0);
                } else if old >= 0.0 && v < 0.0 {
                    self.d.set_rotation(a, 90.0);
                }
            }
            self.hud.speed.vval = Some(v);
            if let Some(t) = self.child(s, "vscore_txt") {
                self.d.set_text(t, &meters(v.abs()));
            }
        }
        if self.hud.speed.hval != Some(h) {
            self.hud.speed.hval = Some(h);
            if let Some(t) = self.child(s, "hscore_txt") {
                self.d.set_text(t, &meters(h));
            }
        }
    }

    // --- Distance, record, hauteur -------------------------------------------------------------

    /// score_mc.view : rend le texte quand le record tombe.
    pub(super) fn score_view(&mut self, s: Obj, score: f64, breakscore: bool) -> Option<String> {
        let (changed, val) = self.hud.score.get(&s).copied().unwrap_or((false, None));
        if val != Some(score) {
            let text = meters(score);
            self.hud.score.insert(s, (changed, Some(score)));
            if let Some(t) = self.child(s, "score_txt") {
                self.d.set_text(t, &text);
            }
            if breakscore {
                if !changed {
                    self.hud.score.insert(s, (true, Some(score)));
                    self.next_frame(s);
                    self.set_rgb(s, 0xFF_0000);
                }
                return Some(text);
            }
        }
        None
    }

    pub(super) fn highscore_view(&mut self, h: Obj, score: Option<String>) {
        if let Some(text) = score
            && let Some(t) = self.child(h, "score_txt")
        {
            self.d.set_text(t, &text);
        }
    }

    pub(super) fn out_view(&mut self, out: Obj, y: f64) {
        if y > 14.0 {
            self.d.set_visible(out, true);
            if self.hud.out.get(&out).copied() != Some(y) {
                self.hud.out.insert(out, y);
                if let Some(t) = self.child(out, "height_txt") {
                    self.d.set_text(t, &meters(y));
                }
            }
        } else {
            self.d.set_visible(out, false);
        }
    }

    // --- Visages et personnages disponibles ---------------------------------------------------

    /// contact.getchara : le visage d'un personnage.
    pub(super) fn contact_get_chara(&self, contact: Obj, code: f64) -> Option<Obj> {
        let name = match code {
            MISATO => "misato_mc",
            TOUKO => "touko_mc",
            KIRI => "kiri_mc",
            MIKI => "miki_mc",
            YOUKO => "youko_mc",
            SAKURABA => "sakuraba_mc",
            TOMOKI => "tomoki_mc",
            _ => return None,
        };
        self.child(contact, name)
    }

    /// Le constructeur de GameCharaFace.
    pub(super) fn face_constructor(&mut self, f: Obj) {
        self.d.set_visible(f, false);
        self.stop(f);
        if let Some(a) = self.child(f, "arrow_mc") {
            self.stop(a);
        }
        let parent = self.parent(f);
        let basex = parent.map_or(f64::NAN, |p| self.d.x(p));
        let pointy = 370.0 - parent.map_or(f64::NAN, |p| self.d.y(p));
        self.hud.faces.insert(f, (basex, pointy));
    }

    /// GameCharaFace.view : le visage et sa flèche vers le personnage.
    pub(super) fn face_view(&mut self, f: Obj, px: f64, scale: f64) {
        let Some(&(basex, pointy)) = self.hud.faces.get(&f) else { return };
        self.d.set_x(f, ((px / 150.0 + 1.0) / 10.0).floor() * 31.25);
        let x = self.d.x(f);
        if x >= 0.0 {
            self.d.set_visible(f, true);
            let dx = px * scale / 100.0 + 250.0 - x - basex;
            let k = pointy / (pointy * pointy + dx * dx).sqrt();
            if let Some(a) = self.child(f, "arrow_mc") {
                self.d.set_xscale(a, 100.0 / k);
                self.d.set_yscale(a, 100.0 * k);
                self.d.set_rotation(a, pointy.atan2(dx) * 180.0 / std::f64::consts::PI);
            }
        } else {
            self.d.set_visible(f, false);
        }
    }

    /// specialchara.view : quels personnages peuvent lancer une attaque.
    pub(super) fn specialchara_view(&mut self, s: Obj) {
        let c = &self.control;
        let (youko, nanaka) = (c.youko, c.nanaka);
        let (misato, touko, kiri, miki) = (c.misato, c.touko, c.kiri, c.miki);
        // L'original nomme « mikid_mc » un clip appelé « mkid_mc » : il
        // n'est jamais touché.
        let shown: Vec<(&str, bool)> = if youko {
            let mut v = vec![("base_mc", false), ("youko_mc", true), ("misato_mc", false), ("touko_mc", false), ("kiri_mc", false), ("miki_mc", false)];
            v.extend([("nanaka_mc", nanaka), ("misatod_mc", !nanaka), ("toukod_mc", !nanaka), ("kirid_mc", !nanaka), ("mikid_mc", !nanaka)]);
            v
        } else {
            vec![
                ("youko_mc", false),
                ("nanaka_mc", false),
                ("base_mc", true),
                ("misato_mc", misato),
                ("touko_mc", touko),
                ("kiri_mc", kiri),
                ("miki_mc", miki),
                ("misatod_mc", !misato),
                ("toukod_mc", !touko),
                ("kirid_mc", !kiri),
                ("mikid_mc", !miki),
            ]
        };
        for (name, v) in shown {
            if let Some(c) = self.child(s, name) {
                self.d.set_visible(c, v);
            }
        }
    }

    pub(super) fn hud_unload(&mut self, o: Obj) {
        self.hud.score.remove(&o);
        self.hud.out.remove(&o);
        self.hud.faces.remove(&o);
        self.hud.highscore_init.remove(&o);
        self.hud.m.rolls.remove(&o);
        self.scene_unload(o);
    }
}
