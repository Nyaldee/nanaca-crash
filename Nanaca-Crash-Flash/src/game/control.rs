//! GameControl, l'objet global `control` : la physique du vol de Taichi
//! (vitesse, gravité, rebonds, ralentis), les attaques spéciales (C.F.F.),
//! les personnages croisés, et la sauvegarde.

use super::as2::meters;
use super::save::{SETTINGS, STATS, Value};
use super::{ColorObject, Game};
use crate::display::Obj;

/// Codes des personnages (characode).
pub const MISATO: f64 = 1.0;
pub const TOUKO: f64 = 2.0;
pub const KIRI: f64 = 3.0;
pub const MIKI: f64 = 4.0;
pub const YOUKO: f64 = 5.0;
pub const SAKURABA: f64 = 6.0;
pub const TOMOKI: f64 = 7.0;
pub const NANAKA: f64 = 8.0;

/// Gravité, rebond horizontal et vertical, pas de temps d'une image.
const G: f64 = -9.8;
const EX: f64 = 0.8;
const EY: f64 = 0.8;
const INTERVAL: f64 = 0.03333333333333333;

/// Teintes de Taichi selon l'attaque spéciale en cours.
const COLOR_A: [f64; 8] = [100.0, 0.0, 100.0, 128.0, 100.0, 0.0, 100.0, 0.0];
const COLOR_B: [f64; 8] = [100.0, 128.0, 100.0, 0.0, 100.0, 0.0, 100.0, 0.0];
const COLOR_C: [f64; 8] = [100.0, 0.0, 100.0, 0.0, 100.0, 128.0, 100.0, 0.0];
const COLOR_D: [f64; 8] = [100.0, 64.0, 100.0, 64.0, 100.0, 0.0, 100.0, 0.0];
const COLOR_N: [f64; 8] = [100.0, 0.0, 100.0, 0.0, 100.0, 0.0, 100.0, 0.0];

/// La version écrite dans la sauvegarde (le script dit 1.08).
const DATA_VERSION: f64 = 1.08;

/// displaysetting : le mode d'affichage (Alt+Entrée), propre au portage.
fn display_name(borderless: bool) -> &'static str {
    if borderless { "borderless" } else { "window" }
}

#[derive(Default)]
pub struct Control {
    pub vx: f64,
    pub vy: f64,
    pub px: f64,
    pub py: f64,
    pub r: f64,
    scale: f64,
    toggleflag: bool,
    pub slowcount: f64,
    slowinterval: f64,
    charapos: f64,
    pub chara_array: Vec<Obj>,
    pub youko: bool,
    pub nanaka: bool,
    pub misato: bool,
    pub touko: bool,
    pub combo: bool,
    pub kiri: bool,
    pub miki: bool,
    /// Le type du dernier personnage placé (None : undefined).
    pub lastchara: Option<f64>,
    pub cffcount: f64,
    /// L'attaque spéciale en cours (0 à 3 ; None : null).
    pub cffs: Option<u8>,
    pub nogravity: bool,
    nolimit: bool,
    pub hitenabled: bool,
    tmpx: f64,
    tmpy: f64,
    pub i_angle: f64,
    pub i_power: f64,
    pub metermode: f64,
    /// Le meilleur record (variable statique).
    pub bestrecord: f64,
    // Les objets que le contrôle pilote.
    pub main: Option<Obj>,
    pub taichi: Option<Obj>,
    pub nanakaobj: Option<Obj>,
    pub contact: Option<Obj>,
    pub special: Option<Obj>,
    pub aerial: Option<Obj>,
    pub speed: Option<Obj>,
}

impl Game {
    pub(super) fn local_load(&mut self) {
        self.control.bestrecord = self.save.number(STATS, "datarecord").unwrap_or(0.0);
        self.quality = match self.save.text(SETTINGS, "qualitysetting") {
            Some("HIGH") => "HIGH",
            Some("LOW") => "LOW",
            _ => "MEDIUM",
        };
        self.control.metermode = self.save.number(SETTINGS, "metersetting").unwrap_or(0.0);
        let volume = self.save.number(SETTINGS, "soundsetting").unwrap_or(100.0);
        self.set_volume(None, volume);
    }

