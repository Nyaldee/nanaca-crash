//! L'affichage de la partie : compteurs de score (ScoreLabel), jauge de
//! lancement (JumpMeterLayer), compteur de vitesse (SpeedMeterLayer),
//! marqueur de hauteur (OutMarkerLayer), SPECIAL (SpecialLayer),
//! indicateur des spéciaux (SpcialIndicatorLayer), visages des
//! personnages qui approchent (ContactLayer), HUD aérien (SceneGame.lua) et
//! menu pause (PauseLayer).

use std::rc::Rc;

use crate::action::Action;
use crate::game::Game;
use crate::node::{Kind, Node, NodeId};

use super::{State, play};

/// Les nœuds du HUD.
#[derive(Clone, Copy)]
pub struct Hud {
    pub record: NodeId,
    pub best: NodeId,
    pub speed: NodeId,
    pub speed_label: NodeId,
    pub speed_arrow: NodeId,
    /// Mode du compteur : 0 vitesse x, 1 vitesse y, 2 vitesse, 3 masqué.
    pub speed_mode: i32,
    pub out_marker: NodeId,
    pub out_icon: NodeId,
    pub out_label: NodeId,
    pub special: NodeId,
    pub indicator: NodeId,
    pub contact: NodeId,
    pub contact_draw: NodeId,
    /// Image paire ou impaire (les flèches sont redessinées une sur deux).
    pub contact_frame: i32,
    pub pause_button: NodeId,
    pub pause_layer: NodeId,
    pub pause_menu: NodeId,
    pub aerial_indicator: NodeId,
    pub daggers: NodeId,
    pub aerial_label: NodeId,
}

/// Les succès de vitesse et de hauteur déjà obtenus pendant la partie.
#[derive(Default, Clone, Copy)]
pub struct HudFlags {
    pub speed_27: bool,
    pub speed_83: bool,
    pub height_100: bool,
    pub height_332: bool,
}

// --- ScoreLabel ----------------------------------------------------------------

/// ScoreLabel::createWithString : un intitulé BMFont suivi d'un nombre en
/// cases.
pub fn score_label(game: &mut Game, caption: &str, text: &str) -> NodeId {
    let node = game.g.color_layer([0xFF, 0, 0, 0], (0.0, 0.0));
    let atlas = game.atlas_label(text, "arialblack32-atlas.png", 16.0, 18.0, ' ');
    game.set_x(atlas, 0.0);
    game.set_anchor(atlas, 0.5, 0.5);
    game.add(node, atlas, 0, 1);
    let label = game.bm_label(caption, "arialblack32.fnt", 0);
    let (lw, _) = game.size(label);
    let (aw, ah) = game.size(atlas);
    game.add(node, label, 0, 2);
    game.set_anchor(label, 0.5, 0.5);
    game.set_x(label, lw * 0.5);
    game.set_x(atlas, lw + aw * 0.5);
    game.set_y(label, ah * 0.5);
    game.set_y(atlas, ah * 0.5);
    if let Some(n) = game.g.get_mut(node) {
        n.size = (aw + lw, ah);
    }
    node
}

/// La taille d'un ScoreLabel (intitulé + nombre).
pub fn score_size(game: &Game, node: NodeId) -> (f32, f32) {
    let (Some(atlas), Some(label)) = (game.child(node, 1), game.child(node, 2)) else { return (0.0, 0.0) };
    let (a, l) = (game.size(atlas), game.size(label));
    (a.0 + l.0, a.1)
}

pub fn set_score(game: &mut Game, node: NodeId, text: &str) {
    if let Some(atlas) = game.child(node, 1) {
        game.g.set_text(atlas, text);
    }
}

pub fn set_score_color(game: &mut Game, node: NodeId, color: [u8; 3]) {
    for tag in [1, 2] {
        if let Some(c) = game.child(node, tag) {
            game.set_color(c, color);
        }
    }
}

// --- La jauge de lancement -------------------------------------------------------

