//! Les armatures CocoStudio 1.x (cocos2d::extension::CCArmature) : des os
//! portant chacun une image de feuille, animés par images clés. Le
//! déroulement suit celui de Cocos2d 2.x (CCArmatureAnimation, CCTween,
//! CCProcessBase) : transition d'entrée adoucie en sinus, interpolation
//! entre images clés, événements d'images et de mouvements.

use std::collections::HashMap;
use std::f32::consts::{FRAC_PI_2, PI};
use std::rc::Rc;

use crate::assets::{Assets, CONTENT_SCALE, Frame};
use crate::data::json::{self, Json};
use crate::draw::Drawer;
use crate::geom::Affine;
use crate::node::{Kind, Node, Sprite};

/// Images par seconde des animations (intervalle du directeur, 1/60 s).
const ANIMATION_INTERVAL: f32 = 1.0 / 60.0;
/// « Pas d'adoucissement imposé » (TWEEN_EASING_MAX).
pub const EASING_MAX: i32 = 10000;
const LINEAR: i32 = 0;

// --- Données -----------------------------------------------------------------

/// Position, inclinaisons (radians), échelles et couleur (a, r, g, b).
#[derive(Clone, Copy, Debug)]
struct Base {
    x: f32,
    y: f32,
    skew_x: f32,
    skew_y: f32,
    scale_x: f32,
    scale_y: f32,
    color: [f32; 4],
    use_color: bool,
}

impl Default for Base {
    fn default() -> Base {
        Base { x: 0.0, y: 0.0, skew_x: 0.0, skew_y: 0.0, scale_x: 1.0, scale_y: 1.0, color: [255.0; 4], use_color: false }
    }
}

impl Base {
    fn read(j: &Json) -> Base {
        let mut b = Base { x: j.f32("x", 0.0), y: j.f32("y", 0.0), skew_x: j.f32("kX", 0.0), skew_y: j.f32("kY", 0.0), scale_x: j.f32("cX", 1.0), scale_y: j.f32("cY", 1.0), ..Base::default() };
        let c = j.get("color");
        if !matches!(c, Json::Null) {
            b.color = [c.f32("a", 255.0), c.f32("r", 255.0), c.f32("g", 255.0), c.f32("b", 255.0)];
            b.use_color = true;
        }
        b
    }

    /// CCBaseData::subtract : de `from` à `to`, inclinaisons ramenées
    /// dans ±π si `limit`.
    fn between(&mut self, from: &Base, to: &Base, limit: bool) {
        self.x = to.x - from.x;
        self.y = to.y - from.y;
        self.scale_x = to.scale_x - from.scale_x;
        self.scale_y = to.scale_y - from.scale_y;
        self.skew_x = to.skew_x - from.skew_x;
        self.skew_y = to.skew_y - from.skew_y;
        if self.use_color || from.use_color || to.use_color {
            for i in 0..4 {
                self.color[i] = to.color[i] - from.color[i];
            }
            self.use_color = true;
        } else {
            self.color = [0.0; 4];
            self.use_color = false;
        }
        if limit {
            for s in [&mut self.skew_x, &mut self.skew_y] {
                if *s > PI {
                    *s -= 2.0 * PI;
                }
                if *s < -PI {
                    *s += 2.0 * PI;
                }
            }
        }
    }

    /// CCTransformHelp::nodeToMatrix.
    fn matrix(&self) -> Affine {
        let (a, b, c, d) = if self.skew_x == -self.skew_y {
            let (s, k) = self.skew_x.sin_cos();
            (self.scale_x * k, self.scale_x * -s, self.scale_y * s, self.scale_y * k)
        } else {
            (self.scale_x * self.skew_y.cos(), self.scale_x * self.skew_y.sin(), self.scale_y * self.skew_x.sin(), self.scale_y * self.skew_x.cos())
        };
        Affine { a, b, c, d, tx: self.x, ty: self.y }
    }
}

#[derive(Clone, Debug)]
struct KeyFrame {
    base: Base,
    display: i32,
    z: i32,
    frame: i32,
    easing: i32,
    tween: bool,
    event: Option<String>,
}

struct MovementBone {
    delay: f32,
    frames: Vec<KeyFrame>,
}

struct Movement {
    duration: i32,
    looped: bool,
    duration_to: i32,
    duration_tween: i32,
    easing: i32,
    scale: f32,
    bones: HashMap<String, MovementBone>,
}

struct Display {
    /// L'image de feuille (nom avec « .png »).
    frame: String,
    skin: Base,
}

