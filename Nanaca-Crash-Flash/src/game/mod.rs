//! La logique du jeu, réécrite en Rust d'après le code ActionScript 2
//! d'origine (décompilé pour lecture, jamais publié). Elle pilote la liste
//! d'affichage comme le faisait Flash :
//!
//! - à chaque image : les `onEnterFrame`, puis l'avance des timelines, puis
//!   les scripts des images affichées (le parent avant les clips qu'il
//!   place), puis les sons ;
//! - les fonctions d'ActionScript posées sur des clips (`onEnterFrame`,
//!   `onMouseDown`, `onRollOver`…) deviennent des fermetures Rust rangées
//!   par objet ;
//! - les classes du jeu (`GameControl`, `GamePauseClip`, `GameCharacter`,
//!   `GameCharaFace`) et les scripts de chaque clip sont dans les modules.
//!
//! Le comportement d'origine est gardé tel quel, bizarreries comprises.

mod as2;
mod chara;
mod control;
mod focus;
mod hud;
mod root;
pub mod save;
mod scene;
mod title;

use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;

use crate::audio::Audio;
use crate::display::{ButtonState, Display, Kind, Obj};
use crate::geom::ColorTransform;
use crate::library::{Character, Id, Library};
use as2::Random;
pub use focus::Dir;
use save::Save;

/// Après une fonction posée sur un objet : la garder ou la retirer
/// (`delete this.onEnterFrame`).
pub enum Flow {
    Keep,
    Delete,
}

/// Une fonction d'ActionScript posée sur un objet ; elle reçoit l'objet
/// (`this`).
pub type Handler = Box<dyn FnMut(&mut Game, Obj) -> Flow>;

/// Une image visée : par numéro ou par étiquette.
pub enum To<'a> {
    N(u16),
    L(&'a str),
}

impl From<u16> for To<'_> {
    fn from(n: u16) -> Self {
        To::N(n)
    }
}

impl<'a> From<&'a str> for To<'a> {
    fn from(l: &'a str) -> Self {
        To::L(l)
    }
}

/// Les clips des symboles liés à GamePauseClip (et à GameCharacter, qui en
/// hérite), par Object.registerClass.
const PAUSE_CLIPS: [Id; 19] = [438, 311, 78, 518, 310, 328, 334, 288, 429, 534, 479, 352, 343, 350, 369, 388, 399, 421, 428];
/// Les clips liés à GameCharacter.
const CHARACTERS: [Id; 7] = [343, 350, 369, 388, 399, 421, 428];
/// Les clips liés à GameCharaFace.
const FACES: [Id; 7] = [469, 470, 471, 472, 473, 474, 475];

/// Les fonctions de bouton posées par le code.
#[derive(Default)]
pub struct ButtonEvents {
    pub roll_over: Option<Handler>,
    pub roll_out: Option<Handler>,
    pub release_outside: Option<Handler>,
}

/// L'état de GamePauseClip : la liste de ses clips (le plus récent
/// d'abord), la pause générale, et pour chaque clip ses drapeaux.
#[derive(Default)]
struct Pause {
    list: Vec<Obj>,
    master: bool,
    /// (pauseflag, stopflag).
    flags: HashMap<Obj, (bool, bool)>,
}

#[derive(Default)]
struct Mouse {
    pos: (f32, f32),
    hover: Option<Obj>,
    pressed: Option<Obj>,
    down: bool,
}