/// JumpMeterLayer::init : cadran, aiguille, barre de puissance, valeurs,
/// logo du skin, PRESS / RELEASE.
pub fn jump_meter(game: &mut Game) -> NodeId {
    let layer = game.g.color_layer([0, 0, 0, 0], (0.0, 0.0));
    let dial = game.sprite("meter0.png");
    let size = game.size(dial);
    game.add(layer, dial, 0, 3);
    let needle = game.sprite("meter1.png");
    game.add(layer, needle, 2, 2);
    let bar = game.sprite("meter2.png");
    game.add(layer, bar, 1, 1);
    let frame = game.sprite("meter3.png");
    game.add(layer, frame, 3, 9);
    game.set_anchor(frame, 0.0, 0.0);
    game.set_pos(frame, -3.0, -3.0);
    let font_color = play(game).font_color;
    let power = game.bm_label("00000%", "arialblack32.fnt", 1);
    game.set_color(power, [0, 0, 0]);
    game.set_anchor(power, 0.5, 0.5);
    game.set_pos(power, 35.0, -10.0);
    game.set_scale(power, 0.0);
    game.add(layer, power, 4, 4);
    let angle = game.bm_label("00000\"", "arialblack32.fnt", 1);
    game.set_color(angle, [0, 0, 0]);
    game.set_anchor(angle, 0.5, 0.5);
    game.set_pos(angle, 80.0, 65.0);
    game.set_scale(angle, 0.0);
    game.add(layer, angle, 5, 5);
    let logo_name = game.skin_image("contact.png", "contact_summer.png", "contact_fireworkshow.png");
    let logo = game.sprite(logo_name);
    game.set_color(logo, font_color);
    game.add(layer, logo, 6, 6);
    game.set_pos(logo, -70.0, 40.0);
    let press = game.sprite("t_press.png");
    game.add(layer, press, 7, 7);
    game.set_pos(press, 38.0, -10.0);
    let release = game.sprite("t_release.png");
    game.add(layer, release, 8, 8);
    game.set_pos(release, 38.0, -10.0);
    game.set_visible(release, false);
    game.set_anchor(bar, 0.0, 0.0);
    game.set_pos(bar, 3.0, 3.0);
    game.set_scale(bar, 0.0);
    game.set_anchor(dial, 0.0, 0.0);
    game.set_anchor(needle, 0.0, 0.5);
    game.set_pos(needle, 1.0, 1.0);
    game.set_rotation(needle, 0.0);
    if let Some(n) = game.g.get_mut(layer) {
        n.size = size;
    }
    layer
}

/// resetAction (tutoriel) : la jauge comme neuve, visible, aiguille à 0.
pub fn reset_meter(game: &mut Game) {
    let jm = play(game).jump_meter;
    let get = |game: &Game, tag| game.child(jm, tag);
    let (bar, needle, dial, logo) = (get(game, 1), get(game, 2), get(game, 3), get(game, 6));
    for (tag, on) in [(1, true), (2, true), (7, true), (8, false)] {
        if let Some(n) = get(game, tag) {
            game.set_visible(n, on);
        }
    }
    for tag in [4, 5] {
        if let Some(n) = get(game, tag) {
            game.set_scale(n, 0.0);
        }
    }
    if let Some(bar) = bar {
        game.set_anchor(bar, 0.0, 0.0);
        game.set_pos(bar, 3.0, 3.0);
        game.set_scale(bar, 0.0);
    }
    if let Some(dial) = dial {
        game.set_anchor(dial, 0.0, 0.0);
    }
    if let Some(needle) = needle {
        game.set_anchor(needle, 0.0, 0.5);
        game.set_pos(needle, 1.0, 1.0);
        game.set_rotation(needle, 0.0);
    }
    if let Some(logo) = logo {
        game.set_visible(logo, true);
        game.set_pos(logo, -70.0, 40.0);
    }
    game.set_scale(jm, 1.0);
    game.set_visible(jm, true);
}

/// startNeedleAction : l'aiguille balaie 0..90°.
pub fn start_needle(game: &mut Game) {
    let jm = play(game).jump_meter;
    let Some(needle) = game.child(jm, 2) else { return };
    game.set_rotation(needle, 0.0);
    game.run(needle, Action::forever(Action::sequence(vec![Action::rotate_to(0.5, -90.0), Action::rotate_to(0.5, 0.0)])));
    play(game).powering = false;
}

/// startPowerAction : l'aiguille s'arrête, la barre de puissance pulse.
pub fn start_power(game: &mut Game) {
    let jm = play(game).jump_meter;
    let (Some(needle), Some(bar)) = (game.child(jm, 2), game.child(jm, 1)) else { return };
    game.stop_actions(needle);
    game.set_scale(bar, 0.0);
    game.run(bar, Action::forever(Action::sequence(vec![Action::scale_to(0.4, 1.0), Action::delay(0.1), Action::scale_to(0.4, 0.0)])));
    play(game).powering = true;
    if let (Some(press), Some(release)) = (game.child(jm, 7), game.child(jm, 8)) {
        game.set_visible(press, false);
        game.set_visible(release, true);
    }
}