struct BoneData {
    name: String,
    base: Base,
    z: i32,
    displays: Vec<Display>,
}

/// Une armature lue dans un ExportJson.
pub struct ArmatureData {
    bones: Vec<BoneData>,
    movements: HashMap<String, Movement>,
    /// Le premier mouvement du fichier (pose de départ, movementNames[0]).
    first_movement: Option<String>,
    /// Point d'ancrage des images (pX, pY), par nom sans extension.
    pivots: HashMap<String, (f32, f32)>,
}

/// Les armatures d'un fichier ExportJson, par nom.
pub fn parse(text: &str) -> Vec<(String, Rc<ArmatureData>)> {
    let Some(root) = json::parse(text) else { return Vec::new() };
    let pivots: HashMap<String, (f32, f32)> = root.items("texture_data").iter().map(|t| (t.str("name").to_string(), (t.f32("pX", 0.5), t.f32("pY", 0.5)))).collect();
    let mut animations: HashMap<String, HashMap<String, Movement>> = HashMap::new();
    let mut firsts: HashMap<String, String> = HashMap::new();
    for a in root.items("animation_data") {
        if let Some(m) = a.items("mov_data").first() {
            firsts.insert(a.str("name").to_string(), m.str("name").to_string());
        }
        let movements = a
            .items("mov_data")
            .iter()
            .map(|m| {
                let bones = m
                    .items("mov_bone_data")
                    .iter()
                    .map(|b| {
                        let frames = b
                            .items("frame_data")
                            .iter()
                            .map(|f| KeyFrame {
                                base: Base::read(f),
                                display: f.i32("dI", 0),
                                z: f.i32("z", 0),
                                frame: f.i32("fi", 0),
                                easing: f.i32("twE", LINEAR),
                                tween: !matches!(f.get("tweenFrame"), Json::Bool(false)),
                                event: Some(f.str("evt").to_string()).filter(|e| !e.is_empty()),
                            })
                            .collect();
                        (b.str("name").to_string(), MovementBone { delay: b.f32("dl", 0.0), frames })
                    })
                    .collect();
                let movement = Movement {
                    duration: m.i32("dr", 0),
                    looped: m.bool("lp"),
                    duration_to: m.i32("to", 0),
                    duration_tween: m.i32("drTW", 0),
                    easing: m.i32("twE", LINEAR),
                    scale: m.f32("sc", 1.0),
                    bones,
                };
                (m.str("name").to_string(), movement)
            })
            .collect();
        animations.insert(a.str("name").to_string(), movements);
    }
    root.items("armature_data")
        .iter()
        .map(|a| {
            let name = a.str("name").to_string();
            let bones = a
                .items("bone_data")
                .iter()
                .map(|b| BoneData {
                    name: b.str("name").to_string(),
                    base: Base::read(b),
                    z: b.i32("z", 0),
                    displays: b
                        .items("display_data")
                        .iter()
                        .map(|d| Display { frame: d.str("name").to_string(), skin: d.items("skin_data").first().map(Base::read).unwrap_or_default() })
                        .collect(),
                })
                .collect();
            let data = ArmatureData { bones, movements: animations.remove(&name).unwrap_or_default(), first_movement: firsts.remove(&name), pivots: pivots.clone() };
            (name, Rc::new(data))
        })
        .collect()
}

// --- Déroulement -------------------------------------------------------------

/// Les modes de boucle de CCProcessBase (ordre de l'énumération d'origine).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Loop {
    SingleFrame,
    NoLoop,
    ToLoopFront,
    ToLoopBack,
    LoopFront,
    /// Jamais employé par le jeu ; gardé pour l'ordre d'origine.
    #[allow(dead_code)]
    LoopBack,
    Max,
}

/// CCProcessBase : l'avancement commun à l'animation et à chaque os.
#[derive(Clone, Debug)]
struct Process {
    scale: f32,
    complete: bool,
    paused: bool,
    playing: bool,
    current_percent: f32,
    raw_duration: i32,
    loop_type: Loop,
    tween_easing: i32,
    duration_tween: i32,
    current_frame: f32,
    next_frame_index: i32,
}

impl Default for Process {
    fn default() -> Process {
        Process { scale: 1.0, complete: true, paused: false, playing: false, current_percent: 0.0, raw_duration: 0, loop_type: Loop::NoLoop, tween_easing: LINEAR, duration_tween: 0, current_frame: 0.0, next_frame_index: 0 }
    }
}

