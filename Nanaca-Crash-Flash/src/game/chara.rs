//! Les personnages : GameCharacter et les sept personnages croisés en vol
//! (Kiri, Miki, Misato, Sakuraba, Tomoki, Touko, Youko), le blocage de
//! Youko (clip 334), Nanaka (clip 310) et Taichi (clip 328).

use std::collections::HashMap;

use super::control::{KIRI, MIKI, MISATO, NANAKA, TOUKO, YOUKO};
use super::{ColorObject, Flow, Game, Handler};
use crate::display::Obj;
use crate::geom::Matrix;
use crate::library::Id;

const PI: f64 = std::f64::consts::PI;

/// Les variables d'un personnage (NaN : undefined).
pub struct Chara {
    pub charatype: f64,
    pub hittarget: Option<Obj>,
    pub charaface: Option<Obj>,
    /// code = control.characode, posé par la première image.
    code: bool,
    hitx: f64,
    hity: f64,
    power: f64,
    angle: f64,
    s_power: f64,
    s_angle: f64,
    e: f64,
    crashmaxx: f64,
    blockmaxx: f64,
    specialmaxx: f64,
    combomaxx: f64,
    px: f64,
    busy: bool,
    active: bool,
    pass: bool,
    specialwait: bool,
    toggleflag: bool,
    standminx: f64,
    standmaxx: f64,
    viewmaxx: f64,
    hitminx: f64,
    hitmaxx: f64,
    hitmaxy: f64,
    movecount: f64,
}

impl Default for Chara {
    fn default() -> Chara {
        let n = f64::NAN;
        Chara {
            charatype: n,
            hittarget: None,
            charaface: None,
            code: false,
            hitx: n,
            hity: n,
            power: n,
            angle: n,
            s_power: n,
            s_angle: n,
            e: n,
            crashmaxx: n,
            blockmaxx: n,
            specialmaxx: n,
            combomaxx: n,
            px: n,
            busy: false,
            active: false,
            pass: false,
            specialwait: false,
            toggleflag: false,
            standminx: n,
            standmaxx: n,
            viewmaxx: n,
            hitminx: n,
            hitmaxx: n,
            hitmaxy: n,
            movecount: n,
        }
    }
}

/// Les variables de Nanaka.
struct Nanaka {
    px: f64,
    maxx: f64,
    movecount: f64,
    cfftype: Option<f64>,
    power: f64,
    angle: f64,
}

/// Les variables de Taichi (clip 328).
#[derive(Default)]
struct Taichi {
    /// count de l'effet C.F.F. en cours (None : null).
    count: Option<f64>,
    rv: bool,
    cv: bool,
}

#[derive(Default)]
pub struct Charas {
    list: HashMap<Obj, Chara>,
    nanaka: HashMap<Obj, Nanaka>,
    taichi: HashMap<Obj, Taichi>,
    /// cfftype du blocage de Youko.
    block: HashMap<Obj, f64>,
    /// dx, dy, dr posés sur les Taichi des animations.
    moves: HashMap<Obj, [f64; 3]>,
    /// Les fonctions moveobjsub (personnages et Nanaka).
    subs: HashMap<Obj, Handler>,
}

impl Charas {
    pub fn get(&self, o: Obj) -> Option<&Chara> {
        self.list.get(&o)
    }

    pub fn get_mut(&mut self, o: Obj) -> Option<&mut Chara> {
        self.list.get_mut(&o)
    }
}

impl Game {
    pub(super) fn chara_frame(&mut self, o: Obj, id: Option<Id>, frame: u16) -> bool {
        match id {
            Some(343) => self.kiri_script(o, frame),
            Some(350) => self.miki_script(o, frame),
            Some(369) => self.misato_script(o, frame),
            Some(388) => self.sakuraba_script(o, frame),
            Some(399) => self.tomoki_script(o, frame),
            Some(421) => self.touko_script(o, frame),
            Some(428) => self.youko_script(o, frame),
            Some(334) => self.block_script(o, frame),
            Some(310) => self.nanaka_script(o, frame),
            Some(328) => self.taichi_script(o, frame),
            Some(352) => {
                if frame == 13 {
                    self.stop(o);
                }
            }
            _ => return false,
        }
        true
    }

    pub(super) fn chara_unload(&mut self, o: Obj) {
        let c = &mut self.chara;
        c.list.remove(&o);
        c.nanaka.remove(&o);
        c.taichi.remove(&o);
        c.block.remove(&o);
        c.moves.remove(&o);
        c.subs.remove(&o);
    }

    // --- GameCharacter --------------------------------------------------------------

    pub(super) fn character_constructor(&mut self, o: Obj) {
        self.d.set_x(o, 5000.0);
        self.d.set_y(o, 0.0);
        self.d.set_visible(o, false);
        let b = self.d.bounds(o, Matrix::IDENTITY);
        let c = self.chara.list.entry(o).or_default();
        c.toggleflag = false;
        c.pass = false;
        c.active = true;
        c.specialwait = false;
        c.standminx = f64::from(b.x_min);
        c.standmaxx = f64::from(b.x_max);
    }

    fn chara_var(&mut self, o: Obj) -> &mut Chara {
        self.chara.list.entry(o).or_default()
    }

    /// this.code.X : la valeur, ou undefined (NaN) si code n'est pas posé.
    fn code(&self, o: Obj, value: f64) -> f64 {
        if self.chara.list.get(&o).is_some_and(|c| c.code) { value } else { f64::NAN }
    }

    /// setposition : le personnage attend Taichi à cette distance.
    pub(super) fn chara_set_position(&mut self, o: Obj, position: f64) {
        let c = self.chara_var(o);
        c.busy = true;
        c.active = true;
        c.pass = false;
        c.px = position;
        c.hitminx = position + c.hitx - 0.5;
        c.hitmaxx = position + c.hitx + 0.5;
        c.hitmaxy = c.hity + 0.5;
        c.viewmaxx = c.standmaxx;
        self.goto_and_stop(o, "stand");
        self.chara_stand(o);
    }

    /// stand (Kiri et Miki seulement).
    fn chara_stand(&mut self, o: Obj) {
        let last = self.control.lastchara;
        match self.d.get(o).and_then(|x| x.character) {
            Some(343) if last == Some(self.code(o, MIKI)) => self.set_special(self.code(o, MIKI), true),
            Some(350) if last == Some(self.code(o, KIRI)) => self.set_special(self.code(o, KIRI), true),
            _ => {}
        }
    }