/// finishAction : angle et puissance lus et affichés.
pub fn finish_power(game: &mut Game) {
    let jm = play(game).jump_meter;
    let (Some(bar), Some(needle)) = (game.child(jm, 1), game.child(jm, 2)) else { return };
    game.stop_actions(bar);
    let angle = -game.rotation(needle);
    let power = game.scale(bar) * 100.0;
    let p = play(game);
    p.meter_angle = angle;
    p.meter_power = power;
    if let Some(l) = game.child(jm, 4) {
        game.g.set_text(l, &format!("{power:0.0}%"));
        game.run(l, Action::scale_to(0.2, 1.0));
    }
    if let Some(l) = game.child(jm, 5) {
        game.g.set_text(l, &format!("{angle:0.0}\""));
        game.run(l, Action::scale_to(0.2, 1.0));
    }
    if let Some(logo) = game.child(jm, 6) {
        game.run(logo, Action::sequence(vec![Action::move_by(0.5, (-200.0, 0.0)), Action::hide()]));
    }
    for tag in [7, 8] {
        if let Some(c) = game.child(jm, tag) {
            game.set_visible(c, false);
        }
    }
}

// --- Compteur de vitesse -----------------------------------------------------------

pub fn speed_meter(game: &mut Game, vx: f32, vy: f32, demo: bool) {
    let hud = play(game).hud;
    let text = match hud.speed_mode {
        2 => {
            let a = (-vy).atan2(vx);
            game.set_rotation(hud.speed_arrow, (f64::from(a * 180.0) * 0.318_309_883_507_877_26) as f32);
            format!("{:6.2}:/;", (vx * vx + vy * vy).sqrt())
        }
        3 => "----.--:/;".to_string(),
        0 => format!("{vx:6.2}:/;"),
        _ => {
            game.set_rotation(hud.speed_arrow, if vy > 0.0 { -90.0 } else { 90.0 });
            format!("{:6.2}:/;", vy.abs())
        }
    };
    game.g.set_text(hud.speed_label, &text);
    if !demo {
        let flags = &mut play(game).flags;
        if !flags.speed_27 && vx >= 27.8 {
            flags.speed_27 = true;
            game.unlock(0x16);
        }
        let flags = &mut play(game).flags;
        if !flags.speed_83 && vx >= 83.3 {
            flags.speed_83 = true;
            game.unlock(0x17);
        }
    }
}

// --- Marqueur de hauteur -------------------------------------------------------------

pub fn out_marker(game: &mut Game, py: f32, _r: f32, demo: bool) {
    let hud = play(game).hud;
    if py <= 12.9 {
        game.set_visible(hud.out_marker, false);
        return;
    }
    let taichi = play(game).taichi;
    let world = game.g.world(taichi.node);
    let (x, _) = world.apply(0.0, 0.0);
    let color = game.g.get(taichi.sprite).map_or([255; 3], |n| n.color);
    game.set_color(hud.out_icon, color);
    game.set_x(hud.out_marker, x);
    game.set_visible(hud.out_marker, true);
    game.g.set_text(hud.out_label, &format!("{py:5.2}:"));
    if py >= 100.0 && !demo {
        let flags = &mut play(game).flags;
        if !flags.height_100 {
            flags.height_100 = true;
            game.unlock(0x18);
        }
        let flags = &mut play(game).flags;
        if !flags.height_332 && py >= 332.6 {
            flags.height_332 = true;
            game.unlock(0x19);
        }
    }
}

// --- SPECIAL ------------------------------------------------------------------------

/// SpecialLayer::create et onEnter : SPECIAL (avec reflet), raté, PRESS.
pub fn special_layer(game: &mut Game, with_shine: bool) -> NodeId {
    let layer = game.g.layer((0.0, 0.0));
    let text = if with_shine {
        let stencil = game.sprite("t_special.png");
        let mut clip = Node::new(Kind::Clip { stencil: Some(stencil), inverted: false });
        clip.size = game.size(stencil);
        let clip = game.g.add(clip);
        let copy = game.sprite("t_special.png");
        game.set_pos(copy, 0.0, 0.0);
        let shine = game.sprite("vshine.png");
        let (sw, _) = game.size(shine);
        game.set_anchor(shine, 0.5, 0.0);
        game.set_pos(shine, -sw, 0.0);
        game.add(clip, shine, 2, 2);
        game.add(clip, copy, 1, 1);
        clip
    } else {
        let s = game.sprite("t_special.png");
        game.set_pos(s, 0.0, 0.0);
        s
    };
    game.add(layer, text, 1, 1);
    let ng = game.sprite("special_ng.png");
    game.add(layer, ng, 2, 2);
    let press = game.sprite("t_press_sp.png");
    game.set_pos(press, 0.0, -50.0);
    game.add(layer, press, 3, 3);
    play_special_flag(game, with_shine);
    layer
}

