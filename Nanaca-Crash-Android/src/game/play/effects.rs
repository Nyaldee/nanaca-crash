//! Les effets d'écran : les images de la couche d'effets (SceneGame.lua,
//! effectSprites), les fonctions ScreenEffect du script, et les traits de
//! vitesse (EffectsLayer). Les particularités du script sont gardées (un
//! tableau d'actions réutilisé, un second COMBO jamais animé).

use crate::action::{Action, Ease};
use crate::game::Game;
use crate::node::{Kind, Node, NodeId, Sprite};

use super::play;

/// Étiquettes des images de la couche d'effets.
pub const YELLOW: i32 = 1;
pub const CRASH: i32 = 3;
pub const AERIAL: i32 = 4;
pub const DIAGONAL: i32 = 5;
pub const WHITE: i32 = 6;
pub const METU: i32 = 7;
pub const CFF_BACK: i32 = 8;
pub const CFF_TEXT: i32 = 9;
pub const COMBO: i32 = 14;
pub const LINES: i32 = 100;

/// La couche d'effets (EffectsLayer) et ses images (effectSprites).
pub fn effects_layer(game: &mut Game) -> NodeId {
    let (w, h) = game.design;
    let layer = game.g.color_layer([0, 0, 0, 0], (w, h));
    let lines = game.g.draw_node();
    game.set_pos(lines, w * 0.5, h * 0.5);
    game.add(layer, lines, 100, LINES);
    game.set_visible(lines, false);
    let yellow = game.sprite_file("flash3.png");
    game.add(layer, yellow, 1, YELLOW);
    let crash = game.sprite("crash.png");
    game.add(layer, crash, 3, CRASH);
    let aerial = game.sprite("aerialcrash.png");
    game.add(layer, aerial, 4, AERIAL);
    let diagonal = game.sprite_file("flash1.png");
    game.add(layer, diagonal, 5, DIAGONAL);
    let white = game.g.color_layer([0xFF; 4], (w, h));
    game.add(layer, white, 6, WHITE);
    let back = game.sprite_file("flash2.png");
    game.add(layer, back, 8, CFF_BACK);
    let text = game.sprite("t_cff.png");
    game.set_pos(text, w / 2.0, h / 2.0);
    game.add(layer, text, 9, CFF_TEXT);
    for (k, name) in ["t_typea.png", "t_typeb.png", "t_typec.png", "t_typed.png"].into_iter().enumerate() {
        let t = game.sprite(name);
        game.set_pos(t, -200.0, h * 0.6);
        game.add(layer, t, 10, 10 + k as i32);
    }
    let combo = game.sprite("t_combo.png");
    game.set_pos(back, w / 2.0, h / 2.0);
    game.set_pos(yellow, w / 2.0, h / 2.0);
    game.set_pos(combo, -200.0, h * 0.75);
    game.add(layer, combo, 10, COMBO);
    game.set_color(aerial, [0x33, 0x33, 0xFF]);
    game.set_anchor(aerial, 0.5, 0.5);
    game.set_pos(aerial, w / 2.0, h / 2.0);
    game.set_anchor(crash, 0.5, 0.0);
    game.set_pos(crash, w / 2.0, 175.0);
    game.set_pos(diagonal, w / 2.0, h / 2.0);
    for id in [back, text, crash, aerial, yellow, diagonal, white] {
        game.set_visible(id, false);
    }
    layer
}

/// EffectsLayer::update : les traits de vitesse, redessinés toutes les
/// 0.08 s quand ils sont visibles.
pub fn update(game: &mut Game, dt: f32) {
    let layer = play(game).effects;
    play(game).lines_timer += dt;
    let Some(lines) = game.child(layer, LINES) else { return };
    if !game.visible(lines) || play(game).lines_timer < 0.08 {
        return;
    }
    let radius = (f64::from(game.design.0 * 0.5) * std::f64::consts::SQRT_2) as f32;
    let mut triangles = Vec::new();
    let mut angle = 6.28318f32;
    loop {
        let (s, c) = (angle.sin(), angle.cos());
        let (s2, c2) = ((angle + 0.05).sin(), (angle + 0.05).cos());
        let (x, y) = (s * radius, c * radius);
        let k = game.rng.lrand48() as f32 * 4.656_613e-10;
        let inner = (x * 0.5 - x * k * 0.2, y * 0.5 - y * k * 0.2);
        triangles.push(([inner, (x, y), (s2 * radius, c2 * radius)], [0.75, 0.5, 0.0, 1.0]));
        angle -= game.rng.lrand48() as f32 * 1.137_824_3e-10;
        if angle < 0.0 {
            break;
        }
    }
    if let Some(n) = game.g.get_mut(lines)
        && let Kind::Draw(t) = &mut n.kind
    {
        *t = triangles;
    }
    play(game).lines_timer = 0.0;
}