    /// passact (Kiri et Miki seulement).
    fn chara_passact(&mut self, o: Obj) {
        match self.d.get(o).and_then(|x| x.character) {
            Some(343) => self.set_special(self.code(o, KIRI), false),
            Some(350) => self.set_special(self.code(o, MIKI), false),
            _ => {}
        }
    }

    /// moveobjsub, s'il est posé.
    fn moveobjsub(&mut self, o: Obj) {
        let Some(mut f) = self.chara.subs.remove(&o) else { return };
        if let Flow::Keep = f(self, o)
            && !self.chara.subs.contains_key(&o)
        {
            self.chara.subs.insert(o, f);
        }
    }

    fn set_sub(&mut self, o: Obj, f: impl FnMut(&mut Game, Obj) -> Flow + 'static) {
        self.chara.subs.insert(o, Box::new(f));
    }

    /// GameCharacter.moveobj : la place du personnage et sa rencontre avec
    /// Taichi.
    pub(super) fn character_move(&mut self, o: Obj, posx: f64, posy: f64, scale: f64, xmin: f64, xmax: f64) {
        let Some(c) = self.chara.list.get_mut(&o) else { return };
        c.toggleflag = !c.toggleflag;
        if !c.busy {
            return;
        }
        let (px, toggle, face) = (c.px, c.toggleflag, c.charaface);
        self.d.set_x(o, (px - posx) * 150.0);
        let x = self.d.x(o);
        let c = &self.chara.list[&o];
        if c.pass {
            if let Some(f) = face {
                self.d.set_visible(f, false);
            }
            if x + c.viewmaxx > xmin {
                self.d.set_visible(o, true);
                self.moveobjsub(o);
            } else {
                self.d.set_visible(o, false);
                if x <= -1350.0 {
                    let charatype = c.charatype;
                    if charatype != self.code(o, YOUKO) || !self.control.youko {
                        self.chara_var(o).busy = false;
                        self.set_chara(o);
                    }
                }
            }
            return;
        }
        if toggle
            && px - posx < 29.0
            && let Some(f) = face
        {
            self.face_view(f, x, scale);
        }
        if x + self.chara.list[&o].standminx < xmax {
            self.d.set_visible(o, true);
            self.moveobjsub(o);
            let c = &self.chara.list[&o];
            if px - posx < -1.0 {
                self.chara_var(o).pass = true;
                self.set_special(self.code(o, NANAKA), px % 100.0 == 90.0);
                self.chara_passact(o);
            } else if c.specialwait {
                match self.special_status() {
                    s if s == 0.0 => {
                        self.chara_var(o).specialwait = false;
                        self.play(o);
                    }
                    s if s == 1.0 => {
                        let c = self.chara_var(o);
                        c.specialwait = false;
                        c.viewmaxx = c.specialmaxx;
                        self.goto_and_play(o, "special");
                    }
                    _ => {}
                }
            } else if c.active && c.hitminx < posx && c.hitmaxx > posx && c.hitmaxy > posy && self.control.hitenabled {
                self.chara_var(o).active = false;
                self.slow(1000.0, 0.0);
                let label = if self.control.combo {
                    self.control.combo = false;
                    let c = self.chara_var(o);
                    c.viewmaxx = c.combomaxx;
                    "combo"
                } else if self.control.youko {
                    let c = self.chara_var(o);
                    c.viewmaxx = c.blockmaxx;
                    "block"
                } else {
                    let c = self.chara_var(o);
                    c.viewmaxx = c.crashmaxx;
                    "crash"
                };
                self.goto_and_play(o, label);
            }
        } else {
            self.d.set_visible(o, false);
        }
    }

    // --- Outils des scripts de personnages ------------------------------------------

    /// Les variables de la première image d'un personnage.
    #[allow(clippy::too_many_arguments)]
    fn chara_init(&mut self, o: Obj, hity: f64, hitx: f64, powers: [f64; 4], maxx: [f64; 4]) {
        let c = self.chara_var(o);
        c.code = true;
        c.hity = hity;
        c.hitx = hitx;
        [c.power, c.angle, c.s_power, c.s_angle] = powers;
        [c.crashmaxx, c.blockmaxx, c.specialmaxx, c.combomaxx] = maxx;
    }

    fn hittarget(&self, o: Obj) -> Option<Obj> {
        self.chara.list.get(&o).and_then(|c| c.hittarget)
    }

    fn chara_px(&self, o: Obj) -> f64 {
        self.chara.list.get(&o).map_or(f64::NAN, |c| c.px)
    }

    /// new Color(cible).setTransform(new Color(hittarget).getTransform()).
    fn copy_hit_color(&mut self, o: Obj, target: Option<Obj>) {
        let (Some(t), Some(h)) = (target, self.hittarget(o)) else { return };
        let c = self.color_transform(h);
        self.set_color_transform(t, &c);
    }

    fn set_hit_visible(&mut self, o: Obj, v: bool) {
        if let Some(h) = self.hittarget(o) {
            self.d.set_visible(h, v);
        }
    }

    /// youko_mc.cfftype = charatype (le blocage de Youko).
    fn set_block_type(&mut self, o: Obj) {
        let t = self.chara.list.get(&o).map_or(f64::NAN, |c| c.charatype);
        if let Some(b) = self.child(o, "youko_mc") {
            self.chara.block.insert(b, t);
        }
    }

    /// La pose « crash » : le joueur peut lancer l'attaque spéciale.
    fn special_wait(&mut self, o: Obj, allowed: bool) {
        if allowed {
            self.special_start();
            self.chara_var(o).specialwait = true;
        } else {
            self.play(o);
        }
    }

    fn move_var(&self, t: Obj) -> [f64; 3] {
        self.chara.moves.get(&t).copied().unwrap_or([f64::NAN; 3])
    }

    fn set_move(&mut self, t: Option<Obj>, dx: f64, dy: f64, dr: f64) {
        if let Some(t) = t {
            self.chara.moves.insert(t, [dx, dy, dr]);
        }
    }

    /// taichi_mc._x += dx ; _y += dy ; _rotation += dr.
    fn taichi_step(&mut self, o: Obj) {
        let Some(t) = self.child(o, "taichi_mc") else { return };
        let [dx, dy, dr] = self.move_var(t);
        let (x, y, r) = (self.d.x(t), self.d.y(t), self.d.rotation(t));
        self.d.set_x(t, x + dx);
        self.d.set_y(t, y + dy);
        self.d.set_rotation(t, r + dr);
    }

