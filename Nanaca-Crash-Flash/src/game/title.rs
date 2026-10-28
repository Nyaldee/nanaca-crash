//! L'écran titre (clip 273) : les lettres qui arrivent en sautant, les
//! boutons, les réglages de qualité et de son animés, le record, le mode
//! d'emploi et son panneau à onglets.

use std::collections::HashMap;

use super::as2::char_at;
use super::{ColorObject, Flow, Game};
use crate::display::Obj;
use crate::library::Id;

/// Les boutons que allenabled active ou désactive.
const BUTTONS: [&str; 5] = ["start_btn", "howto_btn", "quality_btn", "sound_btn", "clear_btn"];

#[derive(Default)]
pub struct Title {
    /// count et frames, variables de title_mc partagées par ses lettres.
    count: f64,
    frames: f64,
    /// Le zoom du bouton START (frame 15) : ses variables.
    start: Start,
    /// Les réglages animés (clips 98 et 69).
    quality: HashMap<Obj, Quality>,
    sound: HashMap<Obj, SoundMode>,
    /// count du mode d'emploi (clip 271).
    howto: HashMap<Obj, f64>,
    /// Le panneau à onglets (clip 269).
    panel: HashMap<Obj, Panel>,
}

#[derive(Default, Clone, Copy)]
struct Start {
    vy: f64,
    pt: f64,
    sx: f64,
    ex: f64,
    vs: f64,
}

struct Quality {
    depth: i32,
    current: &'static str,
    current_mc: Option<Obj>,
    base_btn: Option<Obj>,
}

struct SoundMode {
    depth: i32,
    on: bool,
}

#[derive(Default)]
struct Panel {
    out_mc: Option<Obj>,
    in_mc: Option<Obj>,
    current: Option<Obj>,
    /// Le panneau de chaque onglet, et la position de chaque panneau.
    target: HashMap<Obj, Obj>,
    pos: HashMap<Obj, f64>,
}

/// frames et max de viewslide.
const SLIDE_FRAMES: f64 = 5.0;
const SLIDE_MAX: f64 = 10.0;

impl Game {
    pub(super) fn title_frame(&mut self, o: Obj, id: Option<Id>, frame: u16) -> bool {
        match id {
            Some(273) => self.title_script(o, frame),
            Some(98) => match frame {
                1 => self.quality_init(o),
                3 => self.stop(o),
                _ => {}
            },
            Some(69) => match frame {
                1 => self.sound_init(o),
                3 => self.stop(o),
                _ => {}
            },
            Some(126) => {
                if frame == 1 {
                    self.stop(o);
                    self.best_view(o);
                }
            }
            Some(271) => self.howto_script(o, frame),
            Some(269) => self.panel_init(o),
            _ => return false,
        }
        true
    }

    fn allenabled(&mut self, title: Obj, v: bool) {
        for name in BUTTONS {
            if let Some(b) = self.child(title, name) {
                self.set_enabled(b, v);
            }
        }
    }