fn play_special_flag(game: &mut Game, with_shine: bool) {
    play(game).hud_special_clip = with_shine;
}

/// SpecialLayer::start : PRESS clignote.
pub fn special_start(game: &mut Game) {
    let hud = play(game).hud;
    let clip = play(game).hud_special_clip;
    let layer = hud.special;
    let (ng, press, text) = (game.child(layer, 2), game.child(layer, 3), game.child(layer, 1));
    if let Some(ng) = ng {
        game.set_visible(ng, false);
    }
    if let Some(p) = press {
        game.set_visible(p, true);
    }
    if let Some(t) = text {
        if !clip {
            game.set_opacity(t, 0xFF);
            game.set_scale(t, 1.0);
            game.set_visible(t, true);
        } else if let Some(shine) = game.child(t, 2) {
            game.set_visible(shine, false);
        }
    }
    if let Some(p) = press {
        game.run(p, Action::blink(1.0, 3));
    }
}

/// SpecialLayer::ok : réussi (reflet ou agrandissement), puis caché.
pub fn special_ok(game: &mut Game) {
    let layer = play(game).hud.special;
    let clip = play(game).hud_special_clip;
    for tag in [2, 3] {
        if let Some(c) = game.child(layer, tag) {
            game.set_visible(c, false);
        }
    }
    let Some(text) = game.child(layer, 1) else { return };
    if !clip {
        game.run(text, Action::spawn(vec![Action::scale_to(1.0, 1.2), Action::fade_out(1.0)]));
    } else if let (Some(shine), Some(copy)) = (game.child(text, 2), game.child(text, 1)) {
        let (cw, _) = game.size(copy);
        let (sw, sh) = game.size(shine);
        game.set_visible(shine, true);
        game.set_pos(shine, -(cw * 0.5), -(sh * 0.5));
        game.run(shine, Action::move_to(0.7, (cw * 0.5 + sw * 0.5, -(sh * 0.5))));
    }
    game.run(layer, Action::sequence(vec![Action::delay(1.0), Action::hide()]));
}

/// SpecialLayer::ng : raté.
pub fn special_ng(game: &mut Game) {
    let layer = play(game).hud.special;
    if let Some(p) = game.child(layer, 3) {
        game.set_visible(p, false);
    }
    if let Some(t) = game.child(layer, 1)
        && let Some(shine) = game.child(t, 2)
    {
        game.set_visible(shine, false);
    }
    if let Some(ng) = game.child(layer, 2) {
        game.set_visible(ng, true);
    }
    game.run(layer, Action::sequence(vec![Action::delay(0.8), Action::hide()]));
}

// --- Indicateur des spéciaux -----------------------------------------------------------

pub fn special_indicator_layer(game: &mut Game) -> NodeId {
    let layer = game.g.color_layer([0, 0, 0, 0], (0.0, 0.0));
    let batch = game.g.node();
    for (name, x, tag) in [("c_misatoface.png", 0.0, 0x6e), ("c_toukoface.png", 32.0, 0x6a), ("c_kiriface.png", 64.0, 0x6b), ("c_mikiface.png", 96.0, 0x6c)] {
        let face = game.sprite(name);
        game.set_color(face, [0x7F; 3]);
        game.set_pos(face, x, 0.0);
        game.add(batch, face, 1, tag);
    }
    for (name, y, z, tag, shown) in [("spindtitle.png", 16.0, 1, 10, true), ("spindblock.png", -16.0, 2, 0xb, false), ("spindversus.png", -16.0, 2, 0xc, false), ("spbase.png", -16.0, 0, 0xd, true)] {
        let s = game.sprite(name);
        game.set_pos(s, -16.0, y);
        game.set_anchor(s, 0.0, 0.0);
        game.set_visible(s, shown);
        game.add(batch, s, z, tag);
    }
    game.add(layer, batch, 0, 1);
    layer
}

