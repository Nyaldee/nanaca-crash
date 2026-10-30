//! Les personnages croisés en vol (GameCast et ses classes : Youko,
//! Tomoki, Sakuraba, Misato, Kiri, Miki, Touko, et le blocage de Youko),
//! et Nanaka (élan d'ouverture et attaques aériennes).

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::action::Action;
use crate::armature::{ArmatureEvent, EASING_MAX};
use crate::game::Game;
use crate::node::NodeId;

use super::{control, effects, hud, play};

/// La classe du personnage.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Who {
    Nanaca,
    Youko,
    Tomoki,
    Sakuraba,
    Misato,
    Kiri,
    Miki,
    Touko,
    YoukoBlock,
}

/// Un personnage du tableau (indice) ou le blocage de Youko.
#[derive(Clone, Copy)]
enum Owner {
    Cast(usize),
    Block,
}

/// Le déplacement d'un os à une image de l'animation (SubMove) : vitesse
/// par image (x, y, rotation en degrés) et décalage unique.
#[derive(Clone, Copy, Default)]
struct SubMove {
    vx: f32,
    vy: f32,
    rot: f32,
    dx: f32,
    dy: f32,
}

impl SubMove {
    fn new(vx: f32, vy: f32, rot: f32, dx: f32, dy: f32) -> SubMove {
        SubMove { vx, vy, rot, dx, dy }
    }
}

/// Un personnage (champs de GameCast).
pub struct Cast {
    pub who: Who,
    pub node: NodeId,
    pub armature: NodeId,
    pub tag: i32,
    /// Position en scène (mètres) et décalage d'affichage (0x134).
    pub stage_pos: f32,
    pub offset: f32,
    /// Relance du crash : puissance et angle (0x110, 0x114).
    pub boost: (f32, f32),
    /// Zone de contact.
    hit_x: f32,
    hit_y: f32,
    hit_min: f32,
    hit_max: f32,
    hit_top: f32,
    /// Le personnage suivant (tag), pour onNextChar.
    pub next_tag: i32,
    /// En scène, dépassé, encore touchable, posé (0x13c..0x13f).
    pub active: bool,
    pub passed: bool,
    pub hittable: bool,
    pub in_stage: bool,
    /// Chrono du spécial en cours (0x140), bloqué par Youko (0x141).
    pub special_pending: bool,
    blocked: bool,
    /// État : -1, 0 crash, 1 spécial, 2 combo, 3 bloqué, 4 contre-attaque.
    pub state: i32,
    /// Les images de l'animation en cours (0x150, 0x154) et les
    /// déplacements de l'os de Taichi (carte image → SubMove).
    sub_running: bool,
    sub_frame: i32,
    moves: BTreeMap<i32, SubMove>,
    sub: SubMove,
    /// Les os suivis : « taichi », ou les currys de Sakuraba.
    bones: Vec<usize>,
    /// Les rappels d'animation déjà branchés (fin de mouvement, images).
    movement_cb: bool,
    frame_cb: bool,
    /// Vitesses des currys de Sakuraba (0x198, 0x19c, 0x1a0).
    curry: [f32; 3],
    /// Le personnage que le blocage de Youko protège (0x198).
    target: Option<usize>,
    /// Angle de la jauge (Nanaka : rotation de l'os au choc).
    pub meter_angle: f32,
}

impl Cast {
    /// Un personnage vide (avant la création des couches).
    pub fn placeholder(node: NodeId) -> Cast {
        Cast {
            who: Who::Nanaca,
            node,
            armature: node,
            tag: 0x69,
            stage_pos: -1.0,
            offset: 0.0,
            boost: (0.0, 0.0),
            hit_x: 0.5,
            hit_y: 0.5,
            hit_min: 0.0,
            hit_max: 0.0,
            hit_top: 0.0,
            next_tag: 0,
            active: false,
            passed: false,
            hittable: false,
            in_stage: false,
            special_pending: false,
            blocked: false,
            state: -1,
            sub_running: false,
            sub_frame: 0,
            moves: BTreeMap::new(),
            sub: SubMove::default(),
            bones: Vec::new(),
            movement_cb: false,
            frame_cb: false,
            curry: [0.0; 3],
            target: None,
            meter_angle: 0.0,
        }
    }

    /// GameCast::init : nœud 64 × 160, caché, l'armature (étiquette 999).
    fn new(game: &mut Game, who: Who, armature_name: &str, tag: i32) -> Cast {
        let node = game.g.layer((64.0, 160.0));
        game.set_visible(node, false);
        let armature = game.armature(armature_name);
        game.add(node, armature, 1, 999);
        let offset = (f64::from(game.design.0) * 0.5 - 50.0) * 0.01;
        if let Some(n) = game.g.get_mut(node) {
            n.tag = tag;
        }
        Cast { who, node, armature, tag, offset: offset as f32, ..Cast::placeholder(node) }
    }

    /// Ajoute un os suivi (s'il existe).
    fn track_bone(&mut self, game: &mut Game, name: &str) {
        if let Some(b) = game.armature_mut(self.armature).and_then(|a| a.bone(name)) {
            self.bones.push(b);
        }
    }
}

/// Nanaka (tag 0x69).
pub fn create_nanaca(game: &mut Game) -> Cast {
    let mut c = Cast::new(game, Who::Nanaca, "nanaca", 0x69);
    c.offset = 0.0;
    game.set_pos(c.armature, 0.0, 0.0);
    c
}