    pub(super) fn local_set(&mut self) {
        let s = &mut self.save;
        s.set(STATS, "dataversion", Value::Number(DATA_VERSION));
        s.set(STATS, "datarecord", Value::Number(self.control.bestrecord));
        s.set(SETTINGS, "qualitysetting", Value::Text(self.quality.into()));
        s.set(SETTINGS, "displaysetting", Value::Text(display_name(self.borderless).into()));
        s.set(SETTINGS, "soundsetting", Value::Number(self.volumes.get(&None).copied().unwrap_or(100.0)));
        s.set(SETTINGS, "metersetting", Value::Number(self.control.metermode));
    }

    pub(super) fn local_save(&mut self) {
        self.local_set();
        self.save.flush();
    }

    pub(super) fn local_clear(&mut self) {
        self.control.bestrecord = 0.0;
        self.local_save();
    }

    /// control.best : le record en texte.
    pub(super) fn best(&self) -> String {
        meters(self.control.bestrecord)
    }

    fn color(v: [f64; 8]) -> ColorObject {
        ColorObject::new(v)
    }

    pub(super) fn game_start(&mut self) {
        let c = &mut self.control;
        c.vx = 0.3 * c.i_power * (c.i_angle * std::f64::consts::PI / 180.0).cos();
        c.vy = 0.3 * c.i_power * (c.i_angle * std::f64::consts::PI / 180.0).sin();
        if let Some(main) = c.main {
            self.goto_and_play(main, "start");
        }
    }

    pub(super) fn speed_blind(&mut self, blind: bool) {
        self.speed_set_blind(blind);
        let (vx, vy) = (self.control.vx, self.control.vy);
        self.speed_view(vx, vy);
    }

    pub(super) fn set_special(&mut self, chara: f64, flag: bool) {
        let c = &mut self.control;
        match chara {
            MISATO => c.misato = flag,
            TOUKO => c.touko = flag,
            KIRI => c.kiri = flag,
            MIKI => c.miki = flag,
            YOUKO => c.youko = flag,
            NANAKA => c.nanaka = flag,
            _ => {}
        }
    }

    pub(super) fn boost(&mut self, power: f64, angle: f64) {
        let p = self.do_cff_boost(power);
        let c = &mut self.control;
        let a = angle * std::f64::consts::PI / 180.0;
        c.vx += p * a.cos();
        c.vy = c.vy.abs() + p * a.sin();
    }

    pub(super) fn vpush(&mut self) {
        let c = &mut self.control;
        (c.tmpx, c.tmpy) = (c.vx, c.vy);
    }

    pub(super) fn vpop(&mut self) {
        let c = &mut self.control;
        (c.vx, c.vy) = (c.tmpx, c.tmpy);
    }

    fn cff_clear(&mut self) {
        self.control.cffs = None;
        self.main_change_color(&Self::color(COLOR_N));
        self.taichi_change_color(Self::color(COLOR_N), 20.0);
    }

    /// setcffa … setcffd : une attaque spéciale commence.
    pub(super) fn set_cff(&mut self, kind: u8) {
        let old = self.control.cffs;
        let count = self.control.cffcount;
        self.taichi_set_cff(old, kind, count);
        self.control.cffs = Some(kind);
        let color = match kind {
            0 => {
                self.control.cffcount += 30.0;
                COLOR_A
            }
            1 => {
                self.control.cffcount += 20.0;
                COLOR_B
            }
            2 => {
                self.vpush();
                self.speed_blind(true);
                let c = &mut self.control;
                c.vx = 10.0;
                c.vy = 20.0;
                c.nogravity = true;
                c.cffcount += 30.0;
                COLOR_C
            }
            _ => {
                self.control.cffcount += 30.0;
                COLOR_D
            }
        };
        self.main_change_color(&Self::color(color));
        self.taichi_change_color(Self::color(color), 40.0);
    }