    fn title_script(&mut self, t: Obj, frame: u16) {
        match frame {
            1 => {
                self.set_volume(Some(t), 100.0);
                self.allenabled(t, false);
                let mut sets: Vec<(Option<Obj>, Option<Obj>)> = Vec::new();
                for name in ["clear_btn", "start_btn", "title_mc", "howto_btn", "quality_btn"] {
                    let c = self.child(t, name);
                    sets.push((c, c));
                }
                let quality = self.child(t, "quality_mc");
                for name in ["h_mc", "m_mc", "l_mc"] {
                    sets.push((quality.and_then(|q| self.child(q, name)), quality));
                }
                let sound_btn = self.child(t, "sound_btn");
                sets.push((sound_btn, sound_btn));
                let sound = self.child(t, "sound_mc");
                for name in ["on_mc", "off_mc"] {
                    sets.push((sound.and_then(|s| self.child(s, name)), sound));
                }
                let best = self.child(t, "best_mc");
                sets.push((best, best));
                for (target, base) in sets {
                    let (Some(target), Some(base)) = (target, base) else { continue };
                    for letter in self.child_clips(target) {
                        self.title_hop(letter, base);
                    }
                }
                if let Some(q) = quality {
                    let btn = self.child(t, "quality_btn");
                    self.title.quality.entry(q).or_insert(Quality { depth: 0, current: "", current_mc: None, base_btn: None }).base_btn = btn;
                }
                self.on_enter_frame(t, |g, this| {
                    if g.title.count == 0.0 {
                        g.delete_mouse_down(this);
                        return Flow::Delete;
                    }
                    g.title.count -= 1.0;
                    Flow::Keep
                });
                self.on_mouse_down(t, |g, this| {
                    g.title.count = 0.0;
                    g.native_goto_and_stop(this, "introend");
                    Flow::Keep
                });
            }
            14 => {
                self.stop(t);
                if let Some(bgm) = self.child(t, "bgm_mc") {
                    self.bgm_sound_start(bgm, 1);
                }
                if let Some(h) = self.child(t, "howto_mc") {
                    self.d.set_visible(h, false);
                    self.stop(h);
                }
            }
            15 => self.title_start(t),
            _ => {}
        }
    }

    /// La fonction hop de l'écran titre, posée sur chaque lettre : un saut
    /// depuis le haut de l'écran vers sa place ; le bouton s'active à
    /// l'arrivée.
    fn title_hop(&mut self, letter: Obj, base: Obj) {
        let (py, px) = (self.d.y(letter), self.d.x(letter));
        let orgscale = self.d.yscale(letter);
        let rot = self.random() * 20.0 + 5.0;
        let (startx, starty) = (225.0 + px / 4.0, 100.0);
        self.title.frames = 40.0;
        let frames = self.title.frames;
        let peak = -self.random() * ((self.d.y(base) + starty) / 2.0 + 200.0);
        let a = -peak / (frames * frames) * 4.0;
        let v0 = peak / frames * 4.0;
        self.title.count = frames;
        let action = move |g: &mut Game, this: Obj| {
            let (count, frames) = (g.title.count, g.title.frames);
            let (bx, by) = (g.d.x(base), g.d.y(base));
            g.d.set_x(this, (startx - bx - px) * count / frames + px);
            g.d.set_y(this, a * count * count + v0 * count + (starty - by - py) * count / frames + py);
            g.d.set_xscale(this, orgscale * (1.0 - 0.5 * count / frames));
            let xs = g.d.xscale(this);
            g.d.set_yscale(this, xs);
            g.d.set_rotation(this, rot * count);
            if count == 0.0 {
                g.set_enabled(base, true);
                return Flow::Delete;
            }
            Flow::Keep
        };
        if let Flow::Keep = action(self, letter) {
            self.on_enter_frame(letter, action);
        }
    }

    /// Frame 15 : START grossit et file, la transition commence.
    fn title_start(&mut self, t: Obj) {
        self.stop(t);
        let Some(start) = self.child(t, "start_mc") else { return };
        self.stop(start);
        if let Some(b) = self.child(t, "start_btn") {
            self.d.set_visible(b, false);
        }
        if let Some(q) = self.child(t, "quality_mc") {
            self.delete_enter_frame(q);
        }
        if let Some(s) = self.child(t, "sound_mc") {
            self.delete_enter_frame(s);
        }
        self.allenabled(t, false);
        self.title.frames = 20.0;
        let pt = 47.0;
        self.title.start = Start {
            vy: -self.d.y(start) / self.title.frames,
            pt,
            sx: self.d.x(t) + self.d.x(start) + self.d.xscale(start) / 100.0 * pt,
            ex: 350.0,
            vs: 1.26,
        };
        self.title.count = 0.0;
        if let Some(changer) = self.root.changer {
            self.change_scene(changer, "start", Some(t));
        }
        self.on_enter_frame(start, move |g, this| {
            let s = g.title.start;
            g.title.count += 1.0;
            let (count, frames) = (g.title.count, g.title.frames);
            let xs = g.d.xscale(this) * s.vs;
            g.d.set_xscale(this, xs);
            let ys = g.d.yscale(this) * s.vs;
            g.d.set_yscale(this, ys);
            let title_x = g.d.x(t);
            let xs = g.d.xscale(this);
            g.d.set_x(this, (s.ex - s.sx) * count * count / frames / frames + s.sx - title_x - xs / 100.0 * s.pt);
            let y = g.d.y(this) + s.vy;
            g.d.set_y(this, y);
            if count >= frames { Flow::Delete } else { Flow::Keep }
        });
    }

