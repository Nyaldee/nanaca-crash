//! Les animations de la galerie (GalleryDemo, StateInitDemo,
//! StatePlayDemo) : le vol de la partie, rejoué depuis une situation fixée
//! (setDemonstrationParams) à chaque bouton (NORMAL, SPECIAL…), puis figé
//! quand la démonstration est finie.

use std::rc::Rc;

use super::{State, cast, control, play};
use crate::game::{Game, widgets};
use crate::node::NodeId;

/// Le menu des démonstrations.
const MENU: i32 = 0x270e;

/// StatePlayDemo.
pub struct Demo {
    /// Le nom de l'élément (Touko, Mens, Nanaca…).
    name: String,
    menu: NodeId,
    /// Figé (0x7f), à figer à l'image suivante (0x80).
    paused: bool,
    freeze: bool,
    /// Temps écoulé et durée de la démonstration (0x84, 0x88).
    timer: f32,
    duration: f32,
    /// Nanaka : attaque aérienne à 0.5 s (0xdc), élan d'ouverture à
    /// lancer (0xdd), fin à 2 s (0xde).
    aerial: bool,
    crash: bool,
    crash_end: bool,
}

fn demo(game: &mut Game) -> &mut Demo {
    play(game).demo.as_mut().expect("pas de démonstration en cours")
}

/// StateInitDemo::completeLoaded puis StatePlayDemo::enter : les couches,
/// la partie sans ses compteurs, le menu, le titre, puis NORMAL figé.
pub fn enter(game: &mut Game, name: &str) {
    let (w, h) = game.design;
    play(game).demo = Some(Demo { name: name.to_string(), menu: play(game).scene, paused: false, freeze: false, timer: 0.0, duration: 0.0, aerial: false, crash: false, crash_end: false });
    play(game).c.demo = true;
    play(game).state = State::Play;
    let (scene, game_layer, taichi) = (play(game).scene, play(game).game_layer, play(game).taichi.node);
    game.set_visible(taichi, true);
    play(game).taichi.hidden = false;
    for tag in [0x68, 0xca, 0xc9] {
        if let Some(n) = game.child(game_layer, tag) {
            game.set_visible(n, false);
        }
    }
    for tag in [1000, 0x12d, 300] {
        if let Some(n) = game.child(scene, tag) {
            game.set_visible(n, false);
        }
    }
    let hud = play(game).hud;
    game.set_visible(hud.indicator, false);
    game.set_visible(hud.speed, false);
    let bg = play(game).bg;
    for id in [game_layer, bg] {
        game.set_anchor(id, 0.3, 0.0);
    }
    // Les boutons : NORMAL et SPECIAL (RABA et TOMOKI, AERIAL, CRASH et
    // CFF, ou BLOCK selon l'élément).
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let labels: &[&str] = match name {
        "Mens" => &["RABA", "TOMOKI"],
        "Nanaca" => &["AERIAL", "CRASH", "CFF"],
        "Youko" => &["NORMAL", "BLOCK"],
        _ => &["NORMAL", "SPECIAL"],
    };
    let y = h - 57.0;
    for (i, text) in labels.iter().enumerate() {
        let tag = i as i32 + 1;
        let item = widgets::pause_item(game, text, (150.0, 44.0), tag, Rc::new(move |game: &mut Game, _| handle_menu_item(game, tag)));
        game.set_pos(item, 75.0, y - 40.0 * i as f32);
        game.add(menu, item, 0, tag);
    }
    game.add(scene, menu, 9999, MENU);
    demo(game).menu = menu;
    let title = game.bm_label(name, "arialblack64short.fnt", 2);
    let size = game.size(title);
    game.set_color(title, [0, 0, 0]);
    game.set_anchor(title, 0.0, 1.0);
    game.set_pos(title, (w - size.0) - 30.0, h - 30.0);
    game.add(scene, title, 9999, 0x270d);
    handle_menu_item_impl(game, 1, false);
}

/// StatePlayDemo::handleMenuItem : le menu glisse à gauche, la
/// démonstration du bouton commence.
fn handle_menu_item(game: &mut Game, tag: i32) {
    demo(game).paused = false;
    handle_menu_item_impl(game, tag, true);
}

fn handle_menu_item_impl(game: &mut Game, tag: i32, pressed: bool) {
    let menu = demo(game).menu;
    if pressed {
        game.set_menu_enabled(menu, false);
        game.run(menu, crate::action::Action::move_by(0.2, (-200.0, 0.0)));
    }
    let name = demo(game).name.clone();
    reset(game, &name, tag, !pressed);
    demo(game).freeze = !pressed;
}