    fn do_cff_boost(&mut self, power: f64) -> f64 {
        if self.control.cffs == Some(0) {
            self.control.cffcount -= 10.0;
            if self.control.cffcount <= 0.0 {
                self.control.cffcount = 0.0;
                self.cff_clear();
            }
            self.taichi_goto("cffa");
            return power * 2.0;
        }
        if self.control.cffs == Some(1) && self.control.cffcount < 60.0 {
            self.control.cffcount = (self.control.cffcount + power * 0.2).min(60.0);
            self.taichi_goto("cffbc");
            return power * 0.9;
        }
        power
    }

    fn do_cffb(&mut self) {
        let c = &self.control;
        if c.vx < 2.0 && c.vy <= 0.0 && c.py < 5.0 && c.py <= -c.vy * c.vy / G && c.hitenabled && c.slowcount == 0.0 && !c.nogravity {
            self.cff_clear();
            let count = self.control.cffcount;
            self.boost(count, 45.0);
            self.control.cffcount = 0.0;
            self.taichi_cffbd();
        }
    }

    fn do_cffc(&mut self) {
        if self.control.py > 2.3 {
            let c = &mut self.control;
            c.slowcount = 0.0;
            c.nolimit = true;
            c.py = 2.3;
            c.vx = 55.0;
            c.vy = 0.0;
            self.taichi_cffc();
        } else {
            self.control.cffcount -= self.control.vx * INTERVAL / 1000.0 * 30.0;
            if self.control.cffcount <= 0.0 {
                self.vpop();
                self.speed_blind(false);
                let c = &mut self.control;
                c.nogravity = false;
                c.nolimit = false;
                c.cffcount = 0.0;
                self.cff_clear();
                self.taichi_cffc_stop();
            } else {
                self.taichi_cffc_view();
            }
        }
    }

    fn do_cffd(&mut self) {
        self.control.cffcount -= 6.0;
        if self.control.cffcount <= 0.0 {
            self.control.cffcount = 0.0;
            self.cff_clear();
        }
        self.taichi_cffd();
        let c = &mut self.control;
        c.vy = (c.vy * c.vy - 2.0 * G * c.py).max(0.0).sqrt() * 1.1;
        c.vx *= 1.1;
    }

    pub(super) fn aerial_crash(&mut self, upper: bool) {
        if let Some(n) = self.control.nanakaobj {
            self.nanaka_aerial(n, upper);
        }
    }

    pub(super) fn aerial_boost(&mut self) {
        let c = &mut self.control;
        let a = c.vy.atan2(c.vx) * 0.9;
        c.vx += 10.0 * a.cos();
        c.vy += 10.0 * a.sin();
        c.vy = -c.vy;
    }

    pub(super) fn cff(&mut self, charatype: f64) {
        if let Some(n) = self.control.nanakaobj {
            self.nanaka_cff(n, charatype);
        }
    }

    pub(super) fn slow(&mut self, frames: f64, interval: f64) {
        self.control.slowcount = frames;
        self.control.slowinterval = interval;
    }

    pub(super) fn slow2(&mut self, frames: f64, distance: f64) {
        let c = &mut self.control;
        c.slowcount = frames;
        c.slowinterval = INTERVAL.min(distance / (c.vx * c.vx + c.vy * c.vy).sqrt() / frames);
    }

    /// initchara : un personnage entre dans la pile, à une place au hasard.
    pub(super) fn init_chara(&mut self, chara: Obj, code: f64) {
        let hittarget = self.control.taichi.and_then(|t| self.child(t, "taichi_mc"));
        let face = self.control.contact.and_then(|c| self.contact_get_chara(c, code));
        if let Some(s) = self.chara.get_mut(chara) {
            s.charatype = code;
            s.hittarget = hittarget;
            s.charaface = face;
        }
        self.set_chara(chara);
    }

    pub(super) fn set_chara(&mut self, chara: Obj) {
        let len = self.control.chara_array.len();
        let i = (self.random() * (len + 1) as f64).floor() as usize;
        self.control.chara_array.insert(i.min(len), chara);
    }

    fn get_chara(&mut self, pos: f64) {
        let chara = self.control.chara_array.pop();
        self.control.lastchara = None;
        if let Some(c) = chara {
            self.chara_set_position(c, pos);
            self.control.lastchara = self.chara.get(c).map(|s| s.charatype);
        }
    }