/// Les personnages du tableau, dans l'ordre de SceneGameDelegate (Youko,
/// Tomoki, Sakuraba, Misato, Kiri, Miki, Touko) : le `k`-ième.
pub fn create(game: &mut Game, who: Who, k: usize) -> Cast {
    let c = match who {
        Who::Youko => {
            let mut c = Cast::new(game, who, "youko", 0x6d);
            (c.hit_x, c.hit_y) = (-0.25, 1.25);
            c
        }
        Who::Tomoki => {
            let mut c = Cast::new(game, who, "tomoki", 0x70);
            (c.hit_x, c.hit_y) = (-0.25, 1.5);
            c
        }
        Who::Sakuraba => {
            let mut c = Cast::new(game, who, "sakuraba", 0x6f);
            (c.hit_x, c.hit_y) = (-0.25, 1.5);
            for bone in ["curry1a", "curry2", "curry1b"] {
                c.track_bone(game, bone);
            }
            c
        }
        Who::Misato => {
            let mut c = Cast::new(game, who, "misato", 0x6e);
            (c.hit_x, c.hit_y) = (-0.125, 1.0);
            c.track_bone(game, "taichi");
            c
        }
        Who::Kiri => {
            let mut c = Cast::new(game, who, "kiri", 0x6b);
            (c.hit_x, c.hit_y, c.boost) = (0.25, 1.25, (10.0, 45.0));
            c.track_bone(game, "taichi");
            c
        }
        Who::Miki => {
            let mut c = Cast::new(game, who, "miki", 0x6c);
            (c.hit_x, c.hit_y, c.boost) = (0.0, 1.25, (11.0, 70.0));
            c.track_bone(game, "taichi");
            c
        }
        _ => {
            let mut c = Cast::new(game, Who::Touko, "touko", 0x6a);
            (c.hit_x, c.hit_y, c.boost) = (0.25, 1.25, (11.0, 20.0));
            c.track_bone(game, "taichi");
            effects::setup_touko_effect(game, c.node);
            c
        }
    };
    listen(game, c.armature, Owner::Cast(k));
    c
}

/// Le blocage de Youko (tag 0x71), hors du tableau.
pub fn create_youko_block(game: &mut Game) -> Cast {
    let mut c = Cast::new(game, Who::YoukoBlock, "youko_block", 0x71);
    c.stage_pos = -1000.0;
    listen(game, c.armature, Owner::Block);
    c
}

fn listen(game: &mut Game, armature: NodeId, owner: Owner) {
    game.listen_armature(armature, Rc::new(move |game: &mut Game, _, e| on_armature(game, owner, e)));
}

fn get(game: &mut Game, owner: Owner) -> &mut Cast {
    match owner {
        Owner::Cast(k) => &mut play(game).casts[k],
        Owner::Block => &mut play(game).block,
    }
}

/// Un son joué dans une suite d'actions (CallFunPlayEffect).
fn sound(path: &'static str) -> Action {
    Action::call(move |game: &mut Game, _| {
        game.play_effect(path);
    })
}

/// GameCast::setStagePosition.
fn stage(c: &mut Cast, pos: f32) {
    c.active = true;
    c.stage_pos = pos;
    c.in_stage = true;
    c.hittable = true;
    c.passed = false;
    c.state = -1;
    c.hit_min = (pos - c.hit_x) - 0.5;
    c.hit_max = c.hit_x + 0.5 + pos;
    c.hit_top = c.hit_y + 0.5;
}

pub fn set_stage_position(game: &mut Game, k: usize, pos: f32) {
    stage(&mut play(game).casts[k], pos);
}

/// setDemonstrationParams : le personnage quitte la scène, caché.
pub fn demo_clear(game: &mut Game, k: usize) {
    let c = &mut play(game).casts[k];
    c.in_stage = false;
    c.stage_pos = -1000.0;
    let node = c.node;
    game.set_visible(node, false);
}

/// setDemonstrationParams : le blocage de Youko reprend « normal », caché
/// et hors scène.
pub fn demo_clear_block(game: &mut Game) {
    let (node, armature) = (play(game).block.node, play(game).block.armature);
    if let Some(a) = game.armature_mut(armature) {
        a.play("normal", -1, -1, 0, 10000);
    }
    game.set_visible(node, false);
    play(game).block.stage_pos = -1000.0;
}

/// GameCast::resetArmature : « normal » arrêté à sa première image.
pub fn reset(game: &mut Game, k: usize) {
    let armature = play(game).casts[k].armature;
    if let Some(a) = game.armature_mut(armature) {
        a.play("normal", -1, -1, 0, 10000);
        a.goto_and_pause(0);
    }
}

/// GameCast::playAnimatioinOnce : les rappels ne sont branchés que s'ils
/// sont donnés (les anciens restent sinon), puis l'animation une fois.
fn play_once(game: &mut Game, owner: Owner, name: &str, movement: bool, frame: bool, start: i32) {
    let c = get(game, owner);
    c.movement_cb |= movement;
    c.frame_cb |= frame;
    let armature = c.armature;
    if let Some(a) = game.armature_mut(armature) {
        a.play(name, -1, -1, 0, 10000);
        a.goto_and_play(start);
    }
}

/// Les images de l'animation repartent de zéro (cbMotionSpecial…).
fn start_frames(c: &mut Cast) {
    c.sub_running = true;
    c.sub_frame = 0;
    c.sub = SubMove::default();
}

fn hide_taichi(game: &mut Game) {
    let t = play(game).taichi.node;
    game.set_visible(t, false);
    play(game).taichi.hidden = true;
}

fn show_taichi(game: &mut Game) {
    let t = play(game).taichi.node;
    game.set_visible(t, true);
    play(game).taichi.hidden = false;
}

/// GameCast::positionTaichiBone : l'os « taichi » prend la place, la
/// rotation (celle de la couche, toujours nulle) et la couleur de Taichi.
fn position_taichi_bone(game: &mut Game, k: usize) {
    let (px, py) = (play(game).c.px, play(game).c.py);
    let t = play(game).taichi;
    let rotation = game.rotation(t.node);
    let color = game.g.get(t.sprite).map_or([0xFF; 3], |n| n.color);
    let c = &play(game).casts[k];
    let (armature, pos) = (c.armature, c.stage_pos);
    let Some(&b) = c.bones.first() else { return };
    if let Some(a) = game.armature_mut(armature) {
        let bone = &mut a.bones[b];
        bone.pos = ((px - pos) * 100.0, py * 100.0);
        bone.rotation = rotation * 0.017_453_292;
        bone.color = color;
    }
}