pub struct Game {
    pub d: Display,
    pub audio: Audio,
    lib: Rc<Library>,
    rng: Random,
    /// getTimer : millisecondes depuis le lancement.
    pub time: f64,
    /// _quality : "HIGH", "MEDIUM" ou "LOW".
    pub quality: &'static str,
    /// Plein écran sans bordure (Alt+Entrée).
    borderless: bool,
    save: Save,
    enter_frame: BTreeMap<Obj, Handler>,
    mouse_down: BTreeMap<Obj, Handler>,
    mouse_up: BTreeMap<Obj, Handler>,
    /// Le setInterval du compteur de lancement (son clip, sa fonction).
    interval: Option<(Obj, Handler)>,
    events: HashMap<Obj, ButtonEvents>,
    /// Clips devenus boutons (une fonction de bouton posée, même nulle).
    clip_buttons: HashSet<Obj>,
    /// enabled = false (clips et boutons).
    disabled: HashSet<Obj>,
    /// useHandCursor = false.
    no_hand: HashSet<Obj>,
    mouse: Mouse,
    /// Le bouton choisi à la manette.
    focus: Option<Obj>,
    pause: Pause,
    /// Volumes (0..100) des objets Sound, par clip (None : le global).
    volumes: HashMap<Option<Obj>, f64>,
    /// Variables liées à des champs de texte, par timeline.
    vars: HashMap<(Obj, &'static str), String>,
    root: root::Root,
    control: control::Control,
    title: title::Title,
    hud: hud::Hud,
    scene: scene::Scene,
    chara: chara::Charas,
}

impl Game {
    pub fn new(lib: Rc<Library>, audio: Audio, save: Save) -> Game {
        Game {
            d: Display::new(lib.clone()),
            audio,
            lib,
            rng: Random::new(),
            time: 0.0,
            quality: "MEDIUM",
            borderless: false,
            save,
            enter_frame: BTreeMap::new(),
            mouse_down: BTreeMap::new(),
            mouse_up: BTreeMap::new(),
            interval: None,
            events: HashMap::new(),
            clip_buttons: HashSet::new(),
            disabled: HashSet::new(),
            no_hand: HashSet::new(),
            mouse: Mouse::default(),
            focus: None,
            pause: Pause::default(),
            volumes: HashMap::new(),
            vars: HashMap::new(),
            root: root::Root::default(),
            control: control::Control::default(),
            title: title::Title::default(),
            hud: hud::Hud::default(),
            scene: scene::Scene::default(),
            chara: chara::Charas::default(),
        }
    }

    // --- Une image ---------------------------------------------------------

    /// Une image du film : les timelines avancent, puis les onEnterFrame,
    /// puis les scripts des images (Flash passe les onEnterFrame avant les
    /// scripts ; un gotoAndPlay dans un onEnterFrame ne fait pas avancer
    /// le clip une seconde fois).
    pub fn step(&mut self) {
        self.d.advance();
        // Les plus récents d'abord, comme Flash.
        let objects: Vec<Obj> = self.enter_frame.keys().rev().copied().collect();
        for o in objects {
            if self.paused(o) {
                continue;
            }
            self.call(Slot::EnterFrame, o);
        }
        self.run_scripts();
    }

    /// Le compteur de lancement suit l'horloge entre les images.
    pub fn tick(&mut self, time: f64) {
        self.time = time;
        if let Some((o, mut f)) = self.interval.take() {
            if let Flow::Keep = f(self, o)
                && self.interval.is_none()
            {
                self.interval = Some((o, f));
            }
        }
    }

    /// Les constructeurs des clips créés, puis les scripts des images
    /// affichées, dans l'ordre ; puis les clips retirés, les sons, les
    /// champs liés.
    pub fn run_scripts(&mut self) {
        loop {
            for o in std::mem::take(&mut self.d.created) {
                self.construct(o);
            }
            if self.d.frames.is_empty() {
                break;
            }
            let (o, frame) = self.d.frames.remove(0);
            if self.d.exists(o) {
                self.frame_script(o, frame);
            }
        }
        for o in std::mem::take(&mut self.d.removed) {
            self.unload(o);
        }
        for (o, s) in std::mem::take(&mut self.d.sounds) {
            if s.stop {
                self.audio.stop_sound(s.id);
            } else if !(s.no_multiple && self.audio.playing(s.id)) {
                let gain = self.gain(Some(o));
                self.audio.start(&self.lib.clone(), s.id, Some(o), 0.0, u32::from(s.loops), Some(&s), gain);
            }
        }
        self.update_texts();
        let volumes = &self.volumes;
        let d = &self.d;
        self.audio.set_gains(|owner| gain(d, volumes, owner));
        for handle in self.audio.finished() {
            self.sound_complete(handle);
        }
    }

    /// Le script d'une image d'un clip.
    fn frame_script(&mut self, o: Obj, frame: u16) {
        let id = self.d.get(o).and_then(|x| x.character);
        let _ = self.root_frame(o, id, frame) || self.title_frame(o, id, frame) || self.scene_frame(o, id, frame) || self.hud_frame(o, id, frame) || self.chara_frame(o, id, frame);
    }