    pub(super) fn back_effect(&mut self, effect: &str, param: Option<f64>) {
        if let Some(main) = self.control.main {
            self.main_back_effect(main, effect, param);
        }
    }

    /// moveobj : une image du vol (appelée par le onEnterFrame de main_mc).
    pub(super) fn control_move(&mut self) {
        match self.control.cffs {
            Some(1) => self.do_cffb(),
            Some(2) => self.do_cffc(),
            _ => {}
        }
        let c = &self.control;
        if c.py == 0.0 && c.slowcount == 0.0 && c.vx < 0.0001 && c.vy < 0.0001 {
            self.control.vx = 0.0;
            self.control.vy = 0.0;
            let c = &self.control;
            let (px, py) = (c.px, c.py);
            self.main_view_score(px, py, c.bestrecord == px, 0.0, 0.0);
            self.local_set();
            self.main_game_over();
            return;
        }
        let c = &mut self.control;
        c.toggleflag = !c.toggleflag;
        let dt = if c.slowcount > 0.0 {
            let dt = c.slowinterval.min(0.8533333333333334 / c.vx.max(c.vy.abs() - G * c.slowinterval));
            c.slowcount -= 1.0;
            dt
        } else if c.nolimit {
            INTERVAL
        } else {
            INTERVAL.min(0.8533333333333334 * 1f64.max(-8.2 / c.py + 2.8) / c.vx.max(c.vy.abs() - G * INTERVAL))
        };
        c.px += c.vx * dt;
        if c.nogravity {
            c.py += c.vy * dt;
        } else {
            c.py += (G * dt / 2.0 + c.vy) * dt;
            c.vy += G * dt;
            if c.py < 0.0 {
                if c.cffs == Some(3) {
                    self.do_cffd();
                } else {
                    let c = &mut self.control;
                    c.vy = (c.vy * c.vy - 2.0 * G * c.py).max(0.0).sqrt() * EY;
                    c.vx *= EX;
                    let (vx, vy) = (c.vx, c.vy);
                    self.taichi_bound(vx, vy);
                }
                self.control.py = 0.0;
                self.set_special(MISATO, false);
            }
        }
        let c = &mut self.control;
        let old_scale = c.scale;
        c.scale = ((205.13 / c.py).max(20.0).min((c.scale - 2.0).max(75.0)) / 0.1).floor() * 0.1;
        c.r += (c.vx * dt * 30.0).min(30.0);
        if c.px + 20.0 >= c.charapos {
            c.charapos += 10.0;
            let pos = c.charapos;
            self.get_chara(pos);
        }
        let c = &mut self.control;
        c.bestrecord = c.bestrecord.max(c.px);
        if c.toggleflag {
            let (px, py, vx, vy, best) = (c.px, c.py, c.vx, c.vy, c.bestrecord == c.px);
            self.main_view_score(px, py, best, vx, vy);
        }
        let c = &self.control;
        let (status, fall) = (c.py <= 10.0 && c.py >= 3.0 && c.hitenabled, c.vy <= 0.0);
        self.aerial_view(status, fall, dt);
        let c = &self.control;
        let (px, py, r, scale) = (c.px, c.py, super::as2::round(c.r), c.scale);
        self.main_move(px, py, r, scale, scale != old_scale);
    }
}

impl Control {
    pub fn initialize(&mut self) {
        self.vx = 0.0;
        self.vy = 0.0;
        self.px = 0.0;
        self.py = 0.5;
        self.r = 0.0;
        self.scale = 100.0;
        self.toggleflag = true;
        self.slowcount = 0.0;
        self.charapos = 0.0;
        self.chara_array.clear();
        self.youko = false;
        self.nanaka = false;
        self.misato = false;
        self.touko = false;
        self.combo = false;
        self.kiri = false;
        self.miki = false;
        self.lastchara = None;
        self.cffcount = 0.0;
        self.cffs = None;
        self.nogravity = false;
        self.nolimit = false;
        self.hitenabled = true;
    }
}