/// La rotation de l'os « taichi » ramenée en degrés.
fn taichi_bone_degrees(game: &mut Game, k: usize) -> f32 {
    let c = &play(game).casts[k];
    let (armature, bone) = (c.armature, c.bones.first().copied());
    let rotation = bone.and_then(|b| game.armature_mut(armature).map(|a| a.bones[b].rotation)).unwrap_or(0.0);
    rotation * 57.295_78
}

/// GameCast::onSubFrameTaichi : à l'image exacte d'une entrée, la vitesse
/// de l'os change ; il avance de cette vitesse (et du décalage unique).
fn sub_frame_taichi(game: &mut Game, k: usize) {
    let c = &mut play(game).casts[k];
    let Some(&b) = c.bones.first() else { return };
    if let Some(m) = c.moves.get(&c.sub_frame) {
        c.sub = *m;
    }
    let m = c.sub;
    c.sub.dx = 0.0;
    c.sub.dy = 0.0;
    let armature = c.armature;
    if let Some(a) = game.armature_mut(armature) {
        let bone = &mut a.bones[b];
        bone.pos = (m.dx + m.vx + bone.pos.0, m.dy + m.vy + bone.pos.1);
        bone.rotation += m.rot * 0.017_453_292;
    }
}

/// GameCast::on_special_frames : une image d'animation par image de jeu
/// écoulée.
fn on_special_frames(game: &mut Game, k: usize, dt: f32) {
    let n = (dt / crate::game::FRAME).round() as i32;
    for _ in 0..n {
        enter_sub_frame(game, k);
        play(game).casts[k].sub_frame += 1;
    }
}

// --- GameCast::moveCast ----------------------------------------------------------------------

/// GameCast::moveCast : la place à l'écran, le visage qui approche, le
/// dépassement, et le contact avec Taichi (crash, spécial, blocage, combo).
pub fn move_cast(game: &mut Game, k: usize, px: f32, py: f32, _r: f32, dt: f32) {
    let c = &play(game).casts[k];
    if !c.in_stage || !c.active {
        return;
    }
    let (node, pos, offset, tag) = (c.node, c.stage_pos, c.offset, c.tag);
    game.set_x(node, ((pos - px) + offset) * 100.0);
    if play(game).casts[k].sub_running {
        on_special_frames(game, k, dt);
    }
    if play(game).casts[k].passed {
        let x = game.pos(node).0;
        game.set_visible(node, -1350.0 < x);
        return;
    }
    game.set_visible(node, true);
    if pos - px < 30.0 && tag != 0x71 {
        hud::contact_view(game, node, tag, pos - px);
    }
    if pos - px < -1.0 {
        play(game).casts[k].passed = true;
        let c = &mut play(game).c;
        if !c.nanaca_first {
            c.nanaca_first = true;
            c.nanaca = true;
        } else {
            c.nanaca = pos % 100.0 == 90.0;
        }
        pass_action(game, k);
        return;
    }
    if !play(game).casts[k].special_pending {
        let c = &play(game).casts[k];
        if !(c.hit_min < px && px < c.hit_max && py < c.hit_top && c.hittable) {
            return;
        }
        play(game).casts[k].hittable = false;
        control::slow_with_interval(game, 1000.0, 0.0);
        let ctl = &play(game).c;
        if ctl.combo {
            let c = &mut play(game).casts[k];
            c.state = 2;
            start_frames(c);
            play(game).c.combo = false;
            motion_combo(game, k);
        } else if ctl.youko_block && tag != 0x6d {
            play(game).casts[k].state = 3;
            motion_block(game, k);
        } else if !special(game, k) {
            play(game).casts[k].state = 0;
            motion_crash(game, k);
        } else if !play(game).c.stop {
            play(game).casts[k].state = -1;
            let layer = play(game).hud.special;
            let id = game.play_effect("seogg/se18.ogg");
            play(game).special_sound = id;
            play(game).casts[k].special_pending = true;
            game.set_visible(layer, true);
            hud::special_start(game);
            let c = &mut play(game).c;
            c.special_timer = 1.0;
            c.special_state = -1;
        } else {
            let layer = play(game).hud.special;
            play(game).c.special_state = 1;
            play(game).casts[k].special_pending = true;
            game.set_visible(layer, true);
        }
        return;
    }
    // Le chrono du spécial a donné son résultat.
    let result = play(game).c.special_state;
    let countering = play(game).c.nanaca && play(game).casts[k].state == 4;
    if result == 0 {
        hud::special_ng(game);
        play(game).casts[k].special_pending = false;
        if countering {
            play(game).casts[k].state = 3;
            unpause_special(game);
        } else {
            play(game).casts[k].state = 0;
            motion_crash(game, k);
        }
    } else if result == 1 {
        let id = play(game).special_sound;
        game.audio.stop_effect(id);
        game.play_effect("seogg/se08.ogg");
        hud::special_ok(game);
        play(game).casts[k].special_pending = false;
        let then: Action = if countering {
            play(game).casts[k].state = 3;
            Action::call(move |game: &mut Game, _| cb_motion_cff(game, k))
        } else {
            play(game).casts[k].state = 1;
            Action::call(move |game: &mut Game, _| cb_motion_special(game, k))
        };
        game.run(node, Action::sequence(vec![Action::delay(1.0), then]));
    }
}

/// GameCast::cbMotionSpecial : le spécial, une seconde après le PRESS.
fn cb_motion_special(game: &mut Game, k: usize) {
    start_frames(&mut play(game).casts[k]);
    motion_special(game, k);
}

/// GameCast::cbMotionCFF : la contre-attaque de Youko (champ de force).
fn cb_motion_cff(game: &mut Game, k: usize) {
    start_frames(&mut play(game).casts[k]);
    block_motion_special(game, k);
}

// --- Les méthodes de chaque personnage -------------------------------------------------------

/// special : le spécial de ce personnage est-il disponible ?
fn special(game: &mut Game, k: usize) -> bool {
    let p = play(game);
    match p.casts[k].who {
        Who::Miki => p.c.miki,
        Who::Kiri => p.c.kiri,
        Who::Misato => p.c.misato,
        Who::Touko => p.c.touko,
        _ => false,
    }
}

