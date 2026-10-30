//! Taichi en vol (Taichi.cpp et Taichi.lua) : une couche dont le sprite
//! tourne et se teinte, trois traînées de lumière, le compteur du
//! C.F.F. C, et les effets des champs de force.

use crate::action::Action;
use crate::game::Game;
use crate::node::NodeId;

use super::play;

#[derive(Clone, Copy)]
pub struct Taichi {
    /// La couche (étiquette 100 dans la couche de jeu).
    pub node: NodeId,
    /// Le sprite de Taichi (rotation et couleur de la couche).
    pub sprite: NodeId,
    /// Les traînées (enfant 2) et le compteur (enfant 10).
    pub lights: NodeId,
    pub counter: NodeId,
    /// Images du C.F.F. C et du C.F.F. D (-1 : arrêté).
    pub c_frame: i32,
    pub d_frame: i32,
    /// La boucle du C.F.F. C.
    pub c_sound: u32,
    /// Caché par un personnage (motion spéciale).
    pub hidden: bool,
}

/// Taichi::create et Taichi.SetupEffects.
pub fn create(game: &mut Game) -> Taichi {
    let node = game.g.color_layer([0, 0, 0, 0], (0.0, 0.0));
    let fly = game.skin_image("c_taichifly.png", "c_taichifly_summer.png", "c_taichifly_firework.png");
    let sprite = game.sprite(fly);
    let size = game.size(sprite);
    game.set_pos(sprite, 0.0, 0.0);
    if let Some(n) = game.g.get_mut(node) {
        n.size = size;
    }
    game.add(node, sprite, 0, -1);
    let lights = game.g.node();
    for i in 0..3 {
        let l = game.sprite("lightsmall.png");
        game.set_pos(l, 0.0, 0.0);
        game.add(lights, l, 1, i);
    }
    game.add(node, lights, 2, 2);
    game.set_visible(lights, false);
    setup_effects(game, node);
    let counter = game.bm_label("0000", "arialblack32.fnt", 2);
    game.set_x(counter, 80.0);
    game.add(node, counter, 10, 10);
    game.set_visible(counter, false);
    Taichi { node, sprite, lights, counter, c_frame: -1, d_frame: -1, c_sound: 0, hidden: false }
}

/// Taichi.SetupEffects (Lua).
fn setup_effects(game: &mut Game, t: NodeId) {
    let boost = game.sprite("t_boost.png");
    game.set_rotation(boost, -10.0);
    if let Some(n) = game.g.get_mut(boost) {
        n.skew.0 = 10.0;
    }
    game.set_visible(boost, false);
    game.set_anchor(boost, 0.0, 0.0);
    game.add(t, boost, 3, 5);
    let back = game.sprite("bakucff.png");
    game.set_color(back, [0, 255, 64]);
    game.set_scale(back, 0.4);
    game.add(t, back, -1, 10);
    let double = game.sprite("t_double.png");
    game.add(t, double, 2, 11);
    game.set_visible(back, false);
    game.set_visible(double, false);
    let shines = game.g.node();
    for i in 0..8 {
        let light = game.sprite("lightsmall.png");
        game.set_anchor(light, -0.5, 0.5);
        game.set_rotation(light, 45.0 * i as f32);
        game.add(shines, light, 0, -1);
    }
    let charge = game.sprite("t_charge.png");
    game.add(t, shines, 2, 20);
    game.add(t, charge, 2, 21);
    game.set_visible(shines, false);
    game.set_visible(charge, false);
    for ii in 0..3 {
        let ring = game.g.node();
        for i in 0..6 {
            let e = game.sprite("cffelement1.png");
            game.set_anchor(e, -1.0, 0.0);
            game.set_rotation(e, 60.0 * i as f32);
            game.add(ring, e, 0, -1);
        }
        game.add(t, ring, 2, 30 + ii);
        game.set_visible(ring, false);
    }
    for i in 0..2 {
        let baku = game.sprite("bakucff.png");
        game.set_scale(baku, 0.05);
        game.set_rotation(baku, 110.0);
        game.set_color(baku, [255, 127, 127]);
        game.set_pos(baku, if i == 0 { 5.0 } else { 25.0 }, if i == 0 { 20.0 } else { -5.0 });
        game.add(t, baku, 2, 40 + i);
        game.set_visible(baku, false);
    }
    let zuba = game.sprite("t_zuba.png");
    game.set_visible(zuba, false);
    game.add(t, zuba, -1, 42);
    let recharge = game.sprite("t_recharge.png");
    game.set_visible(recharge, false);
    game.set_pos(recharge, 120.0, 20.0);
    game.add(t, recharge, 2, 43);
    let convert = game.sprite("t_convert.png");
    game.set_pos(convert, 135.0, 0.0);
    game.set_visible(convert, false);
    game.add(t, convert, 2, 44);
}