fn effect(game: &Game, tag: i32) -> Option<NodeId> {
    game.child(play_ref(game).effects, tag)
}

fn play_ref(game: &Game) -> &super::Play {
    match &game.logic {
        crate::game::Logic::Play(p) => p,
        _ => unreachable!("la partie n'est pas la scène en cours"),
    }
}

/// ScreenEffect.SceneEnter : la scène part de la lettre T de TARGET.
pub fn scene_enter(game: &mut Game) {
    let w = game.design.0;
    let (scene, game_layer) = (play(game).scene, play(game).game_layer);
    let Some(target) = game.child(game_layer, 201) else { return };
    let t_size = game.size(target);
    let offset = game.child(target, 0).map_or(0.0, |l| game.size(l).0) + 4.0;
    let posx = w * 0.57 - t_size.0 / 2.0 + offset;
    game.set_anchor(scene, posx / w, 0.65);
    game.set_scale(scene, 20.0);
    game.run(scene, Action::sequence(vec![Action::ease(Action::scale_to(0.4, 1.0), Ease::Out(4.0))]));
}

/// ScreenEffect.InitialCrash : Taichi s'envole au choc de Nanaka.
pub fn initial_crash(game: &mut Game) {
    let p = play(game);
    let (game_layer, taichi) = (p.game_layer, p.taichi.node);
    game.set_visible(taichi, true);
    for tag in [104, 201, 202] {
        if let Some(n) = game.child(game_layer, tag) {
            game.set_visible(n, false);
        }
    }
    let (Some(yellow), Some(lines), Some(crash)) = (effect(game, YELLOW), effect(game, LINES), effect(game, CRASH)) else { return };
    game.set_opacity(yellow, 255);
    game.set_scale(yellow, 1.0);
    game.run(lines, Action::sequence(vec![Action::show(), Action::delay(0.66), Action::hide()]));
    // Le script ajoute ces actions au tableau déjà donné aux traits.
    game.run(yellow, Action::sequence(vec![Action::show(), Action::delay(0.66), Action::hide(), Action::show(), Action::scale_to(0.33, 3.0), Action::fade_out(0.22), Action::hide()]));
    game.set_scale(crash, 0.0);
    game.set_opacity(crash, 255);
    game.run(crash, Action::sequence(vec![Action::show(), Action::scale_to(0.44, 1.0), Action::delay(0.44), Action::fade_out(0.2), Action::hide()]));
}

/// ScreenEffect.ComboEffect.
pub fn combo(game: &mut Game) {
    let (w, h) = game.design;
    let layer = play(game).effects;
    // Un second COMBO est ajouté, mais c'est le premier qui est animé.
    let extra = game.sprite("t_combo.png");
    game.set_pos(extra, -200.0, h * 0.6);
    game.add(layer, extra, 10, COMBO);
    if let Some(d) = effect(game, DIAGONAL) {
        game.set_visible(d, true);
        game.set_opacity(d, 255);
        game.run(d, Action::sequence(vec![Action::show(), Action::delay(1.2), Action::fade_out(0.3), Action::hide()]));
    }
    if let Some(t) = effect(game, COMBO) {
        game.set_x(t, -200.0);
        let y = game.pos(t).1;
        game.run(t, Action::sequence(vec![Action::show(), Action::delay(0.333), Action::move_to(0.333, (w / 2.0, y)), Action::delay(0.6666), Action::move_to(0.333, (w + 200.0, y)), Action::hide()]));
    }
}