/// onNextChar : Miki devant Kiri, Kiri devant Miki rendent le spécial
/// possible.
pub fn on_next_char(game: &mut Game, k: usize) {
    let c = &play(game).casts[k];
    let (who, next) = (c.who, c.next_tag);
    match who {
        Who::Miki if next == 0x6b => play(game).c.miki = true,
        Who::Kiri if next == 0x6c => play(game).c.kiri = true,
        _ => {}
    }
}

fn pass_action(game: &mut Game, k: usize) {
    match play(game).casts[k].who {
        Who::Miki => play(game).c.miki = false,
        Who::Kiri => play(game).c.kiri = false,
        _ => {}
    }
}

/// motionBlock : Youko protège le personnage ; Tomoki et Sakuraba
/// tombent quand même.
fn motion_block(game: &mut Game, k: usize) {
    match play(game).casts[k].who {
        Who::Miki | Who::Kiri | Who::Misato | Who::Touko => block_motion_defence(game, k),
        Who::Tomoki => {
            game.play_effect("voiceogg/youko02.ogg");
            block_motion_defence(game, k);
            play(game).casts[k].blocked = true;
            motion_crash(game, k);
        }
        Who::Sakuraba => {
            block_motion_defence(game, k);
            game.play_effect("voiceogg/youko02.ogg");
            let c = &mut play(game).casts[k];
            c.state = 0;
            c.blocked = true;
            motion_crash(game, k);
        }
        _ => {}
    }
}

/// motionCrash : le crash de chaque personnage.
fn motion_crash(game: &mut Game, k: usize) {
    let owner = Owner::Cast(k);
    match play(game).casts[k].who {
        Who::Miki | Who::Kiri => {
            let kiri = play(game).casts[k].who == Who::Kiri;
            if kiri {
                play(game).casts[k].boost = (10.0, 45.0);
            }
            let (p, a) = play(game).casts[k].boost;
            control::boost(game, p, a);
            play_once(game, owner, "normal", true, false, 0);
            control::slow_with_distance(game, 50.0, 1.0);
            let c = &mut play(game).c;
            c.touko = false;
            c.misato = true;
            game.play_effect(if kiri { "voiceogg/kiri05.ogg" } else { "voiceogg/miki05.ogg" });
            game.play_effect("seogg/se01.ogg");
        }
        Who::Touko => {
            play_once(game, owner, "normal", true, false, 0);
            play(game).casts[k].boost = (11.0, 20.0);
            control::slow_with_distance(game, 50.0, 1.0);
            control::boost(game, 11.0, 20.0);
            let c = &mut play(game).c;
            c.touko = true;
            c.misato = true;
            game.play_effect("voiceogg/touko02.ogg");
            game.play_effect("seogg/se01.ogg");
        }
        Who::Misato => {
            // Misato rattrape Taichi : la partie s'arrête à la fin de son
            // animation (sauf champ de force B).
            let sprite = play(game).taichi.sprite;
            game.set_opacity(sprite, 0);
            control::slow_with_interval(game, 1000.0, 0.0);
            play_once(game, owner, "normal", true, false, 0);
            game.play_effect("seogg/se15.ogg");
        }
        Who::Youko => {
            play_once(game, owner, "normal", true, false, 0);
            control::slow_with_distance(game, 55.0, 1.0);
            let c = &mut play(game).c;
            c.nanaca = false;
            c.youko_block = true;
            c.touko = false;
            let node = play(game).casts[k].node;
            game.run(node, Action::sequence(vec![sound("seogg/se15.ogg"), Action::delay(0.3), sound("voiceogg/youko03.ogg"), Action::delay(0.3), sound("seogg/se06.ogg")]));
        }
        Who::Tomoki => {
            let c = &mut play(game).c;
            c.vx *= 0.7;
            c.vy = c.vy.abs() * 0.7;
            game.play_effect("seogg/se14.ogg");
            control::slow_with_distance(game, 50.0, 1.0);
            play_once(game, owner, "normal", true, false, 0);
            count_tomoki_sakuraba(game);
            fall_voice(game, k, "voiceogg/tomoki02.ogg", "voiceogg/tomoki01.ogg");
        }
        Who::Sakuraba => sakuraba_crash(game, k),
        Who::Nanaca | Who::YoukoBlock => {}
    }
}

/// Tomoki et Sakuraba : dix chutes dans la partie, un succès.
fn count_tomoki_sakuraba(game: &mut Game) {
    let c = &mut play(game).c;
    c.fallen += 1;
    let earned = !c.demo && c.fallen == 10;
    unlock_if(game, earned, 0x15);
}

/// Un succès si la condition est vraie.
fn unlock_if(game: &mut Game, on: bool, index: i32) {
    if on {
        game.unlock(index);
    }
}

/// La voix 0.3 s après la chute (une autre si Youko l'a bloqué).
fn fall_voice(game: &mut Game, k: usize, normal: &'static str, blocked: &'static str) {
    let c = &mut play(game).casts[k];
    let voice = if c.blocked { blocked } else { normal };
    c.blocked = false;
    let node = c.node;
    game.run(node, Action::sequence(vec![Action::delay(0.3), sound(voice)]));
}

/// Sakuraba::motionCrash : Taichi repart vers le haut, les currys volent.
fn sakuraba_crash(game: &mut Game, k: usize) {
    let c = &mut play(game).c;
    let (vx, vy) = (c.vx, c.vy);
    let speed = (vx * vx + vy * vy).sqrt();
    let angle = 10.0 + ((45.0 + vy.atan2(vx).abs() * 57.295_826) % 90.0) * 0.777_777_8;
    let a = angle * 0.017_453_28;
    c.vx = a.cos() * speed;
    c.vy = a.sin() * speed;
    play_once(game, Owner::Cast(k), "normal", true, false, 0);
    let armature = play(game).casts[k].armature;
    if let Some(arm) = game.armature_mut(armature) {
        let (up, down) = (arm.bone("angle_up"), arm.bone("angle_down"));
        for b in [up, down].into_iter().flatten() {
            arm.bones[b].opacity = 0;
        }
        if let Some(b) = if angle < 45.0 { down } else { up } {
            arm.bones[b].opacity = 0xFF;
        }
    }
    game.play_effect("seogg/se14.ogg");
    control::slow_with_distance(game, 50.0, 1.0);
    let r = [game.rng.lrand48(), game.rng.lrand48(), game.rng.lrand48()].map(|v| v as f32);
    let c = &mut play(game).casts[k];
    start_frames(c);
    c.curry = [200.0 + r[0] * 3.101_304e-8, 33.3 + r[1] * 9.313_226e-8, 66.6 + r[2] * 3.101_304e-8];
    count_tomoki_sakuraba(game);
    fall_voice(game, k, "voiceogg/sakuraba02.ogg", "voiceogg/sakuraba01.ogg");
}