/// MotionHelper.Spawn_scale_rotate_move.
pub fn spawn_scale_rotate_move(time: f32, scale: f32, rotate: f32, x: f32, y: f32) -> Action {
    Action::spawn(vec![Action::scale_to(time, scale), Action::rotate_by(time, rotate), Action::move_by(time, (x, y))])
}

/// moveCast : la hauteur, la rotation, et l'image des C.F.F. C et D.
pub fn move_cast(game: &mut Game, _px: f32, py: f32, r: f32) {
    let t = play(game).taichi;
    if py <= 14.0 {
        game.set_y(t.node, py * 100.0);
        game.set_rotation(t.sprite, r);
        if !t.hidden {
            game.set_visible(t.node, true);
        }
    } else {
        game.set_visible(t.node, false);
    }
    if t.d_frame != -1 {
        cff_d_enterframe(game);
    } else if t.c_frame != -1 {
        cff_c_enterframe(game);
    }
}

/// bounce : les sons du rebond, au volume de la vitesse.
pub fn bounce(game: &mut Game, vx: f32, vy: f32) {
    let horizontal = (f64::from(vx * 10.0) * 0.01).min(1.0) as f32;
    let vertical = (f64::from(vy * 10.0) * 0.01).min(1.0) as f32;
    if f64::from(horizontal) >= 0.03 {
        game.play_effect_gain("seogg/se07.ogg", horizontal);
    }
    if f64::from(vertical) >= 0.03 {
        game.play_effect_gain("seogg/se14.ogg", vertical);
    }
}

/// cff_start : la teinte du champ, les mots RECHARGE / CONVERT, les anneaux.
pub fn cff_start(game: &mut Game, color: [u8; 3], recharge: bool, convert: bool) {
    let t = play(game).taichi;
    game.stop_actions(t.node);
    game.run(t.sprite, Action::tint_to(1.5, color));
    if let Some(r) = game.child(t.node, 0x2b)
        && recharge
    {
        game.set_visible(r, true);
        game.run(r, Action::sequence(vec![Action::delay(1.5), Action::hide()]));
        if let Some(c) = game.child(t.node, 0x2c)
            && convert
        {
            game.set_visible(c, true);
            game.run(c, Action::sequence(vec![Action::delay(1.5), Action::hide()]));
        }
    }
    for i in 0..3 {
        let Some(ring) = game.child(t.node, 0x1e + i) else { continue };
        game.set_rotation(ring, 0.0);
        game.set_scale(ring, 1.0);
        let action = match i {
            0 => Action::sequence(vec![
                Action::show(),
                Action::call(|game: &mut Game, _| {
                    game.play_effect("seogg/se09x3.ogg");
                }),
                Action::spawn(vec![Action::rotate_to_xy(0.5, 180.0, 0.0), Action::scale_to(0.5, 0.72)]),
                Action::spawn(vec![Action::rotate_to(0.5, 90.0), Action::scale_to(0.5, 0.43)]),
                Action::hide(),
            ]),
            1 => Action::sequence(vec![Action::show(), Action::delay(0.2), Action::spawn(vec![Action::rotate_to_xy(1.0, 0.0, 180.0), Action::scale_to(1.0, 0.43)]), Action::hide()]),
            _ => Action::sequence(vec![Action::show(), Action::delay(0.4), Action::spawn(vec![Action::rotate_to(1.0, 180.0), Action::scale_to(1.0, 0.43)]), Action::hide()]),
        };
        game.run(ring, action);
    }
}