    pub(super) fn title_button(&mut self, id: Id, this: Obj) -> bool {
        match id {
            80 => {
                // Le son, sur l'écran titre.
                let bgm = self.child(this, "bgm_mc");
                if self.volume(None) == 0.0 {
                    self.set_volume(None, 100.0);
                    if let Some(b) = bgm {
                        self.bgm_sound_restart(b);
                    }
                } else {
                    self.set_volume(None, 0.0);
                    if let Some(b) = bgm {
                        self.bgm_sound_stop(b);
                    }
                }
                self.local_set();
            }
            108 => {
                self.cycle_quality();
                self.local_set();
            }
            111 => {
                if let Some(h) = self.child(this, "howto_mc") {
                    self.d.set_visible(h, true);
                    self.allenabled(this, false);
                    self.goto_and_play(h, "start");
                }
            }
            114 => self.goto_and_play(this, "start"),
            118 => {
                if self.best() != "0.00" {
                    self.local_clear();
                    if let Some(b) = self.child(this, "best_mc") {
                        self.play(b);
                    }
                }
            }
            148 => {
                // Fermer le mode d'emploi.
                for name in ["about_mc", "aerial_mc", "special_mc", "chara_mc"] {
                    if let Some(c) = self.child(this, name) {
                        self.stop(c);
                        self.delete_enter_frame(c);
                    }
                }
                if let Some(p) = self.parent(this) {
                    self.goto_and_play(p, "hide");
                }
            }
            _ => return false,
        }
        true
    }

    /// _quality passe à la valeur suivante.
    pub(super) fn cycle_quality(&mut self) {
        self.quality = match self.quality {
            "HIGH" => "MEDIUM",
            "MEDIUM" => "LOW",
            _ => "HIGH",
        };
    }

    // --- Record (clip 126) -----------------------------------------------------------

    fn best_view(&mut self, o: Obj) {
        let best = self.best();
        let len = best.chars().count() as i64;
        for (name, k) in [("n1_mc", 1), ("n2_mc", 2), ("n4_mc", 4), ("n5_mc", 5), ("n6_mc", 6), ("n7_mc", 7), ("n8_mc", 8), ("n9_mc", 9), ("n10_mc", 10)] {
            if let Some(c) = self.child(o, name) {
                self.set_digit(c, Some(char_at(&best, len - k)));
            }
        }
    }

    // --- Réglages animés (clips 98 et 69) ------------------------------------------------

    fn quality_init(&mut self, o: Obj) {
        self.stop(o);
        let base_btn = self.title.quality.get(&o).and_then(|q| q.base_btn);
        for name in ["h_mc", "m_mc", "l_mc"] {
            if let Some(c) = self.child(o, name) {
                self.d.set_visible(c, false);
            }
        }
        let current = self.quality;
        let current_mc = self.quality_mc(o, current);
        if let Some(c) = current_mc {
            self.d.set_visible(c, true);
        }
        self.title.quality.insert(o, Quality { depth: 0, current, current_mc, base_btn });
        self.on_enter_frame(o, |g, this| {
            let Some(q) = g.title.quality.get(&this) else { return Flow::Keep };
            let base_enabled = q.base_btn.is_some_and(|b| g.enabled(b));
            if q.current != g.quality && base_enabled {
                let current = g.quality;
                g.native_goto_and_play(this, "change");
                let old = g.title.quality.get(&this).and_then(|q| q.current_mc);
                if let Some(old) = old {
                    g.view_out(this, old, true);
                }
                let mc = g.quality_mc(this, current);
                if let Some(q) = g.title.quality.get_mut(&this) {
                    q.current = current;
                    q.current_mc = mc;
                }
                if let Some(mc) = mc {
                    g.d.set_visible(mc, true);
                }
            }
            Flow::Keep
        });
    }