/// ScreenEffect.MisatoEffect.
pub fn misato(game: &mut Game) {
    let (Some(yellow), Some(lines)) = (effect(game, YELLOW), effect(game, LINES)) else { return };
    game.set_opacity(yellow, 255);
    game.set_scale(yellow, 1.0);
    game.run(lines, Action::sequence(vec![Action::show(), Action::delay(0.66), Action::hide()]));
    game.run(yellow, Action::sequence(vec![Action::show(), Action::delay(0.66), Action::hide(), Action::show(), Action::scale_to(0.33, 3.0), Action::fade_out(0.1), Action::hide()]));
}

/// SetupToukoEffect : trois lames (sprites vides) avec leur explosion et
/// leur BOOST, et le mot ZUBA, dans la couche de Touko.
pub fn setup_touko_effect(game: &mut Game, touko: NodeId) {
    for (i, (x, y)) in [(110.0, 100.0), (100.0, 70.0), (90.0, 120.0)].into_iter().enumerate() {
        let mut node = Node::new(Kind::Sprite(Sprite::empty()));
        node.anchor = (0.5, 0.5);
        let s = game.g.add(node);
        game.set_pos(s, x, y);
        game.add(touko, s, 1001 + i as i32, 1001 + i as i32);
        let baku = game.sprite("bakucff.png");
        let boost = game.sprite("t_boost.png");
        game.set_visible(baku, false);
        game.set_opacity(baku, 0x7F);
        game.set_color(baku, [0xFF, 0xFF, 0]);
        game.set_pos(baku, 120.0, 40.0);
        game.set_anchor(boost, 0.0, 0.0);
        game.set_visible(boost, false);
        game.set_pos(boost, 120.0, 40.0);
        if let Some(n) = game.g.get_mut(boost) {
            n.skew.0 = 10.0;
        }
        game.set_rotation(boost, -10.0);
        game.add(s, baku, 1, 1);
        game.add(s, boost, 2, 2);
    }
    let zuba = game.sprite("t_zuba.png");
    game.set_visible(zuba, false);
    game.set_pos(zuba, 0.0, 0.0);
    game.add(touko, zuba, 1004, 1004);
}

/// ScreenEffect.WhiteEffect : l'écran blanc, le flash, et les lames de
/// Touko.
pub fn white_effect(game: &mut Game, touko: NodeId) {
    let (Some(white), Some(metu)) = (effect(game, WHITE), effect(game, METU)) else { return };
    game.set_visible(white, true);
    game.set_opacity(white, 255);
    game.set_visible(metu, true);
    game.set_opacity(metu, 0);
    game.run(white, Action::sequence(vec![Action::show(), Action::delay(1.55), Action::hide()]));
    game.run(metu, Action::sequence(vec![Action::show(), Action::delay(1.5), Action::fade_in(0.0), Action::delay(0.5), Action::fade_out(0.3), Action::hide()]));
    let blades: Vec<_> = (1..=4).filter_map(|i| game.assets.frame(&format!("blade{i}.png"))).map(|f| (f, 1.0)).collect();
    for i in 0..3 {
        let Some(s) = game.child(touko, 1001 + i) else { continue };
        let wait = 0.2 * i as f32;
        game.set_opacity(s, 255);
        game.run(s, Action::sequence(vec![Action::delay(wait), Action::animate(blades.clone(), 0.1, 1, true), Action::fade_out(0.5)]));
        if let Some(baku) = game.child(s, 1) {
            game.set_scale(baku, 0.0);
            game.set_opacity(baku, 0x7F);
            game.run(baku, Action::sequence(vec![Action::show(), Action::delay(wait), Action::scale_to(0.4, 0.5), Action::fade_out(0.2), Action::hide()]));
        }
        if let Some(boost) = game.child(s, 2) {
            game.set_pos(boost, 120.0, 40.0);
            game.set_scale(boost, 0.0);
            game.set_opacity(boost, 255);
            game.run(boost, Action::sequence(vec![Action::show(), Action::delay(wait), Action::scale_to(0.4, 1.0), Action::move_by(0.4, (150.0, 25.0)), Action::hide()]));
        }
    }
    let Some(z) = game.child(touko, 1004) else { return };
    game.set_scale(z, 0.0);
    game.set_rotation(z, 135.0);
    game.set_pos(z, 280.0, 150.0);
    game.run(
        z,
        Action::sequence(vec![
            Action::show(),
            Action::delay(0.5),
            super::taichi::spawn_scale_rotate_move(0.2, 0.4, -75.0, 0.0, 33.0),
            super::taichi::spawn_scale_rotate_move(0.15, 0.65, -65.0, -67.0, -16.0),
            super::taichi::spawn_scale_rotate_move(0.65, 1.0, 0.0, -38.0, -42.0),
            Action::hide(),
        ]),
    );
}