/// C.F.F. A : se01 et Taichi.cff_a (Lua).
pub fn cff_a(game: &mut Game) {
    game.play_effect("seogg/se01.ogg");
    let t = play(game).taichi.node;
    let (Some(baku), Some(double)) = (game.child(t, 10), game.child(t, 11)) else { return };
    game.set_scale(baku, 0.1);
    game.set_rotation(baku, 0.0);
    game.set_color(baku, [0, 200, 0]);
    let spin = Action::spawn(vec![Action::scale_to(0.3, 1.0), Action::rotate_to(0.3, 359.0)]);
    game.run(baku, Action::sequence(vec![Action::show(), Action::delay(0.05), spin, Action::delay(0.25), Action::hide()]));
    game.set_scale(double, 0.6);
    game.run(double, Action::sequence(vec![Action::show(), Action::scale_to(0.5, 1.0), Action::hide()]));
}

/// C.F.F. B, charge : se09 et Taichi.cff_b_charge.
pub fn cff_b_charge(game: &mut Game) {
    game.play_effect("seogg/se09.ogg");
    let t = play(game).taichi.node;
    let (Some(batch), Some(charge)) = (game.child(t, 20), game.child(t, 21)) else { return };
    game.set_scale(batch, 1.0);
    game.set_visible(batch, true);
    game.run(batch, Action::sequence(vec![Action::scale_to(0.5, 0.5), Action::hide()]));
    game.set_scale(charge, 0.6);
    game.run(charge, Action::sequence(vec![Action::show(), Action::scale_to(0.5, 1.0), Action::hide()]));
}

/// C.F.F. B, relâche : Taichi.cff_b_release puis se141413.
pub fn cff_b_release(game: &mut Game) {
    let t = play(game).taichi.node;
    let get = |game: &Game, tag| game.child(t, tag);
    let (Some(shines), Some(b0), Some(b1), Some(b2), Some(z), Some(bs)) = (get(game, 20), get(game, 10), get(game, 40), get(game, 41), get(game, 42), get(game, 5)) else { return };
    game.stop_actions(t);
    game.set_scale(shines, 0.5);
    game.set_visible(shines, true);
    game.run(shines, Action::sequence(vec![Action::scale_to(0.5, 1.0), Action::hide()]));
    game.set_pos(b0, 0.0, -10.0);
    for b in [b0, b1, b2] {
        game.set_color(b, [255, 127, 0]);
    }
    game.set_visible(b0, true);
    game.set_scale(b0, 0.05);
    game.set_rotation(b0, 0.0);
    game.run(b0, Action::sequence(vec![Action::delay(0.4), Action::show(), Action::scale_to(0.2, 1.0), Action::delay(0.8), Action::hide()]));
    game.set_scale(b1, 0.05);
    game.run(b1, Action::sequence(vec![Action::show(), Action::scale_to(0.4, 0.4), Action::hide()]));
    game.set_scale(b2, 0.05);
    game.run(b2, Action::sequence(vec![Action::delay(0.2), Action::show(), Action::scale_to(0.4, 0.4), Action::hide()]));
    game.set_scale(z, 0.1);
    game.set_rotation(z, 135.0);
    game.set_pos(z, 20.0, 10.0);
    game.run(
        z,
        Action::sequence(vec![
            Action::show(),
            Action::delay(0.4),
            spawn_scale_rotate_move(0.2, 0.4, -75.0, 0.0, 33.0),
            spawn_scale_rotate_move(0.15, 0.65, -65.0, -67.0, -16.0),
            spawn_scale_rotate_move(0.65, 1.0, 0.0, -38.0, -42.0),
            Action::hide(),
        ]),
    );
    game.set_visible(bs, true);
    game.set_scale(bs, 0.05);
    game.set_pos(bs, 0.0, 0.0);
    game.run(bs, Action::sequence(vec![Action::show(), Action::delay(0.4), Action::scale_to(0.2, 1.0), Action::move_by(0.5, (76.0, 76.0)), Action::delay(0.1), Action::hide()]));
    game.play_effect("seogg/se141413.ogg");
}

