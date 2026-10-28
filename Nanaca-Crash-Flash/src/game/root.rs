//! La timeline principale (chargement, intro, enchaînement titre / jeu), et
//! les petits clips communs : chiffres, transitions entre scènes, musique,
//! bruitages.

use std::collections::{HashMap, HashSet};

use super::as2::{char_at, number_to_string, string_to_number};
use super::{Flow, Game, To};
use crate::display::Obj;
use crate::library::Id;

/// Où vont les fonctions prevact et nextact de la racine.
#[derive(Clone, Copy, Default)]
enum Act {
    #[default]
    None,
    First,
    Wait,
    Title,
    Game,
}

#[derive(Default)]
pub struct Root {
    init: bool,
    /// L'étiquette d'attente en cours ("wait1"…).
    wait: &'static str,
    prevact: Act,
    nextact: Act,
    /// changer_mc, la transition entre les scènes.
    pub changer: Option<Obj>,
    /// Chiffres (clip 22) : leur variable num, et s'ils la surveillent.
    digits: HashMap<Obj, (Option<String>, bool)>,
    /// La position de départ du masque du chargement (clip 24).
    inity: HashMap<Obj, f64>,
    /// Transitions (clip 57) : le clip dont elles baissent le son.
    slide: HashMap<Obj, Option<Obj>>,
    /// Musiques (clip 135) : le son attaché et la reprise à la fin.
    bgm: HashMap<Obj, (Option<&'static str>, Option<f64>)>,
    /// Voix lancées par les musiques, vers leur clip.
    voices: HashMap<u64, Obj>,
    /// Bruitages (clips 313 et 315).
    se: HashSet<Obj>,
}

impl Game {
    pub(super) fn root_frame(&mut self, o: Obj, id: Option<Id>, frame: u16) -> bool {
        match id {
            None => self.root_script(o, frame),
            Some(22) => self.digit_frame(o, frame),
            Some(24) => {
                // Le chargement.
                self.stop(o);
                let mask = self.child(o, "mask_mc");
                let y = mask.map_or(f64::NAN, |m| self.d.y(m));
                self.root.inity.insert(o, y);
            }
            Some(39) => {
                for (name, n) in [("n1_mc", 0.0), ("n2_mc", 0.0), ("n3_mc", 1.0)] {
                    if let Some(c) = self.child(o, name) {
                        self.set_digit(c, Some(number_to_string(n)));
                    }
                }
            }
            Some(57) => self.changer_frame(o, frame),
            Some(54) => {
                // Un cache pendant les transitions : il prend les clics
                // (onPress = null en fait un bouton), sans main.
                self.set_hand_cursor(o, false);
                self.make_clip_button(o);
            }
            Some(135) => self.bgm_frame(o, frame),
            Some(313 | 315) => match frame {
                1 => {
                    self.stop(o);
                    self.d.set_visible(o, false);
                    self.root.se.insert(o);
                }
                2 => self.next_frame(o),
                3 => self.stop(o),
                _ => {}
            },
            _ => return false,
        }
        true
    }

    fn root_script(&mut self, root: Obj, frame: u16) {
        match frame {
            1 => {
                self.audio.stop_all();
                self.delete_mouse_down(root);
                self.play(root);
                if !self.root.init {
                    self.root.init = true;
                    // La version de Flash (« WIN 8,0,24,0 ») : 8, assez récente.
                }
                if let Some(load) = self.child(root, "load_mc") {
                    self.loading_view(load, 1.0);
                }
            }
            2 => self.goto_and_play(root, 7),
            3 => {
                self.root.wait = "wait4";
                self.root.prevact = Act::Wait;
                self.root.nextact = Act::Wait;
                self.delete_mouse_down(root);
                let wait = self.root.wait;
                self.goto_and_stop(root, wait);
            }
            7 => {
                self.control = super::control::Control::default();
                self.local_load();
                if let Some(load2) = self.child(root, "load2_mc") {
                    let (x, y) = (self.d.x(load2), self.d.y(load2));
                    for letter in self.child_clips(load2) {
                        self.intro_hop(letter, x, y);
                    }
                }
                self.root.wait = "wait1";
                self.root.prevact = Act::First;
                self.root.nextact = Act::Title;
                self.on_mouse_down(root, |g, this| {
                    g.goto_and_stop(this, "title");
                    Flow::Keep
                });
                self.goto_and_stop(root, "wait1");
            }
            11 => {
                self.root.changer = self.child(root, "changer_mc");
                self.local_save();
                self.root.wait = "wait2";
                self.root.prevact = Act::First;
                self.root.nextact = Act::Game;
                self.delete_mouse_down(root);
                self.goto_and_stop(root, "wait2");
            }
            15 => {
                self.local_save();
                let main = self.child(root, "main_mc");
                self.control.main = main;
                self.control.initialize();
                if let Some(main) = main {
                    self.goto_and_stop(main, "stop");
                }
                self.root.wait = "wait3";
                self.root.prevact = Act::Title;
                self.root.nextact = Act::Wait;
                self.delete_mouse_down(root);
                self.goto_and_stop(root, "wait3");
            }
            4 | 8 | 12 | 16 => self.root_act(root, self.root.prevact),
            6 | 10 | 14 | 18 => self.root_act(root, self.root.nextact),
            _ => {}
        }
    }