    fn add_xy(&mut self, t: Option<Obj>, dx: f64, dy: f64) {
        if let Some(t) = t {
            let (x, y) = (self.d.x(t), self.d.y(t));
            self.d.set_x(t, x + dx);
            self.d.set_y(t, y + dy);
        }
    }

    /// Une copie de position (a = b), avec rotation ou échelle si demandé ;
    /// rien si le clip visé n'existe pas, NaN (ignoré) si la source manque.
    fn follow(&mut self, o: Obj, to: &str, from: &str, extra: Extra) {
        let Some(t) = self.child(o, to) else { return };
        let f = self.child(o, from);
        let get = |g: &Game, k: fn(&Game, Obj) -> f64| f.map_or(f64::NAN, |f| k(g, f));
        let (y, x) = (get(self, |g, f| g.d.y(f)), get(self, |g, f| g.d.x(f)));
        self.d.set_y(t, y);
        self.d.set_x(t, x);
        match extra {
            Extra::None => {}
            Extra::Rotation => {
                let r = get(self, |g, f| g.d.rotation(f));
                self.d.set_rotation(t, r);
            }
            Extra::XScale => {
                let s = get(self, |g, f| g.d.xscale(f));
                self.d.set_xscale(t, s);
            }
        }
    }

    fn set_xy(&mut self, o: Obj, name: &str, x: Option<f64>, y: Option<f64>) {
        if let Some(c) = self.child(o, name) {
            if let Some(y) = y {
                self.d.set_y(c, y);
            }
            if let Some(x) = x {
                self.d.set_x(c, x);
            }
        }
    }

    fn movecount(&self, o: Obj) -> f64 {
        self.chara.list.get(&o).map_or(f64::NAN, |c| c.movecount)
    }

    fn set_movecount(&mut self, o: Obj, v: f64) {
        self.chara_var(o).movecount = v;
    }

    /// Le Taichi de l'animation part de la place du vrai Taichi.
    fn taichi_from_control(&mut self, o: Obj) -> Option<Obj> {
        let t = self.child(o, "taichi_mc")?;
        let (cpx, cpy, r, px) = (self.control.px, self.control.py, self.control.r, self.chara_px(o));
        self.d.set_y(t, -cpy * 150.0);
        self.d.set_x(t, (cpx - px) * 150.0);
        self.d.set_rotation(t, r);
        Some(t)
    }

    // --- Kiri (clip 343) ------------------------------------------------------------------

    fn kiri_script(&mut self, o: Obj, frame: u16) {
        let (cpx, cpy) = (self.control.px, self.control.py);
        let px = self.chara_px(o);
        let taichi = self.child(o, "taichi_mc");
        match frame {
            1 => self.chara_init(o, 0.8333333333333334, 0.16666666666666666, [10.0, 45.0, 25.0, 60.0], [451.1, 245.8, 1051.1, 714.3]),
            2 | 4 | 36 | 101 => self.stop(o),
            3 => self.set_block_type(o),
            5 => {
                self.stop(o);
                self.set_special(self.code(o, MISATO), true);
                self.set_special(self.code(o, TOUKO), false);
                let kiri = self.control.kiri;
                self.special_wait(o, kiri);
            }
            6 => self.normal_boost(o, 25.0),
            37 => {
                self.vpush();
                self.speed_blind(true);
                self.control.combo = true;
                self.copy_hit_color(o, taichi);
                self.set_hit_visible(o, false);
                self.slow(100.0, 0.01);
                self.control.vx = ((px - cpx) * 150.0 + 700.0) / 41.0 / 150.0 / 0.01;
                self.control.vy = 0.0;
                self.control.nogravity = true;
                if let Some(t) = taichi {
                    self.d.set_rotation(t, self.control.r);
                    self.d.set_y(t, -cpy * 150.0);
                    self.d.set_x(t, (cpx - px) * 150.0);
                }
                self.set_move(taichi, (550.0 - (cpx - px) * 150.0) / 41.0, (cpy * 150.0 - 100.0) / 41.0, 5.0);
                self.set_movecount(o, 0.0);
                self.set_sub(o, |g, this| {
                    g.follow(this, "kiri3_mc", "kiri2_mc", Extra::Rotation);
                    g.follow(this, "kiri2_mc", "kiri1_mc", Extra::Rotation);
                    // kiri3._visible et kiri2._visible : des noms sans clip, sans effet.
                    let mc = g.movecount(this);
                    if mc <= 12.0 {
                        g.set_xy(this, "kiri1_mc", Some(mc / 12.0 * 400.0), None);
                    } else if mc <= 23.0 {
                        g.set_xy(this, "kiri1_mc", None, Some(-(mc - 12.0) / 11.0 * 125.0));
                    } else if mc <= 34.0 {
                        let t = (mc - 23.0) / 11.0;
                        g.set_xy(this, "kiri1_mc", Some(400.0 + t * 175.0), Some(-125.0 + t * 125.0));
                        if let Some(k) = g.child(this, "kiri1_mc") {
                            g.d.set_rotation(k, t * 360.0);
                        }
                        g.circle(this, "kiri1_mc", 23.0, 11.0);
                    }
                    g.taichi_step(this);
                    g.set_movecount(this, mc + 1.0);
                    Flow::Keep
                });
            }
            38 => self.add_xy(taichi, 50.0, 0.0),
            42 | 46 | 50 | 71 => self.add_xy(taichi, 25.0, 0.0),
            78 => {
                self.control.nogravity = false;
                self.control.vx = 30.0;
                self.control.vy = 1.0;
                self.slow2(20.0, 1.0);
                self.taichi_release(o, taichi, 0.0);
            }
            102 => {
                self.set_special(self.code(o, MISATO), true);
                self.copy_hit_color(o, taichi);
                self.set_hit_visible(o, false);
                self.taichi_from_control(o);
                self.set_move(taichi, (250.0 - (cpx - px) * 150.0) / 24.0, (cpy * 150.0 - 300.0) / 24.0, 15.0);
                self.back_effect("combo", None);
                self.set_movecount(o, 0.0);
                self.set_sub(o, |g, this| {
                    g.follow(this, "kiri3_mc", "kiri2_mc", Extra::None);
                    g.follow(this, "kiri2_mc", "kiri1_mc", Extra::None);
                    g.follow(this, "miki3_mc", "miki2_mc", Extra::None);
                    g.follow(this, "miki2_mc", "miki1_mc", Extra::None);
                    let mc = g.movecount(this);
                    if mc <= 40.0 {
                        g.set_xy(this, "kiri1_mc", Some(mc * 350.0 / 40.0), Some(((mc - 20.0) * (mc - 20.0) / 400.0 - 1.0) * 175.0));
                    }
                    if mc >= 6.0 {
                        if mc <= 17.0 {
                            let t = (mc - 6.0) / 11.0;
                            g.set_xy(this, "miki1_mc", Some(-375.0 + t * 375.0), Some(-125.0 + t * 125.0));
                        } else if mc <= 25.0 {
                            let t = (mc - 17.0) / 8.0;
                            g.set_xy(this, "miki1_mc", Some(t * 275.0), Some(-t * 275.0));
                        } else if mc <= 34.0 {
                            let t = (mc - 25.0) / 9.0;
                            g.set_xy(this, "miki1_mc", Some(275.0 + t * 75.0), Some(-275.0 - t * 25.0));
                        } else if mc <= 55.0 {
                            g.set_xy(this, "miki1_mc", Some(350.0 + (mc - 34.0) / 21.0 * 125.0), Some(-300.0 + (mc - 34.0) * (mc - 34.0) / 441.0 * 300.0));
                        }
                    }
                    g.taichi_step(this);
                    g.set_movecount(this, mc + 1.0);
                    Flow::Keep
                });
            }
            127 => self.combo_finish_start(o, taichi, 3.235294117647059),
            144 => self.combo_finish(o, taichi),
            162 => {
                self.stop(o);
                self.chara.subs.remove(&o);
            }
            _ => {}
        }
    }