/// SpcialIndicatorLayer::update.
pub fn special_indicator(game: &mut Game) {
    let indicator = play(game).hud.indicator;
    let Some(batch) = game.child(indicator, 1) else { return };
    let c = &play(game).c;
    let (block, misato, touko, kiri, miki, nanaca) = (c.youko_block, c.misato, c.touko, c.kiri, c.miki, c.nanaca);
    let face = |on: bool| if on { [0xFF; 3] } else { [0x7F; 3] };
    let faces = [(0x6e, misato), (0x6a, touko), (0x6b, kiri), (0x6c, miki)];
    let (spblock, versus) = (game.child(batch, 0xb), game.child(batch, 0xc));
    if !block {
        for (tag, on) in faces {
            if let Some(f) = game.child(batch, tag) {
                game.set_color(f, face(on));
            }
        }
        for s in [spblock, versus].into_iter().flatten() {
            game.set_visible(s, false);
        }
    } else {
        if let Some(b) = spblock
            && !game.visible(b)
        {
            game.set_visible(b, true);
        }
        if let Some(v) = versus {
            game.set_visible(v, nanaca);
            if !nanaca {
                for (tag, _) in faces {
                    if let Some(f) = game.child(batch, tag) {
                        game.set_color(f, [0x7F; 3]);
                    }
                }
            }
        }
    }
}

// --- Visages des personnages qui approchent ----------------------------------------------

/// Couleur de la flèche de chaque visage (RVBA 0..1).
const CONTACT_COLORS: [(i32, &str, [f32; 4]); 7] = [
    (0x6a, "c_toukoface.png", [0.627_451, 0.0, 0.0, 1.0]),
    (0x6b, "c_kiriface.png", [0.0, 0.0, 0.627_451, 1.0]),
    (0x6c, "c_mikiface.png", [0.627_451, 0.627_451, 0.0, 1.0]),
    (0x6d, "c_youkoface.png", [0.627_451, 0.0, 0.627_451, 1.0]),
    (0x6e, "c_misatoface.png", [0.0, 0.627_451, 0.0, 1.0]),
    (0x6f, "c_sakurabaface.png", [0.627_451, 0.313_725, 0.0, 1.0]),
    (0x70, "c_tomokiface.png", [0.0, 0.627_451, 0.627_451, 1.0]),
];

pub fn contact_layer(game: &mut Game) -> (NodeId, NodeId) {
    let layer = game.g.color_layer([0, 0, 0, 0], (96.0, 32.0));
    let batch = game.g.node();
    for (tag, name, _) in CONTACT_COLORS {
        let face = game.sprite(name);
        game.set_visible(face, false);
        game.add(batch, face, 1, tag);
    }
    game.add(layer, batch, 0, 1);
    let draw = game.g.draw_node();
    game.add(layer, draw, 2, 200);
    game.set_visible(layer, false);
    (layer, draw)
}

/// ContactLayer::ready : les flèches sont effacées une image sur deux.
pub fn contact_ready(game: &mut Game) {
    let hud = &mut play(game).hud;
    if hud.contact_frame > 0 {
        hud.contact_frame = 0;
        let draw = hud.contact_draw;
        if let Some(n) = game.g.get_mut(draw)
            && let Kind::Draw(t) = &mut n.kind
        {
            t.clear();
        }
    } else {
        hud.contact_frame += 1;
    }
}

/// ContactLayer::view : le visage d'un personnage à `distance` mètres, et
/// une flèche vers lui.
pub fn contact_view(game: &mut Game, cast_node: NodeId, tag: i32, distance: f32) {
    let hud = play(game).hud;
    if hud.contact_frame != 0 {
        return;
    }
    let Some(batch) = game.child(hud.contact, 1) else { return };
    let Some(face) = game.child(batch, tag) else { return };
    // La position du personnage (dans sa couche) prise comme un point de
    // l'écran, comme dans l'original.
    let (cx, cy) = game.pos(cast_node);
    let (lx, ly) = game.g.world(hud.contact).invert().apply(cx, cy + 150.0);
    if distance <= 0.0 {
        game.set_visible(face, false);
        return;
    }
    game.set_visible(face, true);
    let x = (f64::from(distance) * 0.1).floor() as f32 * 32.0;
    game.set_x(face, x);
    let tip = ((lx + x * 19.0) * 0.05, (ly + -608.0) * 0.05);
    let color = CONTACT_COLORS.iter().find(|c| c.0 == tag).map_or([1.0; 4], |c| c.2);
    if let Some(n) = game.g.get_mut(hud.contact_draw)
        && let Kind::Draw(t) = &mut n.kind
    {
        t.push(([(x - 10.0, -16.0), (x + 10.0, -16.0), tip], color));
    }
}