    fn root_act(&mut self, root: Obj, act: Act) {
        match act {
            Act::None => {}
            Act::First => self.goto_and_stop(root, 1),
            Act::Wait => {
                let wait = self.root.wait;
                self.goto_and_stop(root, wait);
            }
            Act::Title => self.goto_and_stop(root, "title"),
            Act::Game => self.goto_and_stop(root, "game"),
        }
    }

    /// load_mc.view : le masque monte et les chiffres montrent le
    /// pourcentage chargé.
    fn loading_view(&mut self, load: Obj, loaded: f64) {
        if let Some(mask) = self.child(load, "mask_mc") {
            let inity = self.root.inity.get(&load).copied().unwrap_or(f64::NAN);
            let h = self.d.height(mask);
            self.d.set_y(mask, inity - loaded * h);
        }
        let s = number_to_string((loaded * 100.0).floor());
        let len = s.chars().count() as i64;
        for (name, k) in [("n1_mc", 1), ("n2_mc", 2), ("n3_mc", 3)] {
            if let Some(c) = self.child(load, name) {
                self.set_digit(c, Some(char_at(&s, len - k)));
            }
        }
    }

    /// La fonction hop de l'intro, posée sur chaque lettre du chargement :
    /// un petit saut sur place, puis un vol en tournant vers le coin.
    fn intro_hop(&mut self, letter: Obj, x: f64, y: f64) {
        let (px, py) = (self.d.x(letter), self.d.y(letter));
        let ex = 575.0 - x - px * 50.0 / 100.0;
        let ey = 300.0 - y - py * 50.0 / 100.0;
        let rot = (self.random() * 4.0).floor() - 1.0;
        let peakx = self.random() * 450.0 - 450.0;
        let peaky = self.random() * 300.0 - 300.0;
        let offset = ((px + 150.0) / 40.0).floor();
        let frames = 20.0;
        let mut count = frames;
        let mut offset2 = (self.random() * 15.0).floor();
        let frames2 = 50.0 - offset2;
        let mut count2 = frames2;
        let pi = std::f64::consts::PI;
        let mut action = move |g: &mut Game, this: Obj| {
            if count > -10.0 {
                g.d.set_x(this, px);
                g.d.set_y(this, -100.0 * (pi * (offset + count) / frames).sin().max(0.0) + py);
                g.d.set_xscale(this, 100.0 * (2.0 * pi * (offset + count).max(0.0).min(frames) / frames).cos());
                g.d.set_yscale(this, 100.0);
                count -= 1.0;
                g.d.set_rotation(this, 0.0);
            } else if offset2 > 0.0 {
                offset2 -= 1.0;
            } else {
                let t = 1.0 - count2 / frames2;
                g.d.set_x(this, peakx * (pi * t).sin() + ex * t + px);
                g.d.set_y(this, peaky * (pi * t).sin() + ey * t + py);
                g.d.set_xscale(this, (99.0 * count2 / frames2 + 50.0 - 49.0) * (2.0 * pi * t).sin() + 50.0 * count2 / frames2 + 50.0);
                let xs = g.d.xscale(this);
                g.d.set_yscale(this, xs);
                g.d.set_rotation(this, -360.0 * rot * count2 / frames2);
                count2 -= 1.0;
                if count2 < -1.0 {
                    // gotoAndStop sans cible : la timeline de la fonction, la racine.
                    let root = g.d.root;
                    g.native_goto_and_stop(root, "title");
                    return Flow::Delete;
                }
            }
            Flow::Keep
        };
        action(self, letter);
        self.on_enter_frame(letter, action);
    }

    // --- Chiffres (clip 22) ------------------------------------------------------

    fn digit_frame(&mut self, o: Obj, frame: u16) {
        if frame == 1 {
            self.stop(o);
            // watch("num") puis num = num : le chiffre s'affiche.
            let num = self.root.digits.get(&o).and_then(|d| d.0.clone());
            self.root.digits.insert(o, (None, true));
            self.set_digit(o, num);
        }
    }