/// C.F.F. C : les traînées, le compteur, se10 en boucle.
pub fn cff_c(game: &mut Game) {
    let t = play(game).taichi;
    play(game).taichi.c_frame = 0;
    game.set_visible(t.lights, true);
    game.set_visible(t.counter, true);
    let assets = game.assets.clone();
    let id = game.audio.play_effect(&assets, "seogg/se10.ogg", true, 1.0);
    play(game).taichi.c_sound = id;
}

fn cff_c_enterframe(game: &mut Game) {
    let t = play(game).taichi;
    play(game).taichi.c_frame += 1;
    let n = play(game).taichi.c_frame as f32;
    game.set_y(t.sprite, (n * 0.157_079_5).sin() * 60.0);
    for (i, (speed, limit)) in [(6.6, -66.0), (13.2, -132.0), (19.8, -198.0)].into_iter().enumerate() {
        let Some(light) = game.child(t.lights, i as i32) else { continue };
        let x = (f64::from(game.pos(light).0) - speed) as f32;
        game.set_x(light, x);
        if x <= limit {
            let y = game.rng.lrand48() as f32 * 4.656_613e-8 - 50.0;
            game.set_pos(light, 0.0, y);
        }
    }
}

pub fn cff_c_stop(game: &mut Game) {
    let t = play(game).taichi;
    play(game).taichi.c_frame = -1;
    game.set_visible(t.lights, false);
    game.set_visible(t.counter, false);
    game.audio.stop_effect(t.c_sound);
    game.set_pos(t.sprite, 0.0, 0.0);
}

/// C.F.F. D : Taichi s'écrase au sol avec le mot BOOST.
pub fn cff_d(game: &mut Game) {
    let t = play(game).taichi;
    play(game).taichi.d_frame = 0;
    let Some(boost) = game.child(t.node, 5) else { return };
    game.set_visible(boost, true);
    game.set_rotation(boost, -10.0);
    if let Some(n) = game.g.get_mut(boost) {
        n.skew.0 = 10.0;
    }
    game.set_pos(boost, -50.0, 50.0);
    game.set_pos(t.sprite, 0.0, -20.0);
}

fn cff_d_enterframe(game: &mut Game) {
    let t = play(game).taichi;
    let boost = game.child(t.node, 5);
    if t.d_frame < 0x28 {
        play(game).taichi.d_frame += 1;
        let s = (play(game).taichi.d_frame as f32 * 0.078_539_75).sin();
        let sx = (s * 50.0 + 100.0) * 0.01;
        let sy = (100.0 - s * 50.0) * 0.01;
        if let Some(n) = game.g.get_mut(t.node) {
            n.scale = (sx, sy);
        }
        if let Some(b) = boost
            && let Some(n) = game.g.get_mut(b)
        {
            n.scale = ((0.7 / f64::from(sx)) as f32, (0.7 / f64::from(sy)) as f32);
        }
    } else {
        game.set_scale(t.node, 1.0);
        game.set_pos(t.sprite, 0.0, 0.0);
        play(game).taichi.d_frame = -1;
        if let Some(b) = boost {
            game.set_visible(b, false);
        }
    }
}