// --- HUD aérien (SceneGame.lua) -----------------------------------------------------------

pub fn aerial_hud(game: &mut Game, scene: NodeId) -> (NodeId, NodeId) {
    let h = game.design.1;
    let aerial = game.sprite("aerial.png");
    game.add(scene, aerial, 400, 400);
    game.set_anchor(aerial, 0.0, 0.0);
    game.set_pos(aerial, 10.0, h - 40.0);
    game.set_scale(aerial, 0.8);
    game.set_color(aerial, [0xCC; 3]);
    let daggers = game.g.node();
    for i in 0..4 {
        let d = game.sprite("dagger.png");
        if i == 3 {
            game.set_pos(d, 20.0, h - 70.0);
            game.set_color(d, [0x66, 0x66, 0xFF]);
        } else {
            game.set_pos(d, i as f32 * 20.0 + 20.0, h - 100.0);
            game.set_color(d, [0xFF, 0x66, 0x66]);
        }
        game.set_anchor(d, 0.5, 0.5);
        game.add(daggers, d, i, i);
    }
    game.add(scene, daggers, 402, 402);
    (aerial, daggers)
}

// --- Menu pause ---------------------------------------------------------------------------

/// MyMenuItemPause : un bouton 150 × 44 (menu_f.png étiré) et son texte.
fn pause_item(game: &mut Game, text: &str, tag: i32, action: fn(&mut Game)) -> NodeId {
    crate::game::widgets::pause_item(game, text, (150.0, 44.0), tag, Rc::new(move |game: &mut Game, _| action(game)))
}

/// PauseLayer : RESUME, AGAIN, QUIT (MORE GAME, toujours caché).
pub fn pause_layer(game: &mut Game) -> (NodeId, NodeId) {
    let (w, h) = game.design;
    let layer = game.g.color_layer([0xFF, 0xFF, 0xFF, 0], (w, h));
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    for (text, tag, action) in [("RESUME", 1, super::resume as fn(&mut Game)), ("AGAIN", 2, super::again), ("QUIT", 3, super::quit), ("MORE GAME", 4, |_: &mut Game| {})] {
        let item = pause_item(game, text, tag, action);
        game.add(menu, item, 0, tag);
    }
    game.add(layer, menu, 999, 999);
    game.set_visible(layer, false);
    (layer, menu)
}

/// MyMenuItemPause::setEnabled : grisé quand inactif.
fn set_item_enabled(game: &mut Game, item: NodeId, on: bool) {
    if let Some(n) = game.g.get_mut(item)
        && let Kind::Item(i) = &mut n.kind
    {
        if i.enabled == on {
            return;
        }
        i.enabled = on;
    }
    game.set_color(item, if on { [0xFF; 3] } else { [0x7F; 3] });
}

/// PauseLayer::show : les boutons selon l'état (pause, jauge, fin).
pub fn pause_show(game: &mut Game) {
    let (_, h) = game.design;
    let hud = play(game).hud;
    let y = h - 27.0;
    let items: Vec<NodeId> = (1..=4).filter_map(|t| game.child(hud.pause_menu, t)).collect();
    if items.len() < 4 {
        return;
    }
    for (k, &item) in items.iter().enumerate() {
        set_item_enabled(game, item, true);
        game.set_visible(item, true);
        game.set_pos(item, 75.0, y - 40.0 * k as f32);
    }
    set_item_enabled(game, items[3], false);
    game.set_visible(items[3], false);
    let (state, prev) = (play(game).state, play(game).prev_state);
    if prev == Some(State::Gauge) {
        set_item_enabled(game, items[1], false);
    } else if matches!(state, State::GameOver { .. }) {
        set_item_enabled(game, items[0], false);
        game.set_visible(items[0], false);
        let (x, py) = game.pos(hud.pause_layer);
        game.set_pos(hud.pause_layer, x, py + 200.0);
        game.run(hud.pause_layer, Action::move_to(0.5, (x, py + 40.0)));
    }
    game.set_visible(hud.pause_layer, true);
}