    /// Les constructeurs des classes liées aux symboles.
    fn construct(&mut self, o: Obj) {
        let Some(id) = self.d.get(o).and_then(|x| x.character) else { return };
        if PAUSE_CLIPS.contains(&id) {
            // GamePauseClip : en tête de liste, en pause si tout l'est.
            self.pause.list.insert(0, o);
            self.pause.flags.insert(o, (self.pause.master, false));
        }
        if CHARACTERS.contains(&id) {
            self.character_constructor(o);
        }
        if FACES.contains(&id) {
            self.face_constructor(o);
        }
    }

    /// Un clip retiré : son onUnload et ce qui le concernait.
    fn unload(&mut self, o: Obj) {
        if self.pause.flags.remove(&o).is_some() {
            // GamePauseClip.onUnload : hors de la liste, ses sons arrêtés.
            self.pause.list.retain(|&x| x != o);
            self.audio.stop_owner(o);
        }
        self.unload_scripts(o);
        self.enter_frame.remove(&o);
        self.mouse_down.remove(&o);
        self.mouse_up.remove(&o);
        self.events.remove(&o);
        self.clip_buttons.remove(&o);
        self.disabled.remove(&o);
        self.no_hand.remove(&o);
        self.volumes.remove(&Some(o));
        self.vars.retain(|k, _| k.0 != o);
        if self.interval.as_ref().is_some_and(|i| i.0 == o) {
            self.interval = None;
        }
    }

    // --- Fonctions posées sur les objets ---------------------------------------

    pub fn on_enter_frame(&mut self, o: Obj, f: impl FnMut(&mut Game, Obj) -> Flow + 'static) {
        self.enter_frame.insert(o, Box::new(f));
    }

    pub fn delete_enter_frame(&mut self, o: Obj) {
        self.enter_frame.remove(&o);
    }

    pub fn on_mouse_down(&mut self, o: Obj, f: impl FnMut(&mut Game, Obj) -> Flow + 'static) {
        self.mouse_down.insert(o, Box::new(f));
    }

    pub fn delete_mouse_down(&mut self, o: Obj) {
        self.mouse_down.remove(&o);
    }

    pub fn on_mouse_up(&mut self, o: Obj, f: impl FnMut(&mut Game, Obj) -> Flow + 'static) {
        self.mouse_up.insert(o, Box::new(f));
    }

    pub fn delete_mouse_up(&mut self, o: Obj) {
        self.mouse_up.remove(&o);
    }

    /// Appelle la fonction d'un objet ; elle reste en place si elle le
    /// demande et n'a pas été remplacée entre-temps.
    fn call(&mut self, slot: Slot, o: Obj) {
        let map = match slot {
            Slot::EnterFrame => &mut self.enter_frame,
            Slot::MouseDown => &mut self.mouse_down,
            Slot::MouseUp => &mut self.mouse_up,
        };
        let Some(mut f) = map.remove(&o) else { return };
        let flow = f(self, o);
        let map = match slot {
            Slot::EnterFrame => &mut self.enter_frame,
            Slot::MouseDown => &mut self.mouse_down,
            Slot::MouseUp => &mut self.mouse_up,
        };
        if let Flow::Keep = flow
            && !map.contains_key(&o)
            && self.d.exists(o)
        {
            map.insert(o, f);
        }
    }

    /// Appelle tout de suite le onEnterFrame d'un objet (`onEnterFrame()`).
    pub fn run_enter_frame(&mut self, o: Obj) {
        self.call(Slot::EnterFrame, o);
    }

    // --- Timelines (avec les surcharges de GamePauseClip) ----------------------

    fn frame_number(&self, o: Obj, to: To) -> Option<u16> {
        match to {
            To::N(n) => Some(n),
            To::L(l) => self.d.label(o, l),
        }
    }

    fn paused(&self, o: Obj) -> bool {
        self.pause.flags.get(&o).is_some_and(|f| f.0)
    }