    /// Le tour en cercle de Kiri : y += -75 + r·sin(…), x += -25 + r·cos(…).
    fn circle(&mut self, o: Obj, name: &str, start: f64, length: f64) {
        let Some(k) = self.child(o, name) else { return };
        let mc = self.movecount(o);
        let a = 1.2490457723982544 + 6.283185307179586 * (mc - start) / length;
        let y = self.d.y(k) + -75.0 + 79.05694150420949 * a.sin();
        self.d.set_y(k, y);
        let x = self.d.x(k) + -25.0 + 79.05694150420949 * a.cos();
        self.d.set_x(k, x);
    }

    /// La poussée simple d'un personnage (frame 6 de Kiri, Miki, Touko).
    fn normal_boost(&mut self, o: Obj, frames: f64) {
        let (power, angle) = self.chara.list.get(&o).map_or((f64::NAN, f64::NAN), |c| (c.power, c.angle));
        self.boost(power, angle);
        self.slow2(frames, 1.0);
    }

    /// Fin de l'attaque spéciale : Taichi reprend sa place (py, r) et
    /// l'animation s'arrête.
    fn taichi_release(&mut self, o: Obj, taichi: Option<Obj>, extra_r: f64) {
        if let Some(t) = taichi {
            self.control.py = -self.d.y(t) / 150.0;
            self.control.r = self.d.rotation(t) + extra_r;
        }
        self.set_hit_visible(o, true);
        if let Some(t) = taichi {
            self.d.set_visible(t, false);
        }
        self.chara.subs.remove(&o);
    }

    /// Le ralenti avant le coup final d'un combo (Kiri 127, Miki 120).
    fn combo_finish_start(&mut self, o: Obj, taichi: Option<Obj>, dx: f64) {
        let px = self.chara_px(o);
        self.slow(30.0, 0.002);
        self.control.vx = (px + 2.1666666666666665 - self.control.px) / 0.034;
        self.control.vy = 0.0;
        self.control.nogravity = true;
        self.control.py = 2.3333333333333335;
        self.add_xy(taichi, 20.0, -20.0);
        self.set_move(taichi, dx, -1.7647058823529411, 3.0);
    }

    /// Le coup final d'un combo (Kiri 144, Miki 137).
    fn combo_finish(&mut self, o: Obj, taichi: Option<Obj>) {
        let (s_power, s_angle) = self.chara.list.get(&o).map_or((f64::NAN, f64::NAN), |c| (c.s_power, c.s_angle));
        self.vpop();
        self.boost(s_power, s_angle);
        self.speed_blind(false);
        self.control.nogravity = false;
        self.set_hit_visible(o, true);
        if let Some(t) = taichi {
            self.control.r = self.d.rotation(t);
            self.d.set_visible(t, false);
        }
    }

    // --- Miki (clip 350) ------------------------------------------------------------------