impl Process {
    fn play(&mut self, duration_to: i32, easing: i32) {
        self.complete = false;
        self.paused = false;
        self.playing = true;
        self.current_frame = 0.0;
        self.next_frame_index = duration_to;
        self.tween_easing = easing;
    }

    /// CCProcessBase::gotoFrame, puis le pourcentage de cette image.
    fn goto_frame(&mut self, frame: i32, raw_duration: i32) {
        if self.loop_type == Loop::NoLoop {
            self.loop_type = Loop::Max;
        } else if self.loop_type == Loop::ToLoopFront {
            self.loop_type = Loop::LoopFront;
        }
        self.next_frame_index = self.duration_tween;
        self.playing = true;
        self.complete = false;
        self.paused = false;
        self.current_percent = frame as f32 / (raw_duration as f32 - 1.0);
        self.current_frame = self.next_frame_index as f32 * self.current_percent;
    }

    /// Rend vrai s'il faut appeler la suite (updateHandler).
    fn advance(&mut self, dt: f32) -> bool {
        if self.complete || self.paused || self.raw_duration <= 0 || dt > 1.0 {
            return false;
        }
        if self.next_frame_index <= 0 {
            self.current_percent = 1.0;
            self.current_frame = 0.0;
        } else {
            self.current_frame += self.scale * (dt / ANIMATION_INTERVAL);
            self.current_percent = self.current_frame / self.next_frame_index as f32;
            self.current_frame %= self.next_frame_index as f32;
        }
        true
    }
}

/// CCTween : l'animation d'un os.
#[derive(Clone, Debug)]
struct Tween {
    p: Process,
    /// Le mouvement de l'os (nom du mouvement), s'il en a un.
    movement: Option<String>,
    from: KeyFrame,
    between: KeyFrame,
    /// L'état relatif à l'os (CCTween::m_pTweenData).
    data: KeyFrame,
    frame_easing: i32,
    total_duration: i32,
    between_duration: i32,
    from_index: usize,
    to_index: usize,
    pass_last_frame: bool,
}

fn empty_frame() -> KeyFrame {
    KeyFrame { base: Base::default(), display: 0, z: 0, frame: 0, easing: LINEAR, tween: true, event: None }
}

/// Un os en cours.
#[derive(Clone, Debug)]
pub struct Bone {
    tween: Tween,
    /// Image affichée (-1 : aucune).
    display: i32,
    z: i32,
    /// Le nœud de l'os lui-même (déplacé par la logique : os « taichi »).
    pub pos: (f32, f32),
    /// Rotation du nœud de l'os : ajoutée à l'inclinaison, donc en radians.
    pub rotation: f32,
    pub color: [u8; 3],
    pub opacity: u8,
}

/// Ce que l'armature signale à la logique.
#[derive(Clone, Debug)]
pub enum ArmatureEvent {
    /// Début (0), fin (1) ou fin de boucle (2) d'un mouvement.
    Movement(i32, String),
    /// Événement d'image, par son nom.
    Frame(String),
}

/// Une armature en cours (le contenu d'un nœud).
#[derive(Clone)]
pub struct Armature {
    data: Rc<ArmatureData>,
    pub bones: Vec<Bone>,
    anim: Process,
    movement: Option<String>,
    speed: f32,
    events: Vec<ArmatureEvent>,
    ignore_frame_event: bool,
}

impl Armature {
    pub fn new(data: Rc<ArmatureData>) -> Armature {
        let bones = data
            .bones
            .iter()
            .map(|b| Bone {
                tween: Tween {
                    p: Process::default(),
                    movement: None,
                    from: empty_frame(),
                    between: empty_frame(),
                    data: KeyFrame { base: b.base, ..empty_frame() },
                    frame_easing: LINEAR,
                    total_duration: 0,
                    between_duration: 0,
                    from_index: 0,
                    to_index: 0,
                    pass_last_frame: false,
                },
                display: -1,
                z: b.z,
                pos: (0.0, 0.0),
                rotation: 0.0,
                color: [255; 3],
                opacity: 255,
            })
            .collect();
        Armature { data, bones, anim: Process::default(), movement: None, speed: 1.0, events: Vec::new(), ignore_frame_event: false }
    }