    fn set_stopflag(&mut self, o: Obj, v: bool) {
        if let Some(f) = self.pause.flags.get_mut(&o) {
            f.1 = v;
        }
    }

    pub fn play(&mut self, o: Obj) {
        self.set_stopflag(o, false);
        if !self.paused(o) {
            self.d.play(o);
        }
    }

    pub fn stop(&mut self, o: Obj) {
        self.set_stopflag(o, true);
        self.d.stop(o);
    }

    pub fn goto_and_play<'a>(&mut self, o: Obj, to: impl Into<To<'a>>) {
        let Some(f) = self.frame_number(o, to.into()) else { return };
        self.set_stopflag(o, false);
        if self.paused(o) {
            self.d.goto_and_stop(o, f);
        } else {
            self.d.goto_and_play(o, f);
        }
    }

    pub fn goto_and_stop<'a>(&mut self, o: Obj, to: impl Into<To<'a>>) {
        let Some(f) = self.frame_number(o, to.into()) else { return };
        self.set_stopflag(o, true);
        self.d.goto_and_stop(o, f);
    }

    /// gotoAndStop natif (sans les surcharges de GamePauseClip).
    pub fn native_goto_and_stop<'a>(&mut self, o: Obj, to: impl Into<To<'a>>) {
        if let Some(f) = self.frame_number(o, to.into()) {
            self.d.goto_and_stop(o, f);
        }
    }

    pub fn native_goto_and_play<'a>(&mut self, o: Obj, to: impl Into<To<'a>>) {
        if let Some(f) = self.frame_number(o, to.into()) {
            self.d.goto_and_play(o, f);
        }
    }

    pub fn next_frame(&mut self, o: Obj) {
        self.set_stopflag(o, true);
        let f = self.d.current_frame(o);
        self.d.goto_and_stop(o, f.saturating_add(1));
    }

    /// GamePauseClip.allpausestop : tout s'arrête.
    pub fn all_pause_stop(&mut self) {
        self.pause.master = true;
        for o in self.pause.list.clone() {
            if let Some(f) = self.pause.flags.get_mut(&o) {
                f.0 = true;
            }
            self.d.stop(o);
        }
    }

    /// GamePauseClip.allpauseplay : tout repart, sauf ce que le code a arrêté.
    pub fn all_pause_play(&mut self) {
        self.pause.master = false;
        for o in self.pause.list.clone() {
            let Some(f) = self.pause.flags.get_mut(&o) else { continue };
            f.0 = false;
            if !f.1 {
                self.d.play(o);
            }
        }
    }

    /// GamePauseClip.allpausetoggle : rend la nouvelle pause générale.
    pub fn all_pause_toggle(&mut self) -> bool {
        if self.pause.master {
            self.all_pause_play();
        } else {
            self.all_pause_stop();
        }
        self.pause.master
    }

    // --- Objets ------------------------------------------------------------------

    pub fn child(&self, o: Obj, name: &str) -> Option<Obj> {
        self.d.child(o, name)
    }

    pub fn parent(&self, o: Obj) -> Option<Obj> {
        self.d.get(o)?.parent
    }

    /// Les clips enfants d'un clip ou d'un bouton, comme `for (i in clip)`
    /// les énumère (les plus récents d'abord).
    pub fn child_clips(&self, o: Obj) -> Vec<Obj> {
        let mut list: Vec<Obj> = self.d.children(o).into_iter().filter(|&c| matches!(self.d.get(c).map(|x| &x.kind), Some(Kind::Clip(_)))).collect();
        list.sort_by_key(|c| std::cmp::Reverse(*c));
        list
    }

    pub fn random(&mut self) -> f64 {
        self.rng.next()
    }

    pub fn set_enabled(&mut self, o: Obj, v: bool) {
        if v {
            self.disabled.remove(&o);
        } else {
            self.disabled.insert(o);
        }
    }

    pub fn enabled(&self, o: Obj) -> bool {
        !self.disabled.contains(&o)
    }

    pub fn set_hand_cursor(&mut self, o: Obj, v: bool) {
        if v {
            self.no_hand.remove(&o);
        } else {
            self.no_hand.insert(o);
        }
    }

    /// Les fonctions de bouton d'un objet (un clip devient bouton).
    pub fn events(&mut self, o: Obj) -> &mut ButtonEvents {
        if !matches!(self.d.get(o).map(|x| &x.kind), Some(Kind::Button(_))) {
            self.clip_buttons.insert(o);
        }
        self.events.entry(o).or_default()
    }

    /// Un clip qui devient bouton sans fonction (`onPress = null`).
    pub fn make_clip_button(&mut self, o: Obj) {
        self.clip_buttons.insert(o);
    }

    /// Une variable de timeline liée à des champs de texte.
    pub fn set_var(&mut self, timeline: Obj, name: &'static str, value: String) {
        self.vars.insert((timeline, name), value);
    }

    /// Les champs liés à une variable affichent sa valeur.
    fn update_texts(&mut self) {
        if self.vars.is_empty() {
            return;
        }
        let mut list = vec![self.d.root];
        while let Some(o) = list.pop() {
            list.extend(self.d.children(o));
            let Some(x) = self.d.get(o) else { continue };
            if !matches!(x.kind, Kind::Text(_)) {
                continue;
            }
            let Some(Character::EditText(def)) = x.character.and_then(|id| self.lib.characters.get(&id)) else { continue };
            if def.variable.is_empty() {
                continue;
            }
            // La timeline d'un champ : son clip, ou celui de son bouton.
            let mut timeline = x.parent;
            if let Some(p) = timeline
                && matches!(self.d.get(p).map(|x| &x.kind), Some(Kind::Button(_)))
            {
                timeline = self.parent(p);
            }
            let Some(t) = timeline else { continue };
            if let Some(value) = self.vars.iter().find(|(k, _)| k.0 == t && k.1 == def.variable).map(|(_, v)| v.clone())
                && self.d.text(o) != Some(value.as_str())
            {
                self.d.set_text(o, &value);
            }
        }
    }

    // --- Couleurs (objets Color) ----------------------------------------------------

    /// Color.setTransform : seules les clés données changent (ra…ab).
    pub fn set_color_transform(&mut self, o: Obj, c: &ColorObject) {
        let Some(x) = self.d.scripted(o) else { return };
        let ct = &mut x.color;
        for (i, (mult, add)) in [(c.ra, c.rb), (c.ga, c.gb), (c.ba, c.bb), (c.aa, c.ab)].into_iter().enumerate() {
            if let Some(m) = mult.filter(|v| v.is_finite()) {
                ct.mult[i] = (m / 100.0) as f32;
            }
            if let Some(a) = add.filter(|v| v.is_finite()) {
                ct.add[i] = (a / 255.0) as f32;
            }
        }
    }

    /// Color.getTransform.
    pub fn color_transform(&self, o: Obj) -> ColorObject {
        let c = self.d.get(o).map_or(ColorTransform::IDENTITY, |x| x.color);
        let m = |i: usize| Some((f64::from(c.mult[i]) * 100.0 * 256.0).round() / 256.0);
        let a = |i: usize| Some((f64::from(c.add[i]) * 255.0).round());
        ColorObject { ra: m(0), rb: a(0), ga: m(1), gb: a(1), ba: m(2), bb: a(2), aa: m(3), ab: a(3) }
    }

    /// Color.setRGB : la couleur remplace le rouge, le vert et le bleu.
    pub fn set_rgb(&mut self, o: Obj, rgb: u32) {
        if let Some(x) = self.d.scripted(o) {
            for i in 0..3 {
                x.color.mult[i] = 0.0;
                x.color.add[i] = ((rgb >> (16 - 8 * i)) & 0xFF) as f32 / 255.0;
            }
        }
    }

    // --- Sons (objets Sound) ---------------------------------------------------------

    /// Sound.getVolume (None : le volume global).
    pub fn volume(&self, target: Option<Obj>) -> f64 {
        self.volumes.get(&target).copied().unwrap_or(100.0)
    }

    pub fn set_volume(&mut self, target: Option<Obj>, v: f64) {
        self.volumes.insert(target, v);
    }

    /// Le volume d'un son d'un clip : le global, et ceux du clip et de ses
    /// parents.
    fn gain(&self, owner: Option<Obj>) -> f32 {
        gain(&self.d, &self.volumes, owner).unwrap_or(0.0)
    }

    /// Lance un son exporté (attachSound puis start) pour un clip.
    pub fn start_sound(&mut self, owner: Obj, name: &str, offset: f64) -> Option<u64> {
        let id = *self.lib.exports.get(name)?;
        let gain = self.gain(Some(owner));
        let lib = self.lib.clone();
        self.audio.start(&lib, id, Some(owner), offset, 1, None, gain)
    }

    // --- Souris --------------------------------------------------------------------------

    /// La souris a bougé : elle reprend la main sur la manette.
    pub fn mouse_move(&mut self, x: f32, y: f32) {
        self.focus = None;
        self.pointer_move(x, y);
    }

    /// Le pointeur sur la scène.
    fn pointer_move(&mut self, x: f32, y: f32) {
        self.mouse.pos = (x, y);
        let over = self.interactive_at(x, y);
        if self.mouse.down
            && let Some(p) = self.mouse.pressed
        {
            // Glisser hors du bouton pressé, ou y revenir.
            let (was, is) = (self.mouse.hover == Some(p), over == Some(p));
            if was != is {
                self.set_button_state(p, if is { ButtonState::Down } else { ButtonState::Over });
            }
            self.mouse.hover = over;
        } else {
            self.set_hover(over);
        }
        self.run_scripts();
    }

    /// L'objet survolé change : roll out de l'ancien, roll over du nouveau.
    fn set_hover(&mut self, over: Option<Obj>) {
        if over == self.mouse.hover {
            return;
        }
        if let Some(old) = self.mouse.hover {
            self.set_button_state(old, ButtonState::Up);
            self.button_event(old, ButtonSlot::RollOut);
        }
        if let Some(new) = over {
            self.set_button_state(new, ButtonState::Over);
            self.button_event(new, ButtonSlot::RollOver);
        }
        self.mouse.hover = over;
    }

    pub fn mouse_down(&mut self) {
        let time = self.time;
        self.tick(time);
        self.mouse.down = true;
        for o in self.mouse_down.keys().rev().copied().collect::<Vec<_>>() {
            self.call(Slot::MouseDown, o);
        }
        if let Some(o) = self.mouse.hover {
            self.mouse.pressed = Some(o);
            self.set_button_state(o, ButtonState::Down);
        }
        self.run_scripts();
    }

    pub fn mouse_up(&mut self) {
        let time = self.time;
        self.tick(time);
        self.mouse.down = false;
        for o in self.mouse_up.keys().rev().copied().collect::<Vec<_>>() {
            self.call(Slot::MouseUp, o);
        }
        if let Some(p) = self.mouse.pressed.take() {
            if self.mouse.hover == Some(p) {
                self.set_button_state(p, ButtonState::Over);
                self.release(p);
            } else {
                self.set_button_state(p, ButtonState::Up);
                self.button_event(p, ButtonSlot::ReleaseOutside);
            }
        }
        self.run_scripts();
        // Ce qui est sous le pointeur a pu changer (sauf si la manette a
        // choisi un bouton).
        if self.focus.is_none() {
            let (x, y) = self.mouse.pos;
            self.pointer_move(x, y);
        }
    }

    /// La main au-dessus des boutons.
    pub fn hand_cursor(&self) -> bool {
        self.mouse.hover.is_some_and(|o| !self.no_hand.contains(&o))
    }

    fn set_button_state(&mut self, o: Obj, state: ButtonState) {
        if let Some(x) = self.d.get_mut(o)
            && let Kind::Button(b) = &mut x.kind
        {
            b.state = state;
        }
    }

    fn button_event(&mut self, o: Obj, slot: ButtonSlot) {
        let Some(events) = self.events.get_mut(&o) else { return };
        let f = match slot {
            ButtonSlot::RollOver => events.roll_over.take(),
            ButtonSlot::RollOut => events.roll_out.take(),
            ButtonSlot::ReleaseOutside => events.release_outside.take(),
        };
        let Some(mut f) = f else { return };
        f(self, o);
        if let Some(events) = self.events.get_mut(&o) {
            let place = match slot {
                ButtonSlot::RollOver => &mut events.roll_over,
                ButtonSlot::RollOut => &mut events.roll_out,
                ButtonSlot::ReleaseOutside => &mut events.release_outside,
            };
            if place.is_none() {
                *place = Some(f);
            }
        }
    }

    /// Relâché sur un bouton : son action on(release), jouée dans la
    /// timeline qui le contient.
    fn release(&mut self, o: Obj) {
        let Some(x) = self.d.get(o) else { return };
        let (Some(id), Some(parent)) = (x.character, x.parent) else { return };
        if matches!(x.kind, Kind::Button(_)) {
            self.button_action(id, parent);
        }
    }

    fn button_action(&mut self, id: Id, this: Obj) {
        let _ = self.title_button(id, this) || self.hud_button(id, this);
    }

    /// L'objet interactif le plus haut sous un point de la scène.
    fn interactive_at(&self, x: f32, y: f32) -> Option<Obj> {
        self.find_interactive(self.d.root, x, y)
    }

    fn find_interactive(&self, o: Obj, x: f32, y: f32) -> Option<Obj> {
        for c in self.d.children(o).into_iter().rev() {
            let Some(cx) = self.d.get(c) else { continue };
            if !cx.visible || cx.clip_depth != 0 {
                continue;
            }
            match cx.kind {
                Kind::Button(_) => {
                    if self.enabled(c) && self.d.hit(c, x, y) {
                        return Some(c);
                    }
                }
                Kind::Clip(_) if self.clip_buttons.contains(&c) => {
                    if self.enabled(c) && self.d.hit(c, x, y) {
                        return Some(c);
                    }
                }
                Kind::Clip(_) => {
                    if let Some(found) = self.find_interactive(c, x, y) {
                        return Some(found);
                    }
                }
                _ => {}
            }
        }
        None
    }

    // --- Affichage ----------------------------------------------------------------------

    /// Le mode d'affichage enregistré (rétabli au lancement).
    pub fn saved_borderless(&self) -> bool {
        self.save.text(save::SETTINGS, "displaysetting") == Some("borderless")
    }

    /// Alt+Entrée : fenêtre ou plein écran sans bordure, enregistré.
    pub fn set_borderless(&mut self, on: bool) {
        self.borderless = on;
        crate::platform::set_borderless(on);
        self.save.set(save::SETTINGS, "displaysetting", save::Value::Text(if on { "borderless" } else { "window" }.into()));
        self.save.flush();
    }

    pub fn borderless(&self) -> bool {
        self.borderless
    }
}