    fn miki_script(&mut self, o: Obj, frame: u16) {
        let (cpx, cpy) = (self.control.px, self.control.py);
        let px = self.chara_px(o);
        let taichi = self.child(o, "taichi_mc");
        match frame {
            1 => self.chara_init(o, 0.8333333333333334, 0.0, [11.0, 70.0, 24.0, 45.0], [288.6, 245.8, 726.1, 714.3]),
            2 | 4 | 36 | 94 => self.stop(o),
            3 => self.set_block_type(o),
            5 => {
                self.stop(o);
                self.set_special(self.code(o, MISATO), true);
                self.set_special(self.code(o, TOUKO), false);
                let miki = self.control.miki;
                self.special_wait(o, miki);
            }
            6 => self.normal_boost(o, 25.0),
            37 => {
                self.vpush();
                self.speed_blind(true);
                self.control.combo = true;
                self.set_hit_visible(o, false);
                self.slow(100.0, 0.01);
                self.control.vx = ((px - cpx) * 150.0 + 250.0) / 30.0 / 150.0 / 0.01;
                self.control.vy = 0.0;
                self.control.nogravity = true;
                self.copy_hit_color(o, taichi);
                if let Some(t) = taichi {
                    self.d.set_y(t, -cpy * 150.0);
                    self.d.set_rotation(t, self.control.r);
                    self.d.set_x(t, (cpx - px) * 150.0);
                    let tx = self.d.x(t);
                    self.set_move(taichi, (225.0 - tx) / 30.0, (cpy * 150.0 - 525.0) / 13.0, 15.0);
                }
                self.set_movecount(o, 0.0);
                self.set_sub(o, |g, this| {
                    g.follow(this, "miki3_mc", "miki2_mc", Extra::XScale);
                    g.follow(this, "miki2_mc", "miki1_mc", Extra::XScale);
                    let mc = g.movecount(this);
                    let m1 = g.child(this, "miki1_mc");
                    if mc <= 14.0 {
                        g.set_xy(this, "miki1_mc", None, Some(-mc / 14.0 * 450.0));
                        if let Some(m) = m1 {
                            g.d.set_xscale(m, 100.0 * (mc / 14.0 * 2.0 * PI).cos());
                        }
                    } else if mc <= 23.0 {
                        g.set_xy(this, "miki1_mc", None, Some(-450.0 + (mc - 14.0) / 9.0 * 450.0));
                    } else if mc <= 31.0 {
                        let t = (mc - 23.0) / 8.0;
                        g.set_xy(this, "miki1_mc", Some(t * 300.0), Some(-t * 175.0));
                        if let Some(m) = m1 {
                            g.d.set_xscale(m, -100.0);
                        }
                    }
                    g.taichi_step(this);
                    g.set_movecount(this, mc + 1.0);
                    Flow::Keep
                });
            }
            39 => self.add_xy(taichi, 0.0, -50.0),
            42 | 47 => self.add_xy(taichi, 0.0, -25.0),
            51 | 60 => {
                if let Some(t) = taichi {
                    let [dx, _, dr] = self.move_var(t);
                    self.set_move(taichi, dx, if frame == 51 { 58.333333333333336 } else { 0.0 }, dr);
                }
            }
            68 => {
                self.add_xy(taichi, 25.0, 0.0);
                self.control.nogravity = false;
                self.control.vx = 30.0;
                self.control.vy = 2.0;
                self.slow2(20.0, 1.0);
                self.taichi_release(o, taichi, 5.0);
            }
            95 => {
                self.set_special(self.code(o, MISATO), true);
                self.copy_hit_color(o, taichi);
                self.set_hit_visible(o, false);
                self.taichi_from_control(o);
                self.set_move(taichi, (275.0 - (cpx - px) * 150.0) / 24.0, (cpy * 150.0 - 300.0) / 24.0, 15.0);
                self.back_effect("combo", None);
                self.set_movecount(o, 0.0);
                self.set_sub(o, |g, this| {
                    g.follow(this, "miki3_mc", "miki2_mc", Extra::None);
                    g.follow(this, "miki2_mc", "miki1_mc", Extra::None);
                    g.follow(this, "kiri3_mc", "kiri2_mc", Extra::Rotation);
                    g.follow(this, "kiri2_mc", "kiri1_mc", Extra::Rotation);
                    let mc = g.movecount(this);
                    if mc <= 40.0 {
                        g.set_xy(this, "miki1_mc", Some(mc * 200.0 / 40.0), Some(((mc - 20.0) * (mc - 20.0) / 400.0 - 1.0) * 275.0));
                    }
                    if mc >= 6.0 {
                        if mc <= 17.0 {
                            let t = (mc - 6.0) / 11.0;
                            g.set_xy(this, "kiri1_mc", Some(-400.0 + t * 400.0), Some(-150.0 + t * 150.0));
                        } else if mc <= 25.0 {
                            let t = (mc - 17.0) / 8.0;
                            g.set_xy(this, "kiri1_mc", Some(t * 275.0), Some(-t * 275.0));
                            if let Some(k) = g.child(this, "kiri1_mc") {
                                g.d.set_rotation(k, t * 360.0);
                            }
                            g.circle(this, "kiri1_mc", 17.0, 8.0);
                        } else if mc <= 34.0 {
                            let t = (mc - 25.0) / 9.0;
                            g.set_xy(this, "kiri1_mc", Some(275.0 + t * 75.0), Some(-275.0 - t * 25.0));
                        } else if mc <= 56.0 {
                            g.set_xy(this, "kiri1_mc", Some(350.0 + (mc - 34.0) / 22.0 * 150.0), Some(-300.0 + (mc - 34.0) * (mc - 34.0) / 484.0 * 300.0));
                        }
                    }
                    g.taichi_step(this);
                    g.set_movecount(this, mc + 1.0);
                    Flow::Keep
                });
            }
            120 => self.combo_finish_start(o, taichi, 1.7647058823529411),
            137 => self.combo_finish(o, taichi),
            156 => {
                self.stop(o);
                self.chara.subs.remove(&o);
            }
            _ => {}
        }
    }

    // --- Misato (clip 369) ----------------------------------------------------------------

    fn misato_script(&mut self, o: Obj, frame: u16) {
        let taichi = self.child(o, "taichi_mc");
        let misato_taichi = self.child(o, "misato_mc").and_then(|m| self.child(m, "taichi_mc"));
        match frame {
            1 => {
                self.chara_init(o, 0.6666666666666666, -0.08333333333333333, [f64::NAN, f64::NAN, 35.0, 45.0], [-60.0, 245.8, 664.3, f64::NAN]);
            }
            2 | 4 | 15 | 22 | 164 => self.stop(o),
            3 => self.set_block_type(o),
            5 => {
                self.stop(o);
                self.copy_hit_color(o, misato_taichi);
                self.slow(1000.0, 0.0);
                self.set_hit_visible(o, false);
                self.set_special(self.code(o, TOUKO), false);
                let misato = self.control.misato;
                self.special_wait(o, misato);
            }
            14 => {
                self.control.vx = 0.0;
                self.control.vy = 0.0;
                self.control.py = 0.0;
                self.slow(0.0, 0.03333333333333333);
                if self.control.cffs == Some(1) {
                    self.set_hit_visible(o, true);
                    if let Some(s) = self.child(o, "stop_mc") {
                        self.d.set_visible(s, false);
                    }
                    self.goto_and_play(o, "cffb");
                }
            }
            23 => {
                self.slow(1000.0, 0.0);
                self.set_special(self.code(o, MISATO), true);
                self.copy_hit_color(o, misato_taichi);
                self.set_movecount(o, 0.0);
                self.set_sub(o, |g, this| {
                    g.taichi_step(this);
                    let mc = g.movecount(this);
                    g.set_movecount(this, mc + 1.0);
                    Flow::Keep
                });
            }
            30 => {
                self.copy_hit_color(o, taichi);
                self.set_move(taichi, 6.0, -2.0, 7.2);
            }
            80 => self.set_move(taichi, 0.0, -75.0, 102.85714285714286),
            88 => {
                self.set_move(taichi, 0.0, 0.0, 0.0);
                self.vpush();
                self.speed_blind(true);
                let px = self.chara_px(o);
                self.control.vx = (px - self.control.px) / 39.0 / 0.01;
                if self.control.vx < 0.0 {
                    self.control.vx = 0.0;
                }
                self.control.vy = 0.0;
                self.control.nogravity = true;
                self.slow(50.0, 0.01);
            }
            127 => {
                self.set_xy(o, "taichi_mc", Some(0.0), Some(-700.0));
                self.set_move(taichi, 0.0, 87.5, 60.0);
            }
            134 => {
                let (s_power, s_angle) = self.chara.list.get(&o).map_or((f64::NAN, f64::NAN), |c| (c.s_power, c.s_angle));
                self.vpop();
                self.control.nogravity = false;
                self.boost(s_power, s_angle);
                self.speed_blind(false);
                self.set_hit_visible(o, true);
                self.control.px += 0.3;
                self.control.py += 0.3;
                self.slow2(20.0, 0.5);
                self.back_effect("misato", None);
            }
            _ => {}
        }
    }