/// motionCombo : Miki et Kiri enchaînent sur Taichi.
fn motion_combo(game: &mut Game, k: usize) {
    let who = play(game).casts[k].who;
    if who != Who::Miki && who != Who::Kiri {
        return;
    }
    let kiri = who == Who::Kiri;
    hide_taichi(game);
    position_taichi_bone(game, k);
    effects::combo(game);
    let (px, py) = (play(game).c.px, play(game).c.py);
    let c = &mut play(game).casts[k];
    let (reach, height, second) = if kiri { (67.0, 2.05, SubMove::new(2.529_411_8, -0.735_294_1, 3.0, 20.0, 20.0)) } else { (83.0, 2.15, SubMove::new(0.991_891_2, -0.675_675_7, 1.5, 20.0, 20.0)) };
    let vx = ((f64::from((c.stage_pos - px) * 100.0) + reach) * 0.020_833_333_333_333_332) as f32;
    let vy = ((height - f64::from(py)) * 2.083_333_333_333_333) as f32;
    c.moves.clear();
    c.moves.insert(0, SubMove::new(vx, vy, 7.5, 0.0, 0.0));
    c.moves.insert(0x30, second);
    play_once(game, Owner::Cast(k), "combo", true, true, 0);
    let node = play(game).casts[k].node;
    let (voice, se, partner) = if kiri { ("voiceogg/kiri01.ogg", "seogg/se13.ogg", "voiceogg/miki02.ogg") } else { ("voiceogg/miki04.ogg", "seogg/se13.ogg", "voiceogg/kiri04.ogg") };
    game.play_effect(voice);
    game.run(node, Action::sequence(vec![Action::delay(0.2), sound(se), Action::delay(0.4), sound(partner)]));
    let c = &mut play(game).c;
    c.misato = true;
    c.touko = false;
}

/// Les compteurs et succès d'un spécial (hors démonstration) : nombre de
/// spéciaux de ce personnage, et les quatre spéciaux vus.
fn special_achievement(game: &mut Game, key: &str, index: i32, goal: f32, bit: u32) {
    if play(game).c.demo {
        return;
    }
    let count = game.save.int(key, 0) + 1;
    game.save.set_int(key, count);
    game.update_achievement(index, 1, count as f32, goal);
    let c = &mut play(game).c;
    c.special_bits |= bit;
    let all = c.special_bits == 0xf;
    unlock_if(game, all, 0x11);
}

/// motionSpecial : le spécial de chaque personnage.
fn motion_special(game: &mut Game, k: usize) {
    let owner = Owner::Cast(k);
    match play(game).casts[k].who {
        Who::Miki => {
            hide_taichi(game);
            position_taichi_bone(game, k);
            let c = &mut play(game).c;
            c.saved = (c.vx, c.vy);
            c.vy = 0.0;
            c.no_gravity = true;
            c.combo = true;
            let f = 150.0 + (play(game).casts[k].stage_pos - play(game).c.px) * 100.0;
            play(game).c.vx = f * 0.016_666_666;
            let v = f * 0.016_666_668;
            control::slow_with_interval(game, 50.0, 0.01);
            let c = &mut play(game).casts[k];
            c.moves.clear();
            c.moves.insert(0, SubMove::new(v, 14.285_714, 7.5, 0.0, 0.0));
            c.moves.insert(0x1c, SubMove::new(v, -20.0, 7.5, 0.0, 0.0));
            c.moves.insert(0x30, SubMove::new(v, 0.0, 7.5, 0.0, 0.0));
            play_once(game, owner, "special", true, true, 0);
            special_achievement(game, "stat_miki_crash", 0xd, 5.0, 8);
            let c = &mut play(game).c;
            c.misato = true;
            c.touko = false;
            control::set_speed_blind(game, true);
            game.play_effect("voiceogg/miki01.ogg");
            game.play_effect("seogg/se05.ogg");
        }
        Who::Kiri => {
            hide_taichi(game);
            position_taichi_bone(game, k);
            let t = play(game).taichi.node;
            let width = game.size(t).0;
            let c = &mut play(game).c;
            c.saved = (c.vx, c.vy);
            let f = (play(game).casts[k].stage_pos - play(game).c.px) * 100.0;
            let c = &mut play(game).c;
            c.vy = 0.0;
            c.no_gravity = true;
            c.combo = true;
            c.vx = (f + 430.0) * 0.014_285_714;
            let v = (width + 430.0 + f) * 0.014_285_714;
            control::slow_with_interval(game, 50.0, 0.01);
            let c = &mut play(game).casts[k];
            c.moves.clear();
            c.moves.insert(0, SubMove::new(v, 0.0, 2.5, 0.0, 0.0));
            play_once(game, owner, "special", true, true, 0);
            special_achievement(game, "stat_kiri_crash", 0xe, 10.0, 4);
            game.play_effect("voiceogg/kiri03.ogg");
            game.play_effect("seogg/se05.ogg");
            let c = &mut play(game).c;
            c.misato = true;
            c.touko = false;
            control::set_speed_blind(game, true);
        }
        Who::Misato => {
            hide_taichi(game);
            control::slow_with_interval(game, 1000.0, 0.0);
            let c = &mut play(game).c;
            c.saved = (c.vx, c.vy);
            c.vx = 0.0;
            c.vy = 0.0;
            c.no_gravity = true;
            c.py = 0.5;
            let c = &play(game).casts[k];
            let (armature, bone) = (c.armature, c.bones.first().copied());
            let sprite = play(game).taichi.sprite;
            let color = game.g.get(sprite).map_or([0xFF; 3], |n| n.color);
            if let Some(b) = bone
                && let Some(a) = game.armature_mut(armature)
            {
                a.bones[b].color = color;
            }
            control::set_speed_blind(game, true);
            play_once(game, owner, "special", true, true, 0);
            special_achievement(game, "stat_misato_crash", 0xf, 10.0, 1);
            game.play_effect("voiceogg/touko01.ogg");
            let node = play(game).casts[k].node;
            let se = "seogg/se01.ogg";
            game.run(
                node,
                Action::sequence(vec![
                    Action::delay(0.25),
                    sound(se),
                    Action::delay(0.7),
                    sound("voiceogg/kiri02.ogg"),
                    Action::delay(0.25),
                    sound(se),
                    Action::delay(0.65),
                    sound("voiceogg/miki03.ogg"),
                    Action::delay(0.2),
                    sound(se),
                    Action::delay(0.4),
                    sound("voiceogg/touko06.ogg"),
                ]),
            );
        }
        Who::Touko => {
            hide_taichi(game);
            play(game).c.py = 1.0;
            position_taichi_bone(game, k);
            let c = &mut play(game).c;
            c.no_gravity = true;
            c.saved = (c.vx, c.vy);
            c.combo = false;
            let v = (183.3 + (play(game).casts[k].stage_pos - play(game).c.px) * 100.0) * 0.008_064_516;
            control::slow_with_interval(game, 1000.0, 0.0);
            let c = &mut play(game).casts[k];
            c.moves.clear();
            c.moves.insert(0, SubMove::new(v, -0.537_582_6, 2.5, 0.0, 0.0));
            c.moves.insert(0x7c, SubMove::default());
            play_once(game, owner, "special", true, true, 0);
            control::set_speed_blind(game, true);
            special_achievement(game, "stat_touko_crash", 0xc, 5.0, 2);
            game.play_effect("voiceogg/touko07se.ogg");
            let c = &mut play(game).c;
            c.touko = true;
            c.misato = true;
        }
        _ => {}
    }
}