    fn quality_mc(&self, o: Obj, mode: &str) -> Option<Obj> {
        match mode {
            "HIGH" => self.child(o, "h_mc"),
            "MEDIUM" => self.child(o, "m_mc"),
            "LOW" => self.child(o, "l_mc"),
            _ => None,
        }
    }

    fn sound_init(&mut self, o: Obj) {
        self.stop(o);
        let on = self.volume(None) != 0.0;
        if let Some(c) = self.child(o, "on_mc") {
            self.d.set_visible(c, on);
        }
        if let Some(c) = self.child(o, "off_mc") {
            self.d.set_visible(c, !on);
        }
        self.title.sound.insert(o, SoundMode { depth: 0, on });
        self.on_enter_frame(o, |g, this| {
            let Some(mode) = g.title.sound.get(&this).map(|s| s.on) else { return Flow::Keep };
            if (g.volume(None) != 0.0) != mode {
                let on = !mode;
                if let Some(s) = g.title.sound.get_mut(&this) {
                    s.on = on;
                }
                g.native_goto_and_play(this, "change");
                let (out, inn) = if on { ("off_mc", "on_mc") } else { ("on_mc", "off_mc") };
                if let Some(c) = g.child(this, out) {
                    g.view_out(this, c, false);
                }
                if let Some(c) = g.child(this, inn) {
                    g.d.set_visible(c, true);
                }
            }
            Flow::Keep
        });
    }

    /// viewout : une copie du mot s'effondre lettre par lettre ; le mot
    /// est caché.
    fn view_out(&mut self, owner: Obj, target: Obj, quality: bool) {
        let depth = if quality {
            let q = self.title.quality.get_mut(&owner);
            q.map_or(1, |q| {
                q.depth = (q.depth + 1) % 100;
                q.depth
            })
        } else {
            let s = self.title.sound.get_mut(&owner);
            s.map_or(1, |s| {
                s.depth = (s.depth + 1) % 100;
                s.depth
            })
        };
        let name = super::as2::number_to_string(self.time.floor());
        let copy = self.d.duplicate(target, &name, depth);
        self.d.set_visible(target, false);
        let Some(copy) = copy else { return };
        for letter in self.child_clips(copy) {
            let (px, py) = (self.d.x(letter), self.d.y(letter));
            self.d.set_rotation(letter, 0.0);
            let r = self.random() * 20.0;
            self.d.set_x(letter, px);
            self.d.set_y(letter, py);
            let orgscale = self.d.yscale(letter);
            let mut count = 0.0;
            self.on_enter_frame(letter, move |g, this| {
                count += 1.0;
                g.d.set_x(this, px + px * count / 30.0);
                g.d.set_y(this, py + count * count / 3.0);
                g.d.set_rotation(this, r * count);
                g.d.set_xscale(this, (1.0 + count / 30.0) * orgscale);
                let xs = g.d.xscale(this);
                g.d.set_yscale(this, xs);
                if g.d.y(this) >= 200.0 {
                    g.d.set_xscale(this, 100.0);
                    g.d.set_yscale(this, 100.0);
                    g.d.set_rotation(this, 0.0);
                    g.d.set_x(this, px);
                    g.d.set_y(this, py);
                    // this._parent.removeMovieClip() : la copie disparaît.
                    if let Some(p) = g.parent(this) {
                        g.d.remove(p);
                    }
                    return Flow::Delete;
                }
                Flow::Keep
            });
        }
    }

    // --- Mode d'emploi (clip 271) et son panneau (clip 269) -----------------------------