#[derive(Clone, Copy)]
enum Slot {
    EnterFrame,
    MouseDown,
    MouseUp,
}

#[derive(Clone, Copy)]
enum ButtonSlot {
    RollOver,
    RollOut,
    ReleaseOutside,
}

/// Un objet de couleur d'ActionScript ({ra, rb, ga, gb, ba, bb, aa, ab}) :
/// multiplicateurs en %, décalages en -255..255.
#[derive(Clone, Copy, Debug, Default)]
pub struct ColorObject {
    pub ra: Option<f64>,
    pub rb: Option<f64>,
    pub ga: Option<f64>,
    pub gb: Option<f64>,
    pub ba: Option<f64>,
    pub bb: Option<f64>,
    pub aa: Option<f64>,
    pub ab: Option<f64>,
}

impl ColorObject {
    /// Les huit valeurs données.
    pub fn new(v: [f64; 8]) -> ColorObject {
        ColorObject { ra: Some(v[0]), rb: Some(v[1]), ga: Some(v[2]), gb: Some(v[3]), ba: Some(v[4]), bb: Some(v[5]), aa: Some(v[6]), ab: Some(v[7]) }
    }
}

/// Le volume d'un son (0..1) : le global, et ceux de son clip et des
/// parents ; None si le clip a disparu.
fn gain(d: &Display, volumes: &HashMap<Option<Obj>, f64>, owner: Option<Obj>) -> Option<f32> {
    let volume = |k: Option<Obj>| volumes.get(&k).copied().unwrap_or(100.0) / 100.0;
    let mut g = volume(None);
    let mut cur = owner;
    while let Some(o) = cur {
        cur = d.get(o)?.parent;
        g *= volume(Some(o));
    }
    Some(g as f32)
}