/// onEnterSubFrame : une image de l'animation en cours.
fn enter_sub_frame(game: &mut Game, k: usize) {
    let c = &play(game).casts[k];
    let (who, state, frame, pos) = (c.who, c.state, c.sub_frame, c.stage_pos);
    match who {
        Who::Miki | Who::Kiri => {
            sub_frame_taichi(game, k);
            if state == 2 && frame == 0x30 {
                game.play_effect("seogg/se17.ogg");
                control::slow_with_interval(game, 60.0, 0.002);
                let (ahead, height) = if who == Who::Kiri { (1.67, 2.05) } else { (0.97, 2.15) };
                let c = &mut play(game).c;
                c.vy = 0.0;
                c.py = height;
                c.no_gravity = true;
                c.vx = ((pos + ahead) - c.px) * 14.705_881;
            }
        }
        Who::Touko => {
            sub_frame_taichi(game, k);
            if state != 1 {
                return;
            }
            if frame == 0x60 {
                game.play_effect("voiceogg/touko03.ogg");
                let node = play(game).casts[k].node;
                game.run(node, Action::sequence(vec![Action::delay(0.15), sound("seogg/se12.ogg")]));
            } else if frame == 0x7e {
                control::slow_with_interval(game, 50.0, 0.01);
                let c = &mut play(game).c;
                c.vy = 0.0;
                c.no_gravity = true;
                c.vx = ((pos + 1.84) - c.px) * 1.111_111_2;
                let node = play(game).casts[k].node;
                effects::white_effect(game, node);
                game.play_effect("voiceogg/touko05.ogg");
                game.play_effect("seogg/se_touko_sp.ogg");
            }
        }
        Who::Misato => {
            if state == 1 && frame == 0x6e {
                let c = &mut play(game).c;
                c.vy = 0.0;
                c.no_gravity = true;
                c.vx = ((pos - c.px) * 1.282_051_3).max(0.0);
                control::slow_with_interval(game, 100.0, 0.0);
            }
        }
        Who::Sakuraba => {
            if state == 0 && frame < 0x3c {
                let c = &play(game).casts[k];
                let (armature, bones, curry) = (c.armature, c.bones.clone(), c.curry);
                if bones.len() < 3 {
                    return;
                }
                let d = (frame - 0x14) as f32;
                let y = 133.3 - d * d * 0.083_312_51;
                let t = frame as f32 * 0.016_666_668;
                let Some(a) = game.armature_mut(armature) else { return };
                for (i, (speed, spin)) in [(curry[0], 0.043_633_23), (curry[2], 0.087_266_46), (curry[1], 0.130_899_69)].into_iter().enumerate() {
                    let bone = &mut a.bones[bones[i]];
                    bone.pos = (t * speed, y);
                    bone.rotation = (f64::from(bone.rotation) + spin) as f32;
                }
            } else {
                let c = &mut play(game).casts[k];
                c.sub_running = false;
                c.sub_frame = 0;
            }
        }
        _ => play(game).casts[k].sub_running = false,
    }
}

// --- Les événements d'animation ------------------------------------------------------------

fn on_armature(game: &mut Game, owner: Owner, e: &ArmatureEvent) {
    let c = get(game, owner);
    let (who, movement_cb, frame_cb) = (c.who, c.movement_cb, c.frame_cb);
    match e {
        ArmatureEvent::Movement(1, id) if movement_cb => {
            if who == Who::Misato
                && id == "normal"
                && let Owner::Cast(k) = owner
            {
                misato_normal_end(game, k);
            }
        }
        ArmatureEvent::Frame(name) if frame_cb => match owner {
            Owner::Cast(k) if name == "special_unpause" => special_unpause(game, k),
            Owner::Block => block_frame(game, name),
            _ => {}
        },
        _ => {}
    }
}