    fn howto_script(&mut self, o: Obj, frame: u16) {
        match frame {
            1 | 33 => self.stop(o),
            2 => {
                self.title.howto.insert(o, 15.0);
                self.howto_animate(o, -1.0, 0.0);
                self.on_mouse_down(o, |g, this| {
                    g.title.howto.insert(this, 1.0);
                    g.native_goto_and_stop(this, "hideend");
                    g.run_enter_frame(this);
                    Flow::Keep
                });
            }
            17 => {
                self.d.set_visible(o, false);
                if let Some(t) = self.parent(o) {
                    self.allenabled(t, true);
                }
                self.native_goto_and_stop(o, 1);
            }
            18 => {
                if let Some(mask) = self.child(o, "mask_mc") {
                    self.native_goto_and_stop(mask, 1);
                }
                if let Some(panel) = self.child(o, "panel_mc") {
                    let about = self.child(panel, "aboutbtn_mc");
                    self.title.panel.entry(panel).or_default().current = about;
                }
                self.title.howto.insert(o, 0.0);
                self.howto_animate(o, 1.0, 15.0);
                self.on_mouse_down(o, |g, this| {
                    g.title.howto.insert(this, 14.0);
                    g.native_goto_and_stop(this, "startend");
                    g.run_enter_frame(this);
                    Flow::Keep
                });
            }
            _ => {}
        }
    }

    /// Le masque rond qui s'ouvre (`step` = 1) ou se ferme (-1) jusqu'à
    /// `end`.
    fn howto_animate(&mut self, o: Obj, step: f64, end: f64) {
        self.on_enter_frame(o, move |g, this| {
            let count = g.title.howto.get(&this).copied().unwrap_or(0.0) + step;
            g.title.howto.insert(this, count);
            let mut done = false;
            if let Some(mask) = g.child(this, "mask_mc") {
                for c in g.child_clips(mask) {
                    let s = 0.05925925925925926 * count * count * count * count + 100.0;
                    g.d.set_xscale(c, s);
                    let xs = g.d.xscale(c);
                    g.d.set_yscale(c, xs);
                    if count == end {
                        done = true;
                    }
                }
            }
            if done {
                g.delete_mouse_down(this);
                return Flow::Delete;
            }
            Flow::Keep
        });
    }

    fn panel_init(&mut self, panel: Obj) {
        self.stop(panel);
        let about = self.child(panel, "aboutbtn_mc");
        let state = self.title.panel.entry(panel).or_default();
        state.out_mc = None;
        state.in_mc = None;
        state.current = about;
        for (btn, target) in [("aboutbtn_mc", "about_mc"), ("aerialbtn_mc", "aerial_mc"), ("specialbtn_mc", "special_mc"), ("charabtn_mc", "chara_mc")] {
            let (Some(b), Some(t)) = (self.child(panel, btn), self.child(panel, target)) else { continue };
            self.panel_tab_init(panel, b, t);
        }
    }

    /// init d'un onglet : son panneau est montré (onglet courant) ou caché
    /// sur le côté ; le survol de l'onglet fait glisser les panneaux.
    fn panel_tab_init(&mut self, panel: Obj, btn: Obj, target: Obj) {
        self.set_hand_cursor(btn, false);
        let p = self.title.panel.entry(panel).or_default();
        p.target.insert(btn, target);
        if p.current == Some(btn) {
            p.pos.insert(target, 0.0);
            self.native_goto_and_stop(btn, 2);
            self.view_slide(panel, target);
        } else {
            p.pos.insert(target, -SLIDE_FRAMES);
            self.d.set_visible(target, false);
            self.native_goto_and_stop(btn, 1);
            self.view_slide(panel, target);
            self.stop(target);
        }
        self.events(btn).roll_over = Some(Box::new(move |g, this| {
            let Some(p) = g.title.panel.get(&panel) else { return Flow::Keep };
            let current = p.current;
            if current != Some(this) {
                g.native_goto_and_stop(this, 2);
                if let Some(cur) = current {
                    if let Some(t) = g.title.panel.get(&panel).and_then(|p| p.target.get(&cur).copied()) {
                        g.out_slide(panel, t);
                    }
                    g.native_goto_and_stop(cur, 1);
                }
                if let Some(p) = g.title.panel.get_mut(&panel) {
                    p.current = Some(this);
                }
                if let Some(t) = g.title.panel.get(&panel).and_then(|p| p.target.get(&this).copied()) {
                    g.in_slide(panel, t);
                }
            }
            Flow::Keep
        }));
    }