    /// num = valeur : un chiffre qui surveille sa variable va à son image,
    /// ou se cache si ce n'est pas un nombre.
    pub(super) fn set_digit(&mut self, o: Obj, value: Option<String>) {
        let watched = self.root.digits.get(&o).is_some_and(|d| d.1);
        if watched {
            let n = value.as_deref().map_or(f64::NAN, string_to_number);
            if n.is_nan() {
                self.d.set_visible(o, false);
            } else {
                self.d.set_visible(o, true);
                let f = (n + 2.0).clamp(0.0, f64::from(u16::MAX)) as u16;
                self.goto_and_stop(o, f);
            }
        }
        self.root.digits.insert(o, (value, watched));
    }

    // --- Transitions (clip 57) ---------------------------------------------------

    /// changer.changescene : la scène suivante glisse, le son de `sound`
    /// baisse ; un clic passe à la fin.
    pub(super) fn change_scene(&mut self, changer: Obj, scene: &'static str, sound: Option<Obj>) {
        self.root.slide.insert(changer, sound);
        self.goto_and_play(changer, scene);
        self.on_enter_frame(changer, move |g, _| {
            let v = g.volume(sound);
            g.set_volume(sound, (v - 10.0).max(0.0));
            Flow::Keep
        });
        self.on_mouse_down(changer, move |g, this| {
            g.goto_and_stop(this, To::L(&format!("{scene}skip")));
            Flow::Keep
        });
    }

    fn changer_frame(&mut self, o: Obj, frame: u16) {
        let skip = |g: &mut Game, scene: &str| {
            let sound = g.root.slide.get(&o).copied().flatten();
            g.set_volume(sound, 0.0);
            g.delete_enter_frame(o);
            g.delete_mouse_down(o);
            if let Some(p) = g.parent(o) {
                g.goto_and_play(p, scene);
            }
            g.goto_and_stop(o, "standby");
        };
        match frame {
            2 => {
                self.stop(o);
                self.d.set_visible(o, false);
            }
            23 | 55 => skip(self, "game"),
            39 => skip(self, "title"),
            24 | 40 => self.d.set_visible(o, true),
            _ => {}
        }
    }

    // --- Musique (clip 135) --------------------------------------------------------

    fn bgm_frame(&mut self, o: Obj, frame: u16) {
        match frame {
            1 => {
                self.stop(o);
                self.d.set_visible(o, false);
                self.root.bgm.insert(o, (None, None));
            }
            2 | 3 => {
                let (name, volume, restart) = if frame == 2 { ("bgm2", 50.0, 6.4) } else { ("bgm1", 25.0, 24.0) };
                self.root.bgm.insert(o, (Some(name), Some(restart)));
                self.set_volume(Some(o), volume);
                self.bgm_start(o, 0.0);
            }
            _ => {}
        }
    }

    /// bgm_sound.start(offset).
    fn bgm_start(&mut self, o: Obj, offset: f64) {
        let Some((Some(name), _)) = self.root.bgm.get(&o).copied() else { return };
        if let Some(handle) = self.start_sound(o, name, offset) {
            self.root.voices.insert(handle, o);
        }
    }

    /// soundStart(track).
    pub(super) fn bgm_sound_start(&mut self, o: Obj, track: u16) {
        self.goto_and_stop(o, track + 1);
    }

    /// soundStop.
    pub(super) fn bgm_sound_stop(&mut self, o: Obj) {
        self.audio.stop_owner(o);
    }

    /// soundRestart.
    pub(super) fn bgm_sound_restart(&mut self, o: Obj) {
        self.audio.stop_owner(o);
        self.bgm_start(o, 0.0);
    }

    /// onSoundComplete : la musique reprend à son point de boucle.
    pub(super) fn sound_complete(&mut self, handle: u64) {
        let Some(o) = self.root.voices.remove(&handle) else { return };
        if !self.d.exists(o) {
            return;
        }
        if let Some((_, Some(restart))) = self.root.bgm.get(&o).copied() {
            self.bgm_start(o, restart);
        }
    }

    /// Les onUnload : musique et bruitages s'arrêtent.
    pub(super) fn unload_scripts(&mut self, o: Obj) {
        if self.root.bgm.remove(&o).is_some() | self.root.se.remove(&o) {
            self.audio.stop_owner(o);
        }
        self.root.voices.retain(|_, v| *v != o);
        self.root.digits.remove(&o);
        self.root.inity.remove(&o);
        self.root.slide.remove(&o);
        self.title_unload(o);
        self.hud_unload(o);
        self.chara_unload(o);
    }

    /// se_mc.soundplay(vol) : un bruitage à ce volume.
    pub(super) fn se_play(&mut self, se: Obj, vol: f64) {
        self.set_volume(Some(se), vol);
        if vol >= 1.0 {
            self.native_goto_and_play(se, "soundplay");
        }
    }
}