/// Misato::cbArmatureEvent : Taichi rattrapé s'arrête, sauf avec le champ
/// de force B (il en repart).
fn misato_normal_end(game: &mut Game, k: usize) {
    control::slow_with_interval(game, 0.0, 0.0);
    let c = &mut play(game).c;
    c.vx = 0.0;
    c.vy = 0.0;
    c.py = 0.0;
    if c.cff == 1 {
        let sprite = play(game).taichi.sprite;
        game.set_opacity(sprite, 0xFF);
        play_once(game, Owner::Cast(k), "cffb", false, false, 0);
        game.play_effect("voiceogg/misato02.ogg");
    } else {
        game.play_effect("voiceogg/misato01.ogg");
        let c = &play(game).c;
        if c.px <= 12.0 && !c.demo {
            game.unlock(0x14);
        }
    }
}

/// cbArmatureFrame « special_unpause » : Taichi repart.
fn special_unpause(game: &mut Game, k: usize) {
    let (who, state) = (play(game).casts[k].who, play(game).casts[k].state);
    if !matches!(who, Who::Miki | Who::Kiri | Who::Misato | Who::Touko) {
        return;
    }
    show_taichi(game);
    let r = taichi_bone_degrees(game, k);
    match (who, state) {
        (Who::Miki | Who::Kiri, 1) => {
            game.play_effect("seogg/se01.ogg");
            let limit = if who == Who::Kiri { 1.7 } else { 1.8 };
            let c = &mut play(game).c;
            c.no_gravity = false;
            c.vy = 1.0;
            c.vx = 30.0;
            c.r = r;
            c.py = c.py.min(limit);
            let distance = if who == Who::Kiri { 20.0 } else { 10.0 };
            control::slow_with_distance(game, distance, 1.0);
            play(game).casts[k].sub_running = false;
        }
        (Who::Miki | Who::Kiri, 2) => {
            let c = &mut play(game).c;
            (c.vx, c.vy) = c.saved;
            let boost = if who == Who::Kiri { (25.0, 60.0) } else { (24.0, 45.0) };
            play(game).casts[k].boost = boost;
            control::boost(game, boost.0, boost.1);
            let c = &mut play(game).c;
            c.no_gravity = false;
            c.r = r;
            control::slow_with_distance(game, 10.0, 1.0);
            control::set_speed_blind(game, false);
        }
        (Who::Misato, 1) => {
            game.play_effect("voiceogg/misato03.ogg");
            game.play_effect("seogg/se131317.ogg");
            let c = &mut play(game).c;
            c.no_gravity = false;
            (c.vx, c.vy) = c.saved;
            c.r = r;
            control::slow_with_distance(game, 40.0, 0.5);
            play(game).casts[k].boost = (35.0, 45.0);
            control::boost(game, 35.0, 45.0);
            play(game).casts[k].sub_running = false;
            control::set_speed_blind(game, false);
            effects::misato(game);
        }
        (Who::Touko, 1) => {
            let c = &mut play(game).c;
            (c.vx, c.vy) = c.saved;
            play(game).casts[k].boost = (25.0, 30.0);
            control::boost(game, 25.0, 30.0);
            play(game).c.no_gravity = false;
            control::slow_with_distance(game, 40.0, 1.0);
            play(game).casts[k].sub_running = false;
            control::set_speed_blind(game, false);
            game.play_effect("seogg/se16.ogg");
            let node = play(game).casts[k].node;
            game.run(node, Action::sequence(vec![Action::delay(0.2), sound("voiceogg/touko04.ogg")]));
        }
        _ => {}
    }
}

// --- Le blocage de Youko (YoukoBlock) --------------------------------------------------------

/// YoukoBlock::moveCast.
pub fn move_youko_block(game: &mut Game, px: f32, _py: f32, _r: f32, _dt: f32) {
    let b = &play(game).block;
    let (node, x) = (b.node, ((b.offset + b.stage_pos) - px) * 100.0);
    game.set_x(node, x);
    game.set_visible(node, -1350.0 < x);
}

/// Youko vient devant le personnage `k`.
fn block_appear(game: &mut Game, k: usize) {
    let c = &play(game).casts[k];
    let (pos, node) = (c.stage_pos, c.node);
    let x = game.pos(node).0;
    let b = &mut play(game).block;
    b.stage_pos = pos;
    b.active = true;
    let node = b.node;
    game.set_x(node, x);
    game.set_visible(node, true);
}

/// YoukoBlock::motionDefence : Youko protège le personnage (Tomoki et
/// Sakuraba n'en profitent pas).
fn block_motion_defence(game: &mut Game, k: usize) {
    block_appear(game, k);
    let b = &mut play(game).block;
    b.target = Some(k);
    b.state = 3;
    game.play_effect("seogg/se15.ogg");
    let tag = play(game).casts[k].tag;
    if tag == 0x70 || tag == 0x6f {
        play_once(game, Owner::Block, "cancel", true, false, 0);
    } else {
        let c = &mut play(game).c;
        if !c.demo {
            c.block_count += 1;
            let earned = c.block_count > 4;
            unlock_if(game, earned, 0x10);
        }
        play_once(game, Owner::Block, "block", true, true, 0);
    }
    control::slow_with_distance(game, 30.0, 1.0);
    play(game).c.youko_block = false;
}

/// YoukoBlock::motionSpecial : Nanaka et Youko lancent un champ de force
/// (le type dépend du personnage protégé).
fn block_motion_special(game: &mut Game, k: usize) {
    block_appear(game, k);
    let kind = match play(game).casts[k].tag {
        0x6a => 1,
        0x6b => 2,
        0x6c => 3,
        0x6e => 0,
        _ => 99,
    };
    play(game).c.cff_next = kind;
    play_once(game, Owner::Block, "special", true, true, 5);
    if !play(game).c.demo {
        game.unlock(0x12);
        let c = &mut play(game).c;
        c.block_bits |= 1u32.checked_shl(kind as u32).unwrap_or(0);
        let all = c.block_bits == 0xf;
        unlock_if(game, all, 0x13);
    }
    game.play_effect("seogg/se09.ogg");
    game.play_effect("voiceogg/youko01.ogg");
    game.play_effect("voiceogg/nanaca04.ogg");
    control::slow_with_interval(game, 1000.0, 0.0);
    play(game).block.state = 1;
    let c = &mut play(game).c;
    c.misato = true;
    c.youko_block = false;
}