    // --- Sakuraba (clip 388) et Tomoki (clip 399) ------------------------------------------

    fn sakuraba_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => self.chara_init(o, 1.0, -0.16666666666666666, [f64::NAN; 4], [430.0, 430.0, f64::NAN, f64::NAN]),
            2 | 43 | 76 => self.stop(o),
            3 => self.slow(1000.0, 0.0),
            11 | 44 => {
                // Le curry : Taichi repart avec un angle tiré de sa direction.
                let c = &mut self.control;
                let angle = (c.vy.atan2(c.vx).abs() / 0.017453292519943295 + 45.0) % 90.0 * 70.0 / 90.0 + 10.0;
                let v = (c.vx * c.vx + c.vy * c.vy).sqrt();
                c.vx = v * (angle * PI / 180.0).cos();
                c.vy = v * (angle * PI / 180.0).sin();
                for (name, v) in [("up_mc", angle >= 45.0), ("down_mc", angle < 45.0)] {
                    if let Some(c) = self.child(o, name) {
                        self.d.set_visible(c, v);
                    }
                }
                self.slow2(30.0, 1.0);
                if frame == 11 {
                    self.set_special(self.code(o, YOUKO), false);
                }
                self.set_movecount(o, 0.0);
                let x1a = 300.0 + 100.0 * self.random();
                let x1b = 50.0 + 300.0 * self.random();
                let x2 = 100.0 + 100.0 * self.random();
                self.set_sub(o, move |g, this| {
                    let mc = g.movecount(this) + 1.0;
                    g.set_movecount(this, mc);
                    let y = (mc - 10.0) * (mc - 10.0) / 20.0 / 20.0 * 200.0 - 200.0;
                    for (name, x, spin) in [("curry1a_mc", x1a, 5.0), ("curry1b_mc", x1b, 10.0), ("curry2_mc", x2, 15.0)] {
                        if let Some(c) = g.child(this, name) {
                            g.d.set_x(c, mc / 30.0 * x);
                            g.d.set_y(c, y);
                            let r = g.d.rotation(c);
                            g.d.set_rotation(c, r + spin);
                        }
                    }
                    if mc == 30.0 { Flow::Delete } else { Flow::Keep }
                });
            }
            _ => {}
        }
    }

    fn tomoki_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => {
                self.chara_init(o, 1.0, -0.16666666666666666, [f64::NAN; 4], [217.9, 217.9, f64::NAN, f64::NAN]);
                self.chara_var(o).e = 0.7;
            }
            2 | 41 | 72 => self.stop(o),
            3 => self.slow(1000.0, 0.0),
            11 | 42 => {
                // Tomoki amortit le vol.
                let e = self.chara.list.get(&o).map_or(f64::NAN, |c| c.e);
                self.control.vx *= e;
                self.control.vy = self.control.vy.abs() * e;
                self.slow2(30.0, 1.0);
                if frame == 11 {
                    self.set_special(self.code(o, YOUKO), false);
                }
            }
            _ => {}
        }
    }

    // --- Touko (clip 421) et Youko (clip 428) -------------------------------------------------

    fn touko_script(&mut self, o: Obj, frame: u16) {
        let (cpx, cpy) = (self.control.px, self.control.py);
        let px = self.chara_px(o);
        let taichi = self.child(o, "taichi_mc");
        match frame {
            1 => self.chara_init(o, 0.8333333333333334, 0.16666666666666666, [11.0, 20.0, 25.0, 30.0], [526.0, 245.8, 605.65, f64::NAN]),
            2 | 4 | 36 | 164 => self.stop(o),
            3 => self.set_block_type(o),
            5 => {
                self.stop(o);
                self.set_special(self.code(o, MISATO), true);
                let touko = self.control.touko;
                self.special_wait(o, touko);
                self.set_special(self.code(o, TOUKO), true);
            }
            6 => self.normal_boost(o, 25.0),
            37 => {
                self.vpush();
                self.speed_blind(true);
                self.slow(1000.0, 0.0);
                if let Some(t) = taichi {
                    self.d.set_y(t, -cpy * 150.0);
                    self.d.set_x(t, (cpx - px) * 150.0);
                }
                self.set_move(taichi, (275.0 - (cpx - px) * 150.0) / 62.0, (cpy * 150.0 - 100.0) / 62.0, 5.0);
                if let Some(t) = taichi {
                    self.d.set_rotation(t, self.control.r);
                }
                self.control.py = 0.6666666666666666;
                self.copy_hit_color(o, taichi);
                self.set_hit_visible(o, false);
                self.set_movecount(o, 0.0);
                self.set_sub(o, |g, this| {
                    g.taichi_step(this);
                    let mc = g.movecount(this);
                    g.set_movecount(this, mc + 1.0);
                    Flow::Keep
                });
            }
            100 => {
                self.slow(50.0, 0.01);
                self.control.vx = ((px - cpx) * 150.0 + 275.0) / 45.0 / 150.0 / 0.01;
                self.control.vy = 0.0;
                self.control.nogravity = true;
                self.back_effect("white", None);
            }
            145 => {
                let (s_power, s_angle) = self.chara.list.get(&o).map_or((f64::NAN, f64::NAN), |c| (c.s_power, c.s_angle));
                self.set_hit_visible(o, true);
                self.control.nogravity = false;
                self.vpop();
                self.boost(s_power, s_angle);
                self.speed_blind(false);
                self.slow2(20.0, 1.0);
                self.back_effect("metu", None);
            }
            _ => {}
        }
    }

    fn youko_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => self.chara_init(o, 0.8333333333333334, -0.16666666666666666, [f64::NAN; 4], [0.0, f64::NAN, f64::NAN, f64::NAN]),
            2 | 31 => self.stop(o),
            3 => {
                self.set_special(self.code(o, YOUKO), true);
                self.set_special(self.code(o, NANAKA), false);
                self.set_special(self.code(o, TOUKO), false);
            }
            15 => self.slow2(15.0, 1.0),
            _ => {}
        }
    }

    // --- Le blocage de Youko (clip 334) ------------------------------------------------------

    fn block_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => self.slow(1000.0, 0.0),
            5 => {
                self.stop(o);
                if self.control.nanaka {
                    self.special_start();
                    self.on_enter_frame(o, |g, this| {
                        let st = g.special_status();
                        if st == 0.0 {
                            g.play(this);
                            return Flow::Delete;
                        }
                        if st == 1.0 {
                            let t = g.chara.block.get(&this).copied().unwrap_or(f64::NAN);
                            g.cff(t);
                            g.goto_and_play(this, "special");
                            return Flow::Delete;
                        }
                        Flow::Keep
                    });
                } else {
                    self.play(o);
                }
            }
            6 => self.slow2(20.0, 1.0),
            18 => {
                self.stop(o);
                self.slow(0.0, 0.03333333333333333);
                self.set_special(YOUKO, false);
            }
            65 => self.stop(o),
            _ => {}
        }
    }

    // --- Nanaka (clip 310) -------------------------------------------------------------------

    fn nanaka_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(o);
                self.chara.nanaka.insert(o, Nanaka { px: -0.6666666666666666, maxx: 103.0, movecount: f64::NAN, cfftype: None, power: f64::NAN, angle: f64::NAN });
            }
            27 | 83 => {
                self.aerial_boost();
                self.slow2(35.0, 1.0);
                self.control.hitenabled = true;
            }
            57 | 113 => self.stop(o),
            114 => {
                if let Some(n) = self.chara.nanaka.get_mut(&o) {
                    n.power = 15.0;
                    n.angle = 45.0;
                }
                self.slow(1000.0, 0.0);
                self.set_special(MISATO, true);
            }
            149 => {
                let (power, angle, cfftype) = self.chara.nanaka.get(&o).map_or((f64::NAN, f64::NAN, None), |n| (n.power, n.angle, n.cfftype));
                self.boost(power, angle);
                self.slow2(70.0, 0.8);
                self.back_effect("cff", cfftype);
            }
            167 => {
                let kind = match self.chara.nanaka.get(&o).and_then(|n| n.cfftype) {
                    Some(MISATO) => Some(0),
                    Some(TOUKO) => Some(1),
                    Some(KIRI) => Some(2),
                    Some(MIKI) => Some(3),
                    _ => None,
                };
                if let Some(k) = kind {
                    self.set_cff(k);
                }
            }
            179 => {
                self.stop(o);
                self.set_special(YOUKO, false);
            }
            _ => {}
        }
    }

    /// Nanaka.moveobj.
    pub(super) fn nanaka_move(&mut self, n: Obj, px: f64, xmin: f64) {
        let Some(state) = self.chara.nanaka.get(&n) else { return };
        let (own, maxx) = (state.px, state.maxx);
        self.d.set_x(n, (own - px) * 150.0);
        let x = self.d.x(n);
        self.d.set_visible(n, x + maxx > xmin);
        self.moveobjsub(n);
    }

    /// Nanaka.aerial : l'attaque aérienne (vers le haut ou vers le bas).
    pub(super) fn nanaka_aerial(&mut self, n: Obj, upper: bool) {
        self.control.hitenabled = false;
        self.slow(1000.0, 0.0);
        if let Some(s) = self.chara.nanaka.get_mut(&n) {
            s.movecount = 0.0;
        }
        self.set_sub(n, move |g, this| {
            let Some(s) = g.chara.nanaka.get_mut(&this) else { return Flow::Keep };
            s.movecount += 1.0;
            let mc = s.movecount;
            g.d.set_xscale(this, (1.0 - mc / 10.0) * 100.0);
            g.d.set_yscale(this, mc / 10.0 * 200.0 + 100.0);
            if mc >= 10.0 {
                g.d.set_xscale(this, 100.0);
                g.d.set_yscale(this, 100.0);
                g.d.set_rotation(this, 0.0);
                let (cpx, cpy) = (g.control.px, g.control.py);
                g.d.set_x(this, 0.0);
                g.d.set_y(this, -cpy * 150.0);
                if let Some(s) = g.chara.nanaka.get_mut(&this) {
                    s.px = cpx;
                    s.maxx = 0.0;
                }
                g.goto_and_play(this, if upper { "aerialu" } else { "aeriald" });
                return Flow::Delete;
            }
            Flow::Keep
        });
    }

    /// Nanaka.cff : l'attaque spéciale de Nanaka avec un personnage.
    pub(super) fn nanaka_cff(&mut self, n: Obj, typecode: f64) {
        self.d.set_rotation(n, 0.0);
        let cpx = self.control.px;
        self.d.set_x(n, 0.0);
        self.d.set_y(n, 0.0);
        if let Some(s) = self.chara.nanaka.get_mut(&n) {
            s.px = cpx;
            s.cfftype = Some(typecode);
            s.maxx = 401.1;
        }
        self.goto_and_play(n, "special");
    }

    // --- Taichi (clip 328) ---------------------------------------------------------------------

    fn taichi_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => {
                self.chara.taichi.insert(o, Taichi::default());
            }
            2 | 18 | 33 | 78 | 94 | 136 => self.stop(o),
            35 | 41 => self.control.py += 0.2,
            47 => {
                self.slow2(30.0, 1.0);
                self.control.hitenabled = true;
                self.control.py += 0.2;
            }
            79 => {
                self.stop(o);
                if let Some(t) = self.child(o, "count_txt") {
                    self.d.set_autosize_right(t);
                }
            }
            80 => {
                self.slow(14.0, 0.0);
                self.control.hitenabled = true;
                if let Some(t) = self.chara.taichi.get_mut(&o) {
                    t.count = Some(0.0);
                }
                self.d.set_xscale(o, 100.0);
                self.d.set_yscale(o, 100.0);
                if let Some(b) = self.child(o, "boost_mc") {
                    self.d.set_xscale(b, 70.0);
                    self.d.set_yscale(b, 70.0);
                    self.d.set_x(b, -80.0);
                    self.d.set_y(b, -50.0);
                }
                self.on_enter_frame(o, |g, this| {
                    let count = g.chara.taichi.get(&this).and_then(|t| t.count);
                    let c = count.unwrap_or(0.0);
                    if c < 20.0 {
                        let c = c + 1.0;
                        if let Some(t) = g.chara.taichi.get_mut(&this) {
                            t.count = Some(c);
                        }
                        g.d.set_xscale(this, 100.0 + 50.0 * (0.3141592653589793 * c).sin());
                        g.d.set_yscale(this, 100.0 - 50.0 * (0.3141592653589793 * c).sin());
                        let (xs, ys) = (g.d.xscale(this), g.d.yscale(this));
                        if let Some(b) = g.child(this, "boost_mc") {
                            g.d.set_xscale(b, 100.0 / xs * 70.0);
                            g.d.set_yscale(b, 100.0 / ys * 70.0);
                            g.d.set_x(b, 100.0 / xs * -80.0);
                            g.d.set_y(b, 100.0 / ys * -50.0 - c * 5.0);
                        }
                        Flow::Keep
                    } else {
                        if let Some(t) = g.chara.taichi.get_mut(&this) {
                            t.count = None;
                        }
                        Flow::Delete
                    }
                });
            }
            95 => {
                let (rv, cv) = self.chara.taichi.get(&o).map_or((false, false), |t| (t.rv, t.cv));
                for (name, v) in [("r_txt", rv), ("c_txt", cv)] {
                    if let Some(t) = self.child(o, name) {
                        self.d.set_visible(t, v);
                    }
                }
            }
            _ => {}
        }
    }

    /// Taichi.bound : les bruitages du rebond, selon sa force.
    pub(super) fn taichi_bound(&mut self, vx: f64, vy: f64) {
        let Some(t) = self.control.taichi else { return };
        if let Some(se) = self.child(t, "se1_mc") {
            self.se_play(se, (vx * 30.0).min(100.0));
        }
        if let Some(se) = self.child(t, "se2_mc") {
            self.se_play(se, (vy * 30.0).min(100.0));
        }
    }

    /// Taichi.moveobj : sa hauteur, sa rotation ; hors de l'écran au-delà
    /// de 14 m.
    pub(super) fn taichi_move(&mut self, t: Obj, py: f64, rot: f64, scale: f64) {
        if let Some(fly) = self.child(t, "taichi_mc")
            && self.d.rotation(fly) != rot
        {
            self.d.set_rotation(fly, rot);
        }
        if py > 14.0 {
            self.d.set_visible(t, false);
        } else {
            if py > 2.735042735042735 && py < 10.256410256410257 {
                self.d.set_y(t, -100.0 / scale * 400.0 / 1.3);
            } else {
                self.d.set_y(t, -py * 150.0);
            }
            self.d.set_visible(t, true);
        }
    }

    /// Taichi.changecolor : sa teinte change en `frames` images.
    pub(super) fn taichi_change_color(&mut self, new: ColorObject, frames: f64) {
        let Some(fly) = self.control.taichi.and_then(|t| self.child(t, "taichi_mc")) else { return };
        let old = self.color_transform(fly);
        let mut count = 0.0;
        self.on_enter_frame(fly, move |g, this| {
            if frames > count {
                count += 1.0;
                let k = count / frames;
                let mix = |n: Option<f64>, o: Option<f64>| Some(n.unwrap_or(f64::NAN) * k + o.unwrap_or(f64::NAN) * (1.0 - k));
                let c = ColorObject {
                    ra: mix(new.ra, old.ra),
                    rb: mix(new.rb, old.rb),
                    ga: mix(new.ga, old.ga),
                    gb: mix(new.gb, old.gb),
                    ba: mix(new.ba, old.ba),
                    bb: mix(new.bb, old.bb),
                    aa: mix(new.aa, old.aa),
                    ab: mix(new.ab, old.ab),
                };
                g.set_color_transform(this, &c);
                Flow::Keep
            } else {
                Flow::Delete
            }
        });
    }

    /// Taichi.setcff : l'annonce d'une attaque spéciale.
    pub(super) fn taichi_set_cff(&mut self, old: Option<u8>, new: u8, _power: f64) {
        let Some(t) = self.control.taichi else { return };
        self.goto_and_play(t, "cff");
        let s = self.chara.taichi.entry(t).or_default();
        s.rv = old.is_some();
        s.cv = old.is_some_and(|o| o != new);
    }

    /// cffa, cffbc : une étiquette de Taichi.
    pub(super) fn taichi_goto(&mut self, label: &str) {
        if let Some(t) = self.control.taichi {
            self.goto_and_play(t, label);
        }
    }

    pub(super) fn taichi_cffbd(&mut self) {
        self.control.hitenabled = false;
        self.slow(1000.0, 0.0);
        self.taichi_goto("cffbd");
    }

    pub(super) fn taichi_cffc(&mut self) {
        if let Some(t) = self.control.taichi {
            self.chara.taichi.entry(t).or_default().count = Some(0.0);
        }
        self.taichi_goto("cffc");
    }

    pub(super) fn taichi_cffd(&mut self) {
        self.control.hitenabled = false;
        self.taichi_goto("cffd");
    }

    pub(super) fn taichi_cffc_stop(&mut self) {
        let Some(t) = self.control.taichi else { return };
        self.chara.taichi.entry(t).or_default().count = None;
        if let Some(fly) = self.child(t, "taichi_mc") {
            self.d.set_y(fly, 0.0);
        }
        self.goto_and_stop(t, "normal");
    }

    /// cffcview : Taichi ondule, la réserve s'affiche, les éclairs filent.
    pub(super) fn taichi_cffc_view(&mut self) {
        let Some(t) = self.control.taichi else { return };
        let Some(count) = self.chara.taichi.get(&t).and_then(|s| s.count) else { return };
        let count = count + 1.0;
        self.chara.taichi.entry(t).or_default().count = Some(count);
        let fly = self.child(t, "taichi_mc");
        if let Some(f) = fly {
            self.d.set_y(f, 60.0 * (6.283185307179586 * count / 20.0).sin());
        }
        if let Some(c) = self.child(t, "count_txt") {
            let v = super::as2::round(self.control.cffcount / 30.0 * 100.0);
            self.d.set_text(c, &super::as2::number_to_string(v));
        }
        let fly_y = fly.map_or(f64::NAN, |f| self.d.y(f));
        for (name, step, limit) in [("light1_mc", 20.0, -150.0), ("light2_mc", 10.0, -100.0), ("light3_mc", 30.0, -200.0)] {
            let Some(l) = self.child(t, name) else { continue };
            let x = self.d.x(l) - step;
            self.d.set_x(l, x);
            if self.d.x(l) <= limit {
                self.d.set_x(l, 0.0);
                let r = self.random();
                self.d.set_y(l, fly_y - 50.0 + r * 100.0);
            }
        }
    }
}

/// Ce que `follow` copie en plus de la position.
#[derive(Clone, Copy)]
enum Extra {
    None,
    Rotation,
    XScale,
}