/// StatePlayDemo::reset : la situation de départ de chaque démonstration.
/// Sans nom (fin de démonstration), seuls le gel et le menu reviennent.
fn reset(game: &mut Game, name: &str, mode: i32, freeze: bool) {
    let d = demo(game);
    d.crash_end = false;
    d.crash = false;
    let pair = |mode: i32| if mode == 2 { (6.0, 2) } else { (2.0, 1) };
    match name {
        "Miki" => {
            let (time, count) = pair(mode);
            set_params(game, time, (7.5, 2.0, 5.0, -2.0), &[0x6c, 0x6b][..count]);
        }
        "Kiri" => {
            let (time, count) = pair(mode);
            set_params(game, time, (7.5, 2.0, 5.0, -2.0), &[0x6b, 0x6c][..count]);
        }
        "Touko" => {
            set_params(game, if mode == 2 { 7.5 } else { 2.0 }, (7.5, 2.0, 5.0, -2.0), &[0x6a]);
            play(game).c.touko = mode == 2;
        }
        "Misato" => {
            if mode == 2 {
                set_params(game, 7.5, (7.5, 2.0, 5.0, -2.0), &[0x6e]);
                play(game).c.misato = true;
            } else {
                set_params(game, 1.5, (7.5, 2.0, 5.0, -2.0), &[0x6e]);
                play(game).c.nanaca_first = true;
            }
        }
        "Youko" => {
            // BLOCK : Youko protège Misato.
            let tag = if mode == 1 { 0x6d } else { 0x6e };
            set_params(game, 1.5, (7.5, 2.0, 5.0, -2.0), &[tag]);
            play(game).c.youko_block = mode != 1;
        }
        "Mens" => {
            let tag = if mode == 1 { 0x6f } else { 0x70 };
            set_params(game, 2.0, (7.5, 2.0, 5.0, -2.0), &[tag]);
        }
        "Nanaca" => {
            let game_layer = play(game).game_layer;
            let stand = game.child(game_layer, 0x68);
            if let Some(s) = stand {
                game.set_visible(s, false);
            }
            demo(game).aerial = false;
            match mode {
                // CRASH : l'élan d'ouverture contre Taichi debout.
                2 => {
                    set_params(game, 5.0, (0.0, 0.75, 0.0, 0.0), &[]);
                    let d = demo(game);
                    d.crash = true;
                    d.crash_end = true;
                    d.paused = false;
                    d.freeze = false;
                    let taichi = play(game).taichi.node;
                    game.set_visible(taichi, false);
                    if let Some(s) = stand {
                        game.set_visible(s, true);
                    }
                }
                // CFF : Youko et Nanaka devant un personnage au hasard.
                3 => {
                    let mut tag = (game.rng.lrand48() as f32 * 4.656_613e-8) as i32 % 4 + 0x6a;
                    if tag == 0x6d {
                        tag = 0x6e;
                    }
                    set_params(game, 6.5, (7.5, 2.0, 5.0, -2.0), &[tag]);
                    let c = &mut play(game).c;
                    c.youko_block = true;
                    c.nanaca_first = false;
                    c.nanaca = true;
                }
                // AERIAL : l'attaque aérienne à 0.5 s.
                1 => {
                    set_params(game, 5.0, (7.5, 5.0, 5.0, -2.0), &[]);
                    demo(game).aerial = true;
                }
                _ => {}
            }
        }
        _ => {}
    }
    play(game).c.stop = true;
    if freeze {
        let d = demo(game);
        d.freeze = true;
        d.paused = false;
    }
    let menu = demo(game).menu;
    game.set_pos(menu, 0.0, 0.0);
    game.set_menu_enabled(menu, true);
}

/// StateControlDelegate::setDemonstrationParams : la partie remise à zéro,
/// les personnages hors scène, la pile réduite à ceux de la liste,
/// Taichi en place (px, py, vx, vy), la durée.
fn set_params(game: &mut Game, duration: f32, flight: (f32, f32, f32, f32), tags: &[i32]) {
    place(game, flight, tags);
    let d = demo(game);
    d.paused = false;
    d.timer = 0.0;
    d.freeze = false;
    d.duration = duration;
}

/// setDemonstrationParams, la partie commune aux animations de la galerie
/// et au tutoriel : tout remis à zéro, la pile réduite aux personnages
/// donnés, Taichi en place.
pub(super) fn place(game: &mut Game, (px, py, vx, vy): (f32, f32, f32, f32), tags: &[i32]) {
    control::initialize(game);
    play(game).c.stack.clear();
    let n = play(game).casts.len();
    for k in 0..n {
        cast::demo_clear(game, k);
    }
    for &tag in tags {
        for k in 0..n {
            if play(game).casts[k].tag == tag {
                play(game).c.stack.push_back(k);
            }
        }
    }
    let w = game.design.0;
    // Taichi est ici un calque transparent : l'opacité va à son image.
    let (taichi, sprite) = (play(game).taichi.node, play(game).taichi.sprite);
    game.set_x(taichi, w * 0.5 - 50.0);
    game.set_opacity(sprite, 0xFF);
    game.set_color(taichi, [0xFF; 3]);
    cast::demo_clear_block(game);
    let c = &mut play(game).c;
    c.px = px;
    c.py = py;
    c.vx = vx;
    c.vy = vy;
}

/// StatePlayDemo::execute : le vol, puis le gel à la fin de la
/// démonstration ; Nanaka attaque ou percute au bon moment.
pub fn execute(game: &mut Game, dt: f32) {
    if demo(game).paused {
        return;
    }
    control::execute(game, dt);
    if !matches!(play(game).state, State::Play) {
        return;
    }
    if demo(game).freeze {
        let d = demo(game);
        d.freeze = false;
        d.paused = true;
        return;
    }
    let d = demo(game);
    d.timer += dt;
    let (timer, duration) = (d.timer, d.duration);
    if duration > 0.0 && duration <= timer {
        reset(game, "", 1, true);
    }
    if demo(game).aerial && demo(game).timer >= 0.5 {
        demo(game).aerial = false;
        play(game).c.aerial_allowed = false;
        super::aerial_crash(game, true);
    }
    if demo(game).crash {
        let taichi = play(game).taichi.node;
        game.set_visible(taichi, false);
        demo(game).crash = false;
        // L'élan d'ouverture (StateStart, empilé).
        play(game).state = State::Start;
        super::start_enter(game);
    }
    if demo(game).crash_end && demo(game).timer >= 2.0 {
        reset(game, "", 1, true);
    }
}

/// GalleryDemo::cbCrashEnd : retour à la démonstration, Taichi relancé.
pub fn crash_end(game: &mut Game) {
    play(game).state = State::Play;
    control::boost(game, 20.0, 45.0);
}