/// YoukoBlock::unpauseSpecial : le PRESS raté, l'animation reprend.
fn unpause_special(game: &mut Game) {
    if play(game).block.state != 3 {
        return;
    }
    game.play_effect("voiceogg/youko01.ogg");
    if play(game).c.nanaca {
        control::slow_with_interval(game, 20.0, 0.0);
        let armature = play(game).block.armature;
        if let Some(a) = game.armature_mut(armature) {
            a.resume();
        }
    }
}

/// YoukoBlock::cbArmatureFrame.
fn block_frame(game: &mut Game, name: &str) {
    let state = play(game).block.state;
    if name == "special_unpause" {
        if state != 1 {
            return;
        }
        play(game).block.boost = (15.0, 45.0);
        control::boost(game, 15.0, 45.0);
        control::slow_with_distance(game, 70.0, 0.8);
        let kind = play(game).c.cff_next;
        control::set_comic_force_field(game, kind);
        effects::cff(game, kind);
        game.play_effect("seogg/se01.ogg");
        game.play_effect("voiceogg/nanaca01.ogg");
    } else if name == "special_check_cff" && state == 3 {
        let pos = play(game).block.stage_pos;
        let c = &play(game).c;
        let refused = (!c.nanaca || pos % 100.0 != 0.0) && (!c.demo || !c.nanaca) && c.nanaca_first;
        if refused {
            game.play_effect("voiceogg/youko01.ogg");
            return;
        }
        play(game).c.nanaca_first = true;
        control::slow_with_interval(game, 1000.0, 0.0);
        let armature = play(game).block.armature;
        if let Some(a) = game.armature_mut(armature) {
            a.pause();
        }
        let id = game.play_effect("seogg/se18.ogg");
        play(game).special_sound = id;
        let Some(k) = play(game).block.target else { return };
        play(game).casts[k].state = 4;
        play(game).casts[k].special_pending = true;
        let layer = play(game).hud.special;
        game.set_visible(layer, true);
        let stop = play(game).c.stop;
        if !stop {
            hud::special_start(game);
        }
        let c = &mut play(game).c;
        c.special_timer = 1.0;
        c.special_state = if stop { 1 } else { -1 };
    }
}

// --- Nanaka ---------------------------------------------------------------------------------

/// Nanaca::moveCast : à sa place en scène, visible à l'écran.
pub fn move_nanaca(game: &mut Game, px: f32) {
    let n = &play(game).nanaca;
    let node = n.node;
    let x = (n.stage_pos - px) * 100.0;
    game.set_x(node, x);
    game.set_visible(node, -1350.0 < x);
}

/// Nanaca::motionCrash : l'élan d'ouverture vers Taichi.
pub fn nanaca_start_crash(game: &mut Game) {
    stage(&mut play(game).nanaca, 0.0);
    let (node, armature) = (play(game).nanaca.node, play(game).nanaca.armature);
    game.set_pos(armature, 0.0, 30.0);
    game.set_anchor(armature, 0.5, 0.5);
    if let Some(a) = game.armature_mut(armature) {
        if let Some(b) = a.bone("nanaca") {
            a.bones[b].rotation = 0.0;
        }
        a.play("start_crash", -1, -1, -1, EASING_MAX);
    }
    game.listen_armature(armature, Rc::new(|game: &mut Game, armature, e| {
        if let ArmatureEvent::Frame(name) = e
            && name == "eventCrash"
        {
            // L'os « nanaca » tourne de l'angle de la jauge (en radians :
            // la rotation d'un os s'ajoute à son inclinaison).
            let angle = play(game).nanaca.meter_angle;
            if let Some(a) = game.armature_mut(armature)
                && let Some(b) = a.bone("nanaca")
            {
                a.bones[b].rotation = -((f64::from(angle) * 0.017_453_292_5) as f32);
            }
            effects::initial_crash(game);
            game.play_effect("seogg/se17.ogg");
        }
    }));
    game.set_visible(node, true);
    let jm = play(game).jump_meter;
    play(game).nanaca.meter_angle = play(game).meter_angle;
    game.run(node, Action::sequence(vec![Action::delay(0.5), Action::move_by(0.5, (-100.0, 0.0))]));
    game.set_anchor(jm, 0.0, 0.0);
    game.run(jm, Action::sequence(vec![Action::delay(1.0), Action::scale_to(0.5, 0.0), Action::hide()]));
    game.play_effect("voiceogg/nanaca03.ogg");
    play(game).nanaca.stage_pos -= 1.0;
}

/// Nanaca::aerialCrash : l'attaque aérienne (haut si Taichi retombe).
pub fn nanaca_aerial_crash(game: &mut Game, upper: bool) {
    if !play(game).flying() {
        return;
    }
    play(game).c.hit_enabled = false;
    control::slow_with_interval(game, 1000.0, 0.0);
    let (px, py) = (play(game).c.px, play(game).c.py);
    play(game).nanaca.stage_pos = px;
    let (node, armature) = (play(game).nanaca.node, play(game).nanaca.armature);
    game.set_pos(armature, 0.0, 0.0);
    if let Some(a) = game.armature_mut(armature)
        && let Some(b) = a.bone("nanaca")
    {
        a.bones[b].rotation = 0.0;
    }
    game.set_pos(armature, -50.0, py * 100.0 - 75.0);
    game.set_anchor(armature, 0.5, 0.5);
    if let Some(a) = game.armature_mut(armature) {
        if upper {
            a.play("aerial_crash_u", -1, -1, -1, EASING_MAX);
        } else {
            a.play("aerial_crash_d", -1, -1, -1, EASING_MAX);
        }
    }
    let rest = play(game).c.ups;
    effects::aerial_hud_motion(game, upper, rest);
    let voice = if upper { "voiceogg/nanaca02.ogg" } else { "voiceogg/nanaca05.ogg" };
    game.run(node, Action::sequence(vec![Action::delay(0.3), sound(voice), sound("seogg/se09.ogg"), Action::delay(0.33), sound("seogg/se01.ogg"), Action::delay(0.66), sound("seogg/se09.ogg")]));
}