    fn slide_pos(&self, panel: Obj, target: Obj) -> f64 {
        self.title.panel.get(&panel).and_then(|p| p.pos.get(&target).copied()).unwrap_or(f64::NAN)
    }

    fn set_slide_pos(&mut self, panel: Obj, target: Obj, v: f64) {
        if let Some(p) = self.title.panel.get_mut(&panel) {
            p.pos.insert(target, v);
        }
    }

    /// viewslide : un panneau à sa position, éclairci sur les côtés.
    fn view_slide(&mut self, panel: Obj, target: Obj) {
        let pos = self.slide_pos(panel, target);
        if pos == SLIDE_FRAMES || pos == -SLIDE_FRAMES {
            self.d.set_visible(target, false);
            self.native_goto_and_stop(target, 1);
        } else {
            self.d.set_visible(target, true);
            self.d.set_x(target, pos * SLIDE_MAX / SLIDE_FRAMES - 350.0);
            let ra = 100.0 * (1.0 - pos.abs() / SLIDE_FRAMES);
            let rb = 255.0 * pos.abs() / SLIDE_FRAMES;
            self.set_color_transform(target, &ColorObject::new([ra, rb, ra, rb, ra, rb, 100.0, 0.0]));
        }
    }

    fn in_slide(&mut self, panel: Obj, target: Obj) {
        if let Some(p) = self.title.panel.get_mut(&panel) {
            p.in_mc = Some(target);
            if p.out_mc == Some(target) {
                p.out_mc = None;
            }
        }
        self.on_enter_frame(target, move |g, this| {
            let mut flow = Flow::Keep;
            if g.title.panel.get(&panel).is_some_and(|p| p.out_mc.is_none()) {
                let pos = g.slide_pos(panel, this);
                if pos > 0.0 {
                    g.set_slide_pos(panel, this, pos - 1.0);
                } else if pos < 0.0 {
                    g.set_slide_pos(panel, this, pos + 1.0);
                } else {
                    if let Some(p) = g.title.panel.get_mut(&panel) {
                        p.in_mc = None;
                    }
                    g.play(this);
                    flow = Flow::Delete;
                }
                g.view_slide(panel, this);
            }
            flow
        });
    }

    fn out_slide(&mut self, panel: Obj, target: Obj) {
        if let Some(in_mc) = self.title.panel.get(&panel).and_then(|p| p.in_mc) {
            self.delete_enter_frame(in_mc);
        }
        if self.title.panel.get(&panel).is_some_and(|p| p.out_mc.is_none()) {
            self.stop(target);
            if let Some(p) = self.title.panel.get_mut(&panel) {
                p.out_mc = Some(target);
            }
            self.on_enter_frame(target, move |g, this| {
                let mut flow = Flow::Keep;
                let pos = g.slide_pos(panel, this);
                if (0.0..SLIDE_FRAMES).contains(&pos) {
                    g.set_slide_pos(panel, this, pos + 1.0);
                } else if pos < 0.0 && pos > -SLIDE_FRAMES {
                    g.set_slide_pos(panel, this, pos - 1.0);
                } else {
                    let in_mc = g.title.panel.get(&panel).and_then(|p| p.in_mc);
                    if let Some(p) = g.title.panel.get_mut(&panel) {
                        p.out_mc = None;
                    }
                    if let Some(i) = in_mc {
                        g.set_slide_pos(panel, i, -pos);
                    }
                    flow = Flow::Delete;
                }
                g.view_slide(panel, this);
                flow
            });
        }
    }

    pub(super) fn title_unload(&mut self, o: Obj) {
        self.title.quality.remove(&o);
        self.title.sound.remove(&o);
        self.title.howto.remove(&o);
        self.title.panel.remove(&o);
    }
}