/// ScreenEffect.CFFEffect : le fond, COMIC FORCE FIELD et le type.
pub fn cff(game: &mut Game, kind: i32) {
    let w = game.design.0;
    let (Some(back), Some(text), Some(kind_text)) = (effect(game, CFF_BACK), effect(game, CFF_TEXT), effect(game, 10 + kind)) else { return };
    game.set_opacity(back, 255);
    game.run(back, Action::sequence(vec![Action::show(), Action::delay(0.8), Action::fade_out(0.4), Action::hide()]));
    game.set_opacity(text, 255);
    game.set_scale(text, 0.5);
    game.run(text, Action::sequence(vec![Action::show(), Action::scale_to(0.5, 1.0), Action::fade_out(1.0), Action::hide()]));
    game.set_x(kind_text, -200.0);
    let y = game.pos(kind_text).1;
    game.run(kind_text, Action::sequence(vec![Action::show(), Action::delay(0.333), Action::move_to(0.333, (w / 2.0, y)), Action::delay(1.5), Action::move_to(0.333, (w + 200.0, y)), Action::hide()]));
}

/// ScreenEffect.GameOverMotion : le score vient au centre.
pub fn game_over_motion(game: &mut Game) {
    let (w, h) = game.design;
    let record = play(game).hud.record;
    let size = game.size(record);
    game.set_anchor(record, 1.1, 0.5);
    game.set_pos(record, w - size.0, h - 50.0);
    game.run(record, Action::spawn(vec![Action::move_to(0.5, (w / 2.0 - size.0 / 2.0, h * 0.5)), Action::scale_to(0.5, 1.0)]));
}

/// ScreenEffect.AerialHudMotion : le HUD aérien s'écarte pendant
/// l'attaque, le poignard utilisé s'efface, AERIAL CRASH traverse l'écran.
pub fn aerial_hud_motion(game: &mut Game, upper: bool, upper_rest: i32) {
    let (w, h) = game.design;
    let hud = play(game).hud;
    let hide_and_back = || vec![Action::move_by(0.2, (-120.0, 0.0)), Action::delay(1.0), Action::move_by(0.2, (120.0, 0.0))];
    for i in 0..4 {
        let Some(dagger) = game.child(hud.daggers, i) else { continue };
        let first = if i != 3 && i == upper_rest {
            Action::spawn(vec![Action::scale_to(0.66, 2.0), Action::fade_to(0.66, 0)])
        } else if !upper && i == 3 {
            Action::tint_to(0.66, [0xCC; 3])
        } else {
            Action::delay(0.66)
        };
        let mut actions = vec![first];
        actions.extend(hide_and_back());
        game.run(dagger, Action::sequence(actions));
    }
    for id in [hud.aerial_label, hud.aerial_indicator] {
        let mut actions = vec![Action::delay(0.66)];
        actions.extend(hide_and_back());
        game.run(id, Action::sequence(actions));
    }
    let Some(logo) = effect(game, AERIAL) else { return };
    let size = game.size(logo);
    game.set_pos(logo, -size.0, h / 2.0);
    game.set_visible(logo, false);
    game.set_scale(logo, 1.0);
    game.set_color(logo, if upper { [0xFF, 0x33, 0x33] } else { [0x33, 0x33, 0xFF] });
    game.run(logo, Action::sequence(vec![Action::delay(0.5), Action::show(), Action::move_to(0.3, (w / 2.0, h / 2.0)), Action::delay(0.4), Action::scale_to_xy(0.33, 2.0, 0.0), Action::hide()]));
}