    /// CCArmature::init puis updateOffsetPoint : le rectangle englobant
    /// (x, y, largeur, hauteur) des images affichées à la première image du
    /// premier mouvement. init appelle update(0) avant la mesure : chaque
    /// image y est placée avec son os (CCSkin::updateArmatureTransform).
    /// CCArmature::boundingBox réunit les rectangles de tous les os sans
    /// écarter les vides : un os sans image affichée (dI négatif, os absent
    /// du mouvement) compte pour CCRect(0, 0, 0, 0)
    /// (CCDisplayManager::getBoundingBox sans nœud de rendu), ce qui tire le
    /// rectangle jusqu'à l'origine de l'armature.
    pub fn initial_bounds(&self, assets: &Assets) -> Option<[f32; 4]> {
        let movement = self.data.movements.get(self.data.first_movement.as_deref()?)?;
        let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
        for bd in &self.data.bones {
            let corners = (|| {
                let first = movement.bones.get(&bd.name)?.frames.first()?;
                let display = bd.displays.get(usize::try_from(first.display).ok()?)?;
                let frame = assets.frame(&display.frame)?;
                let mut abs = first.base;
                abs.x += bd.base.x;
                abs.y += bd.base.y;
                abs.skew_x += bd.base.skew_x;
                abs.skew_y += bd.base.skew_y;
                abs.scale_x += bd.base.scale_x - 1.0;
                abs.scale_y += bd.base.scale_y - 1.0;
                let (skin, _, (w, h)) = skin_transform(&self.data, display, &frame);
                let t = skin.then(&abs.matrix());
                Some([t.apply(0.0, 0.0), t.apply(w, 0.0), t.apply(0.0, h), t.apply(w, h)])
            })()
            .unwrap_or([(0.0, 0.0); 4]);
            for (x, y) in corners {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
        (x0 <= x1).then_some([x0, y0, x1 - x0, y1 - y0])
    }

    pub fn bone(&self, name: &str) -> Option<usize> {
        self.data.bones.iter().position(|b| b.name == name)
    }

    pub fn pause(&mut self) {
        self.anim.paused = true;
        for b in &mut self.bones {
            b.tween.p.paused = true;
        }
    }

    pub fn resume(&mut self) {
        self.anim.paused = false;
        for b in &mut self.bones {
            b.tween.p.paused = false;
        }
    }

    /// CCArmatureAnimation::play (-1 : valeur du mouvement ; `loop_` 0 :
    /// une fois, 1 : en boucle).
    pub fn play(&mut self, name: &str, duration_to: i32, duration_tween: i32, loop_: i32, easing: i32) {
        let data = self.data.clone();
        let Some(m) = data.movements.get(name) else { return };
        self.movement = Some(name.to_string());
        self.anim.raw_duration = m.duration;
        self.anim.scale = self.speed * m.scale;
        let duration_to = if duration_to == -1 { m.duration_to } else { duration_to };
        let duration_tween = if duration_tween == -1 { m.duration_tween } else { duration_tween };
        let duration_tween = if duration_tween == 0 { m.duration } else { duration_tween };
        let easing = if easing == EASING_MAX { m.easing } else { easing };
        let looped = if loop_ < 0 { m.looped } else { loop_ != 0 };
        self.anim.play(duration_to, easing);
        if m.duration == 0 {
            self.anim.loop_type = Loop::SingleFrame;
        } else {
            self.anim.loop_type = if looped { Loop::ToLoopFront } else { Loop::NoLoop };
            self.anim.duration_tween = duration_tween;
        }
        for (i, bd) in data.bones.iter().enumerate() {
            match m.bones.get(&bd.name).filter(|mb| !mb.frames.is_empty()) {
                Some(mb) => {
                    let scale = self.anim.scale;
                    self.tween_play(i, name, mb, m.duration, duration_to, duration_tween, looped, easing);
                    self.bones[i].tween.p.scale = scale;
                }
                None => {
                    // L'os n'a rien dans ce mouvement : caché.
                    self.bones[i].display = -1;
                    self.bones[i].tween.p.complete = true;
                    self.bones[i].tween.p.playing = false;
                    self.bones[i].tween.movement = None;
                }
            }
        }
        self.update(0.0);
    }

    /// CCArmatureAnimation::gotoAndPlay (sans événements d'image).
    pub fn goto_and_play(&mut self, frame: i32) {
        let duration = self.anim.raw_duration;
        if self.movement.is_none() || frame < 0 || frame >= duration {
            return;
        }
        let ignore = self.ignore_frame_event;
        self.ignore_frame_event = true;
        self.anim.goto_frame(frame, duration);
        for b in &mut self.bones {
            let t = &mut b.tween;
            t.p.goto_frame(frame, t.p.raw_duration);
            t.total_duration = 0;
            t.between_duration = 0;
            t.from_index = 0;
            t.to_index = 0;
        }
        let events = self.update(0.0);
        self.events = events.into_iter().filter(|e| matches!(e, ArmatureEvent::Movement(..))).collect();
        self.ignore_frame_event = ignore;
    }

    /// CCArmatureAnimation::gotoAndPause.
    pub fn goto_and_pause(&mut self, frame: i32) {
        self.goto_and_play(frame);
        self.pause();
    }

    #[allow(clippy::too_many_arguments)]
    fn tween_play(&mut self, i: usize, name: &str, mb: &MovementBone, duration: i32, duration_to: i32, duration_tween: i32, looped: bool, easing: i32) {
        let bone_base = self.data.bones[i].base;
        let t = &mut self.bones[i].tween;
        t.p.play(duration_to, easing);
        t.p.loop_type = if looped { Loop::ToLoopFront } else { Loop::NoLoop };
        t.total_duration = 0;
        t.between_duration = 0;
        t.from_index = 0;
        t.to_index = 0;
        let different = t.movement.as_deref() != Some(name);
        t.movement = Some(name.to_string());
        t.p.raw_duration = duration;
        let first = mb.frames[0].clone();
        t.data.display = first.display;
        // L'état devient relatif à l'os (nodeSub, échelles + 1).
        let d = &mut t.data.base;
        d.x -= bone_base.x;
        d.y -= bone_base.y;
        d.skew_x -= bone_base.skew_x;
        d.skew_y -= bone_base.skew_y;
        d.scale_x -= bone_base.scale_x - 1.0;
        d.scale_y -= bone_base.scale_y - 1.0;
        if duration == 0 {
            t.p.loop_type = Loop::SingleFrame;
            let from = if duration_to == 0 { first.clone() } else { t.data.clone() };
            self.set_between(i, &from, &first, true);
            self.bones[i].tween.frame_easing = LINEAR;
        } else if mb.frames.len() > 1 {
            t.p.duration_tween = duration_tween;
            if looped && mb.delay != 0.0 {
                let percent = self.update_frame_data(i, mb, 1.0 - mb.delay);
                let mut node = self.bones[i].tween.data.clone();
                self.tween_node_to(i, percent, Some(&mut node));
                let data = self.bones[i].tween.data.clone();
                self.set_between(i, &data, &node, true);
            } else {
                let from = if !different || duration_to == 0 { first.clone() } else { self.bones[i].tween.data.clone() };
                self.set_between(i, &from, &first, true);
            }
        }
        self.tween_node_to(i, 0.0, None);
    }

    /// CCTween::setBetween.
    fn set_between(&mut self, i: usize, from: &KeyFrame, to: &KeyFrame, limit: bool) {
        let t = &mut self.bones[i].tween;
        if from.display < 0 && to.display >= 0 {
            t.from = to.clone();
            t.between.base.between(&to.base, &to.base, limit);
        } else if to.display < 0 && from.display >= 0 {
            t.from = from.clone();
            t.between.base.between(&to.base, &to.base, limit);
        } else {
            t.from = from.clone();
            t.between.base.between(&from.base, &to.base, limit);
        }
        if !from.tween {
            t.data = from.clone();
            t.data.tween = true;
        }
        // arriveKeyFrame : image, ordre z.
        self.bones[i].display = from.display.max(-1);
        self.bones[i].tween.data.z = from.z;
        self.bones[i].z = from.z + self.data.bones[i].z;
    }

    /// CCTween::tweenNodeTo (dans l'état de l'os, ou dans `node`).
    fn tween_node_to(&mut self, i: usize, percent: f32, node: Option<&mut KeyFrame>) {
        let t = &mut self.bones[i].tween;
        let percent = if t.from.tween { percent } else { 0.0 };
        let (f, b) = (t.from.base, t.between.base);
        let n = match node {
            Some(n) => &mut n.base,
            None => &mut t.data.base,
        };
        n.x = f.x + percent * b.x;
        n.y = f.y + percent * b.y;
        n.scale_x = f.scale_x + percent * b.scale_x;
        n.scale_y = f.scale_y + percent * b.scale_y;
        n.skew_x = f.skew_x + percent * b.skew_x;
        n.skew_y = f.skew_y + percent * b.skew_y;
        if b.use_color {
            for k in 0..4 {
                n.color[k] = f.color[k] + percent * b.color[k];
            }
            n.use_color = true;
        }
    }

    /// CCTween::updateFrameData : l'image clé en cours et la fraction
    /// entre elle et la suivante.
    fn update_frame_data(&mut self, i: usize, mb: &MovementBone, percent: f32) -> f32 {
        let mut percent = percent;
        if percent > 1.0 && mb.delay != 0.0 {
            percent %= 1.0;
        }
        let raw = self.bones[i].tween.p.raw_duration;
        let played = (raw as f32 - 1.0) * percent;
        let frames = &mb.frames;
        let length = frames.len();
        let t = &self.bones[i].tween;
        if played < t.total_duration as f32 || played >= (t.total_duration + t.between_duration) as f32 {
            if played < frames[0].frame as f32 {
                self.set_between(i, &frames[0], &frames[0], true);
                return self.bones[i].tween.p.current_percent;
            }
            if played >= frames[length - 1].frame as f32 {
                if self.bones[i].tween.pass_last_frame {
                    self.set_between(i, &frames[length - 1], &frames[length - 1], true);
                    return self.bones[i].tween.p.current_percent;
                }
                self.bones[i].tween.pass_last_frame = true;
            } else {
                self.bones[i].tween.pass_last_frame = false;
            }
            let (mut from, mut to);
            loop {
                let t = &mut self.bones[i].tween;
                t.from_index = t.to_index;
                from = &frames[t.from_index];
                t.total_duration = from.frame;
                t.to_index = t.from_index + 1;
                if t.to_index >= length {
                    t.to_index = 0;
                }
                to = &frames[t.to_index];
                let from_index = t.from_index;
                let pass_last = t.pass_last_frame;
                if let Some(e) = &from.event
                    && !self.ignore_frame_event
                {
                    self.events.push(ArmatureEvent::Frame(e.clone()));
                }
                if played == from.frame as f32 || (pass_last && from_index == length - 1) {
                    break;
                }
                if !(played < from.frame as f32 || played >= to.frame as f32) {
                    break;
                }
            }
            let t = &mut self.bones[i].tween;
            t.between_duration = to.frame - from.frame;
            t.frame_easing = from.easing;
            let (from, to) = (from.clone(), to.clone());
            self.set_between(i, &from, &to, false);
        }
        let t = &self.bones[i].tween;
        let mut current = if t.between_duration == 0 { 0.0 } else { (played - t.total_duration as f32) / t.between_duration as f32 };
        let kind = if t.frame_easing != LINEAR { t.frame_easing } else { t.p.tween_easing };
        if kind != EASING_MAX && kind != LINEAR && !t.pass_last_frame {
            current = ease(kind, current);
        }
        current
    }

    /// CCTween::update pour un os.
    fn tween_update(&mut self, i: usize, dt: f32) {
        if !self.bones[i].tween.p.advance(dt) {
            return;
        }
        let data = self.data.clone();
        let Some(mb) = self.bones[i].tween.movement.as_ref().and_then(|m| data.movements.get(m)).and_then(|m| m.bones.get(&data.bones[i].name)) else { return };
        let t = &mut self.bones[i].tween;
        let p = &mut t.p;
        if p.current_percent >= 1.0 {
            match p.loop_type {
                Loop::SingleFrame | Loop::Max => {
                    p.current_percent = 1.0;
                    p.complete = true;
                    p.playing = false;
                }
                Loop::NoLoop => {
                    p.loop_type = Loop::Max;
                    p.current_percent = if p.duration_tween <= 0 { 1.0 } else { (p.current_percent - 1.0) * p.next_frame_index as f32 / p.duration_tween as f32 };
                    if p.current_percent >= 1.0 {
                        p.current_percent = 1.0;
                        p.complete = true;
                        p.playing = false;
                    } else {
                        p.next_frame_index = p.duration_tween;
                        p.current_frame = p.current_percent * p.next_frame_index as f32;
                        t.total_duration = 0;
                        t.between_duration = 0;
                        t.from_index = 0;
                        t.to_index = 0;
                    }
                }
                Loop::ToLoopFront => {
                    p.loop_type = Loop::LoopFront;
                    p.next_frame_index = if p.duration_tween > 0 { p.duration_tween } else { 1 };
                    if mb.delay != 0.0 {
                        p.current_frame = (1.0 - mb.delay) * p.next_frame_index as f32;
                        p.current_percent = p.current_frame / p.next_frame_index as f32;
                    } else {
                        p.current_percent = 0.0;
                        p.current_frame = 0.0;
                    }
                    t.total_duration = 0;
                    t.between_duration = 0;
                    t.from_index = 0;
                    t.to_index = 0;
                }
                _ => p.current_frame %= p.next_frame_index.max(1) as f32,
            }
        }
        if p.current_percent < 1.0 && p.loop_type <= Loop::ToLoopBack {
            p.current_percent = (p.current_percent * FRAC_PI_2).sin();
        }
        let mut percent = p.current_percent;
        if p.loop_type > Loop::ToLoopBack {
            percent = self.update_frame_data(i, mb, percent);
        }
        if self.bones[i].tween.frame_easing != EASING_MAX {
            self.tween_node_to(i, percent, None);
        }
    }

    /// Une image d'animation (CCArmatureAnimation::update) ; rend les
    /// événements à transmettre.
    pub fn update(&mut self, dt: f32) -> Vec<ArmatureEvent> {
        if self.anim.advance(dt) {
            self.animation_handler();
        }
        for i in 0..self.bones.len() {
            self.tween_update(i, dt);
        }
        std::mem::take(&mut self.events)
    }

    /// CCArmatureAnimation::updateHandler.
    fn animation_handler(&mut self) {
        let id = self.movement.clone().unwrap_or_default();
        let p = &mut self.anim;
        if p.current_percent < 1.0 {
            return;
        }
        match p.loop_type {
            Loop::NoLoop => {
                p.loop_type = Loop::Max;
                p.current_frame = (p.current_percent - 1.0) * p.next_frame_index as f32;
                p.current_percent = p.current_frame / p.duration_tween.max(1) as f32;
                if p.current_percent < 1.0 {
                    p.next_frame_index = p.duration_tween;
                    self.events.push(ArmatureEvent::Movement(0, id));
                    return;
                }
                p.current_percent = 1.0;
                p.complete = true;
                p.playing = false;
                self.events.push(ArmatureEvent::Movement(1, id));
            }
            Loop::Max | Loop::SingleFrame => {
                p.current_percent = 1.0;
                p.complete = true;
                p.playing = false;
                self.events.push(ArmatureEvent::Movement(1, id));
            }
            Loop::ToLoopFront => {
                p.loop_type = Loop::LoopFront;
                p.current_percent %= 1.0;
                p.current_frame = if p.next_frame_index == 0 { 0.0 } else { p.current_frame % p.next_frame_index as f32 };
                p.next_frame_index = if p.duration_tween > 0 { p.duration_tween } else { 1 };
                self.events.push(ArmatureEvent::Movement(0, id));
            }
            _ => {
                p.current_frame %= p.next_frame_index.max(1) as f32;
                self.events.push(ArmatureEvent::Movement(2, id));
            }
        }
    }

    /// Le dessin des os, par ordre z (CCArmature::draw).
    pub fn draw(&self, drawer: &mut Drawer, world: &Affine, rgba: [f32; 4]) {
        let mut order: Vec<usize> = (0..self.bones.len()).collect();
        order.sort_by_key(|&i| self.bones[i].z);
        for i in order {
            let bone = &self.bones[i];
            let bd = &self.data.bones[i];
            let Some(display) = usize::try_from(bone.display).ok().and_then(|d| bd.displays.get(d)) else { continue };
            let Some(frame) = drawer.assets.frame(&display.frame) else { continue };
            // L'état absolu de l'os (relatif + données de l'os).
            let mut abs = bone.tween.data.base;
            abs.x += bd.base.x;
            abs.y += bd.base.y;
            abs.skew_x += bd.base.skew_x;
            abs.skew_y += bd.base.skew_y;
            abs.scale_x += bd.base.scale_x - 1.0;
            abs.scale_y += bd.base.scale_y - 1.0;
            // Le nœud de l'os (déplacé par la logique) s'ajoute à l'animation ;
            // sa rotation va dans l'inclinaison, en radians (CCBone::update).
            abs.x += bone.pos.0;
            abs.y += bone.pos.1;
            abs.skew_x += bone.rotation;
            abs.skew_y -= bone.rotation;
            let bone_world = abs.matrix();
            let skin = skin_transform(&self.data, display, &frame);
            let full = skin.0.then(&bone_world).then(world);
            let c = abs.color;
            let k = [bone.color[0], bone.color[1], bone.color[2]].map(|v| f32::from(v) / 255.0);
            let a = rgba[3] * (c[0] / 255.0).clamp(0.0, 1.0) * f32::from(bone.opacity) / 255.0;
            let base = if rgba[3] > 0.0 { [rgba[0] / rgba[3], rgba[1] / rgba[3], rgba[2] / rgba[3]] } else { [0.0; 3] };
            let col = [0, 1, 2].map(|j| base[j] * k[j] * (c[j + 1] / 255.0).clamp(0.0, 1.0) * a);
            drawer.sprite(&skin.1, &full, skin.2, [col[0], col[1], col[2], a]);
        }
    }
}

/// L'image d'un os comme un sprite (CCSkin) : sa transformation dans
/// l'os, le sprite et sa taille.
fn skin_transform(data: &ArmatureData, display: &Display, frame: &Rc<Frame>) -> (Affine, Sprite, (f32, f32)) {
    let (sprite, size) = Sprite::frame(frame);
    let key = display.frame.strip_suffix(".png").unwrap_or(&display.frame);
    let mut n = Node::new(Kind::Node);
    n.size = size;
    n.anchor = data.pivots.get(key).copied().unwrap_or((0.5, 0.5));
    n.pos = (display.skin.x, display.skin.y);
    n.scale = (display.skin.scale_x, display.skin.scale_y);
    n.rotation = (display.skin.skew_x.to_degrees(), (-display.skin.skew_y).to_degrees());
    let _ = CONTENT_SCALE;
    (n.transform(), sprite, size)
}

/// CCTweenFunction::tweenTo, les adoucissements utilisés par le jeu.
fn ease(kind: i32, t: f32) -> f32 {
    match kind {
        1 => -(t * FRAC_PI_2).cos() + 1.0,
        2 => (t * FRAC_PI_2).sin(),
        3 => -0.5 * ((PI * t).cos() - 1.0),
        4 => t * t,
        5 => -t * (t - 2.0),
        6 => {
            let t = t * 2.0;
            if t < 1.0 { 0.5 * t * t } else { -0.5 * ((t - 1.0) * (t - 3.0) - 1.0) }
        }
        7 => t * t * t,
        8 => {
            let t = t - 1.0;
            t * t * t + 1.0
        }
        9 => {
            let t = t * 2.0;
            if t < 1.0 { 0.5 * t * t * t } else { 0.5 * ((t - 2.0).powi(3) + 2.0) }
        }
        _ => t,
    }
}

impl Assets {
    /// Charge les armatures d'un ExportJson (addArmatureFileInfo).
    pub fn add_armature_file(&self, name: &str) {
        let Some(text) = self.apk.text(name) else { return };
        let mut store = self.armatures.borrow_mut();
        for (n, data) in parse(&text) {
            store.entry(n).or_insert(data);
        }
    }

    pub fn armature(&self, name: &str) -> Option<Rc<ArmatureData>> {
        self.armatures.borrow().get(name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JSON: &str = r#"{"armature_data":[{"name":"t","bone_data":[{"name":"b","parent":"","x":0,"y":0,"z":1,"cX":1,"cY":1,"kX":0,"kY":0,
        "display_data":[{"name":"i.png","displayType":0,"skin_data":[{"x":0,"y":0,"cX":1,"cY":1,"kX":0,"kY":0}]}]}]}],
        "animation_data":[{"name":"t","mov_data":[{"name":"m","dr":10,"lp":false,"to":0,"drTW":0,"twE":0,"sc":1,
        "mov_bone_data":[{"name":"b","dl":0,"frame_data":[
            {"dI":0,"x":0,"y":0,"z":0,"cX":1,"cY":1,"kX":0,"kY":0,"fi":0,"twE":0,"tweenFrame":true},
            {"dI":0,"x":90,"y":0,"z":0,"cX":1,"cY":1,"kX":0,"kY":0,"fi":9,"twE":0,"tweenFrame":true,"evt":"end"}]}]}]}],
        "texture_data":[{"name":"i","width":10,"height":10,"pX":0.5,"pY":0.5}]}"#;

    #[test]
    fn plays_once() {
        let data = parse(JSON).remove(0).1;
        let mut a = Armature::new(data);
        a.play("m", -1, -1, -1, EASING_MAX);
        assert_eq!(a.bones[0].display, 0);
        let mut events = Vec::new();
        let mut xs = Vec::new();
        for _ in 0..12 {
            events.extend(a.update(1.0 / 60.0));
            xs.push(a.bones[0].tween.data.base.x);
        }
        // 10 images, de 0 à 90 : l'image jouée est (durée − 1) × avancement.
        assert!((xs[0] - 9.0).abs() < 1e-3, "{xs:?}");
        assert!((xs[9] - 90.0).abs() < 1e-3, "{xs:?}");
        assert!(events.iter().any(|e| matches!(e, ArmatureEvent::Frame(n) if n == "end")));
        assert!(events.iter().any(|e| matches!(e, ArmatureEvent::Movement(1, _))));
    }
}
