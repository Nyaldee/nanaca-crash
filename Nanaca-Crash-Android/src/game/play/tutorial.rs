//! Le tutoriel (SceneTutorial, StateInitTutorial, StateTutorialStep1 à 5,
//! Step1Gauge/Start/Play, StateTutorialSkip) : Miki explique dans sa
//! boîte (msgbox) ; on lance Taichi (étape 1), on regarde les contacts
//! (étape 2), on essaie l'attaque aérienne (3) et un spécial (4), puis la
//! fin (5) ou SKIP. Chaque étape rejoue la partie depuis une situation
//! fixée (setDemonstrationParams), figée tant que Miki parle.

use std::rc::Rc;

use super::msgbox::MsgBox;
use super::{State, cast, control, demo, effects, hud, play};
use crate::action::Action;
use crate::game::{Game, SceneKind, Transition, widgets};
use crate::node::NodeId;

/// Les états du tutoriel.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Step {
    Init,
    Step1,
    Gauge,
    Start,
    Step1Play,
    Step2,
    Step3,
    Step4,
    Step5,
    Skip,
}

/// Le trait rouge de l'attaque aérienne et les flèches qui clignotent.
const LINE: i32 = 7999;
const ARROW: i32 = 0x1f41;

pub struct Tutorial {
    pub step: Step,
    /// L'étape 1, mise de côté pendant la jauge, l'élan et le vol.
    saved: Option<Step>,
    /// Ouvert depuis les options (la fin y ramène).
    from_options: bool,
    msg: Option<MsgBox>,
    /// L'avancement de l'étape (textes lus), et pour l'étape 2 le
    /// personnage montré, pour l'étape 3 les attaques aériennes faites.
    sub: i32,
    cycle: i32,
    aerials: i32,
    /// Figé (0x7f), à figer à l'image suivante (0x80), temps écoulé et
    /// durée (0x84, 0x88).
    paused: bool,
    freeze: bool,
    timer: f32,
    duration: f32,
    /// StateInitTutorial : chargé, puis une seconde d'attente.
    loaded: bool,
    wait: f32,
    /// Le toucher en cours a-t-il glissé (20 points) ?
    touch: (f32, f32),
    moved: bool,
    skip_menu: NodeId,
    skip_item: NodeId,
}

fn tut(game: &mut Game) -> &mut Tutorial {
    play(game).tutorial.as_mut().expect("pas de tutoriel en cours")
}

pub fn new(from_options: bool, fake: NodeId) -> Tutorial {
    Tutorial {
        step: Step::Init,
        saved: None,
        from_options,
        msg: None,
        sub: 0,
        cycle: 0,
        aerials: 0,
        paused: true,
        freeze: false,
        timer: 0.0,
        duration: 0.0,
        loaded: false,
        wait: 2.0,
        touch: (0.0, 0.0),
        moved: false,
        skip_menu: fake,
        skip_item: fake,
    }
}

/// La boîte de Miki (créée à la première étape).
fn with_msg(game: &mut Game, f: impl FnOnce(&mut Game, &mut MsgBox)) {
    if let Some(mut m) = tut(game).msg.take() {
        f(game, &mut m);
        tut(game).msg = Some(m);
    }
}

fn say(game: &mut Game, key: &str) {
    let text = game.text(key);
    with_msg(game, |game, m| m.set_script(game, &text));
}

fn msg_finished(game: &mut Game) -> bool {
    tut(game).msg.as_ref().is_some_and(|m| m.finished)
}

fn show_msg(game: &mut Game, on: bool) {
    with_msg(game, |game, m| m.show(game, on));
}

/// changeWindowSize puis setPositionY : la boîte avec visage (125 de
/// haut, à 0.6 de l'écran) ou sans (60, plus haut).
fn msg_layout(game: &mut Game, height: f32, with_face: bool, y: f32) {
    let h = game.design.1;
    with_msg(game, |game, m| {
        m.resize(game, (400.0, height), with_face);
        m.set_y(game, h * y);
    });
}

// --- StateInitTutorial -------------------------------------------------------------------

/// StateInitTutorial::completeLoaded : les couches, SKIP à la place de
/// la pause, l'effet d'entrée, puis une seconde avant l'étape 1.
pub fn complete_loaded(game: &mut Game) {
    super::setup_layers(game);
    post_setup_layers(game);
    effects::scene_enter(game);
    tut(game).wait = 1.0;
    tut(game).loaded = true;
}

/// SceneTutorial::postSetupLayers : le bouton pause remplacé par SKIP.
fn post_setup_layers(game: &mut Game) {
    let (w, _) = game.design;
    let scene = play(game).scene;
    if let Some(pause) = game.child(scene, 1000) {
        game.g.remove(pause);
    }
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let skip = widgets::pause_item(game, "SKIP", (120.0, 40.0), 1, Rc::new(|game: &mut Game, _| menu_skip(game)));
    game.set_pos(skip, w - 60.0, 20.0);
    game.add(menu, skip, 0, 1);
    game.add(scene, menu, 1000, 1000);
    let t = tut(game);
    t.skip_menu = menu;
    t.skip_item = skip;
}

/// setSkipEnable : SKIP actif ou grisé.
fn set_skip_enabled(game: &mut Game, on: bool) {
    let (menu, item) = (tut(game).skip_menu, tut(game).skip_item);
    game.set_menu_enabled(menu, on);
    widgets::set_item_enabled(game, item, on);
}

// --- Les changements d'état ------------------------------------------------------------------

fn change(game: &mut Game, step: Step) {
    let old = tut(game).step;
    if old == Step::Start {
        // StateTutorialStep1Start::exit.
        let scene = play(game).scene;
        game.stop_actions(scene);
        game.audio.pause_music(false);
    }
    tut(game).step = step;
    play(game).state = if step == Step::Start { State::Start } else { State::Play };
    match step {
        Step::Step1 => step1_enter(game),
        Step::Gauge => set_skip_enabled(game, false),
        Step::Start => start_enter(game),
        Step::Step1Play => step1_play_enter(game),
        Step::Step2 => step2_enter(game),
        Step::Step3 => step3_enter(game),
        Step::Step4 => step4_enter(game),
        Step::Step5 => step5_enter(game),
        Step::Skip => skip_enter(game),
        Step::Init => {}
    }
}

/// Une étape suivante qui garde l'étape 1 de côté (pushState).
fn push(game: &mut Game, step: Step) {
    if tut(game).saved.is_none() {
        let cur = tut(game).step;
        tut(game).saved = Some(cur);
        change(game, step);
    }
}

/// setDemonstrationParams avec les champs de l'étape : rejoué, sans
/// gel, chrono à zéro.
fn params(game: &mut Game, duration: f32, flight: (f32, f32, f32, f32), tags: &[i32]) {
    demo::place(game, flight, tags);
    let t = tut(game);
    t.paused = false;
    t.freeze = false;
    t.timer = 0.0;
    t.duration = duration;
}

/// Chaque image : la boîte écrit, l'état avance.
pub fn update(game: &mut Game, dt: f32) {
    with_msg(game, |game, m| m.update(game));
    match tut(game).step {
        Step::Init => {
            let t = tut(game);
            if t.loaded {
                if t.wait > 0.0 {
                    t.wait -= dt;
                } else {
                    t.loaded = false;
                    change(game, Step::Step1);
                }
            }
        }
        Step::Step1Play => step1_play_execute(game, dt),
        Step::Step2 => step2_execute(game, dt),
        Step::Step3 => step3_execute(game, dt),
        Step::Step4 => step4_execute(game, dt),
        Step::Step5 | Step::Skip => {
            if !tut(game).paused {
                control::execute(game, dt);
                let t = tut(game);
                if t.freeze {
                    t.freeze = false;
                    t.paused = true;
                } else {
                    t.timer += dt;
                }
            }
        }
        Step::Step1 | Step::Gauge | Step::Start => {}
    }
}

// --- Étape 1 : le lancement ------------------------------------------------------------------

/// StateTutorialStep1::enter : la boîte apparaît, la musique du tutoriel.
fn step1_enter(game: &mut Game) {
    tut(game).saved = None;
    let scene = play(game).scene;
    let m = MsgBox::new(game, scene);
    tut(game).msg = Some(m);
    set_skip_enabled(game, false);
    tut(game).sub = 0;
    let speed = play(game).hud.speed;
    game.set_visible(speed, false);
    say(game, "Step1_Text_0");
    reset_scene(game);
    with_msg(game, |game, m| m.pop_in(game));
    game.play_music("bgmogg/bgm2.ogg", true, 0.0);
    game.play_effect("spvoiceogg/tutorial01.ogg");
}

/// StateTutorialStep1::resetScene : Taichi debout, la cible, la jauge.
fn reset_scene(game: &mut Game) {
    let w = game.design.0;
    hud::reset_meter(game);
    let p = play(game);
    let (out, game_layer, bg, scene, taichi) = (p.hud.out_marker, p.game_layer, p.bg, p.scene, p.taichi.node);
    game.set_visible(out, false);
    game.set_scale(game_layer, 1.0);
    game.set_scale(bg, 1.0);
    for tag in [0x12d, 300] {
        if let Some(n) = game.child(scene, tag) {
            game.set_visible(n, false);
        }
    }
    if let Some(stand) = game.child(game_layer, 0x68) {
        game.set_visible(stand, true);
    }
    game.set_visible(taichi, false);
    if let Some(allow) = game.child(game_layer, 0xca) {
        game.set_pos(allow, (f64::from(w) * 0.6) as f32, 170.0);
        game.set_visible(allow, true);
        game.stop_actions(allow);
        game.run(allow, Action::forever(Action::jump_by(0.4, (0.0, 0.0), -6.0, 1)));
    }
    if let Some(target) = game.child(game_layer, 0xc9) {
        game.set_visible(target, true);
    }
}

/// StateTutorialStep1::cbTapHandler.
fn step1_tap(game: &mut Game) {
    match tut(game).sub {
        s @ 0..=1 => {
            tut(game).sub = s + 1;
            say(game, &format!("Step1_Text_{}", s + 1));
        }
        2 => {
            game.play_effect("spvoiceogg/tutorial04.ogg");
            show_msg(game, false);
            hud::start_needle(game);
            push(game, Step::Gauge);
        }
        3 => {
            let jm = play(game).jump_meter;
            game.set_visible(jm, true);
            hud::reset_meter(game);
            say(game, "Step1_Text_2");
            tut(game).sub = 2;
        }
        4 => {
            show_msg(game, false);
            change(game, Step::Step2);
        }
        _ => {}
    }
}

/// StateTutorialStep1::resume : retour après le vol ; recommencer si
/// l'angle (30 à 60°) ou la puissance (80 %) ne vont pas.
fn step1_resume(game: &mut Game) {
    let (angle, power) = (play(game).meter_angle, play(game).meter_power);
    if !(30.0..=60.0).contains(&angle) || power < 80.0 {
        game.play_effect("spvoiceogg/tutorial03.ogg");
        tut(game).sub = 3;
        let text = printf_2_0f(&game.text("Step1_Text_3"), &[angle, power]);
        with_msg(game, |game, m| m.set_script(game, &text));
        show_msg(game, true);
        reset_scene(game);
    } else {
        tut(game).sub = 4;
        game.play_effect("spvoiceogg/tutorial02.ogg");
        say(game, "Step1_Text_4");
        show_msg(game, true);
    }
}

/// string_format pour « %2.0f » : les valeurs arrondies, deux caractères
/// au moins.
fn printf_2_0f(format: &str, values: &[f32]) -> String {
    let mut out = String::new();
    let mut values = values.iter();
    let mut rest = format;
    while let Some(i) = rest.find("%2.0f") {
        out.push_str(&rest[..i]);
        out.push_str(&format!("{:2.0}", values.next().copied().unwrap_or(0.0)));
        rest = &rest[i + 5..];
    }
    out.push_str(rest);
    out
}

/// StateTutorialStep1Start::enter : Nanaka percute Taichi debout.
/// Taichi en vol à la moitié de sa propre hauteur, comme StateStart::enter.
fn start_enter(game: &mut Game) {
    let w = game.design.0;
    let t = play(game).taichi.node;
    let h = game.size(t).1;
    game.set_pos(t, 50.0 + w * 0.5, h * 0.5);
    let nanaca = play(game).nanaca.node;
    play(game).nanaca.stage_pos = 0.0;
    game.set_pos(nanaca, 0.0, 0.0);
    cast::nanaca_start_crash(game);
    let scene = play(game).scene;
    game.run(scene, Action::sequence(vec![Action::delay(1.5), Action::call(|game: &mut Game, _| super::crash_end(game))]));
    game.run(t, Action::sequence(vec![Action::delay(0.5), Action::move_by(0.5, (-100.0, 0.0))]));
}

/// SceneTutorial::cbCrashEnd : 1.5 s de vol (StateTutorialStep1Play).
pub fn crash_end(game: &mut Game) {
    let t = tut(game);
    t.duration = 1.5;
    t.timer = 0.0;
    change(game, Step::Step1Play);
}

/// StateTutorialStep1Play::enter : le vol depuis la jauge, sans personne.
fn step1_play_enter(game: &mut Game) {
    control::initialize(game);
    play(game).c.stack.clear();
    tut(game).paused = false;
    set_skip_enabled(game, true);
}

fn step1_play_execute(game: &mut Game, dt: f32) {
    if tut(game).paused {
        return;
    }
    control::execute(game, dt);
    let t = tut(game);
    t.timer += dt;
    if t.duration > 0.0 && t.duration <= t.timer && t.saved == Some(Step::Step1) {
        t.saved = None;
        t.step = Step::Step1;
        step1_resume(game);
    }
}

// --- Étape 2 : les contacts ------------------------------------------------------------------

/// La boîte reprend sa taille avec visage, à 0.6 de l'écran.
fn msg_default(game: &mut Game) {
    msg_layout(game, 125.0, true, 0.6);
}

/// Les étapes 2 à 5 et SKIP : la boîte remise, figé.
fn step_enter_common(game: &mut Game, text: &str) {
    let t = tut(game);
    t.saved = None;
    t.freeze = false;
    t.paused = true;
    play(game).c.demo = true;
    msg_default(game);
    say(game, text);
}

fn step2_enter(game: &mut Game) {
    step_enter_common(game, "Step2_Text_0");
    tut(game).sub = 1;
    reset_tutorial_scene(game);
    show_msg(game, true);
    with_msg(game, |game, m| m.pop_in(game));
}

/// resetTutorialScene : rien de l'étape 1 ne reste, Taichi caché.
fn reset_tutorial_scene(game: &mut Game) {
    let p = play(game);
    let (game_layer, bg, scene, taichi, sprite, jm) = (p.game_layer, p.bg, p.scene, p.taichi.node, p.taichi.sprite, p.jump_meter);
    game.set_scale(game_layer, 1.0);
    game.set_scale(bg, 1.0);
    for tag in [0x68, 0xca, 0xc9] {
        if let Some(n) = game.child(game_layer, tag) {
            game.set_visible(n, false);
        }
    }
    game.set_visible(jm, false);
    for tag in [0x195, 0x12d, 300] {
        if let Some(n) = game.child(scene, tag) {
            game.set_visible(n, false);
        }
    }
    game.set_visible(taichi, false);
    game.set_opacity(sprite, 0xFF);
}

fn step2_execute(game: &mut Game, dt: f32) {
    if tut(game).paused {
        return;
    }
    control::execute(game, dt);
    let t = tut(game);
    if t.freeze {
        t.freeze = false;
        t.paused = true;
        return;
    }
    t.timer += dt;
    if t.timer < t.duration {
        return;
    }
    t.paused = true;
    // Les démonstrations qui recommencent en boucle.
    match t.sub {
        2 => {
            t.cycle += 1;
            let tag = match t.cycle {
                1 => 0x6b,
                2 => 0x6a,
                _ => {
                    t.cycle = 0;
                    0x6c
                }
            };
            params(game, 1.5, (7.0, 2.0, 10.0, 0.0), &[tag]);
            let c = &mut play(game).c;
            c.kiri = false;
            c.miki = false;
            c.touko = false;
        }
        3 => {
            t.cycle += 1;
            let tag = if t.cycle == 1 {
                0x70
            } else {
                t.cycle = 0;
                0x6f
            };
            params(game, 2.0, (7.0, 2.0, 10.0, 0.0), &[tag]);
        }
        4 => {
            params(game, 2.5, (6.5, 1.0, 15.0, 0.0), &[0x6d, 0x6e]);
            play(game).c.nanaca_first = true;
        }
        _ => {}
    }
}

/// StateTutorialStep2::cbTapHandler : le texte suivant et sa
/// démonstration (filles, garçons, Youko, Misato).
fn step2_tap(game: &mut Game) {
    let sub = tut(game).sub;
    if sub < 6 {
        say(game, &format!("Step2_Text_{sub}"));
        tut(game).sub = sub + 1;
    }
    let sub = tut(game).sub;
    match sub {
        2 => {
            msg_layout(game, 60.0, false, 0.7);
            game.audio.stop_all_effects();
            tut(game).cycle = 0;
            params(game, 1.5, (7.0, 2.0, 10.0, 0.0), &[0x6c]);
        }
        3 => {
            game.audio.stop_all_effects();
            tut(game).cycle = 0;
            params(game, 2.0, (7.0, 2.0, 10.0, 0.0), &[0x6f]);
        }
        4 => {
            game.audio.stop_all_effects();
            params(game, 2.5, (6.5, 1.0, 15.0, 0.0), &[0x6d, 0x6e]);
            play(game).c.nanaca_first = true;
        }
        5 => {
            game.audio.stop_all_effects();
            params(game, 2.0, (9.0, 5.0, 2.0, -10.0), &[0x6e]);
            play(game).c.nanaca_first = true;
        }
        6 => {
            msg_default(game);
            tut(game).sub = 7;
        }
        7 => {
            show_msg(game, false);
            change(game, Step::Step3);
        }
        _ => {}
    }
}

// --- Étape 3 : l'attaque aérienne ------------------------------------------------------------

fn step3_enter(game: &mut Game) {
    let (w, _) = game.design;
    step_enter_common(game, "Step3_Text_0");
    tut(game).sub = 1;
    reset_tutorial_scene(game);
    show_msg(game, true);
    with_msg(game, |game, m| m.pop_in(game));
    params(game, -1.0, (10.0, 10.0, 5.0, -5.0), &[]);
    tut(game).freeze = true;
    // La ligne au-dessus de laquelle l'attaque est possible.
    let scene = play(game).scene;
    let line = game.g.color_layer([0xFF, 0, 0, 0xFF], (w, 4.0));
    if let Some(n) = game.g.get_mut(line) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.set_pos(line, 0.0, 48.0);
    game.add(scene, line, LINE, LINE);
    let arrow = blinking_arrow(game, (0.0, 10.0));
    game.set_pos(arrow, w * 0.5, 100.0);
    game.set_rotation(arrow, -90.0);
    game.add(scene, arrow, ARROW, ARROW);
}

/// indicateallow.png, rouge clignotant qui bat dans une direction.
fn blinking_arrow(game: &mut Game, step: (f32, f32)) -> NodeId {
    let arrow = game.sprite("indicateallow.png");
    game.set_color(arrow, [0xFF, 0, 0]);
    let tint = Action::sequence(vec![Action::tint_to(0.2, [0xFF; 3]), Action::tint_to(0.2, [0xFF, 0, 0])]);
    let bob = Action::sequence(vec![Action::move_by(0.2, step), Action::move_by(0.2, (-step.0, -step.1))]);
    game.run(arrow, Action::forever(Action::spawn(vec![tint, bob])));
    arrow
}

fn step3_execute(game: &mut Game, dt: f32) {
    if tut(game).paused {
        return;
    }
    control::execute(game, dt);
    let t = tut(game);
    if t.freeze {
        t.freeze = false;
        t.paused = true;
        return;
    }
    t.timer += dt;
    if !(t.duration <= 0.0 || t.timer < t.duration) || t.sub != 2 {
        return;
    }
    if t.aerials < 4 {
        let c = &play(game).c;
        if c.vx <= 0.0 && c.vy <= 0.0 {
            // Taichi s'est arrêté : on recommence.
            let t = tut(game);
            t.sub = 1;
            t.paused = true;
            let c = &mut play(game).c;
            c.ups = 3;
            c.aerial_gauge = 100.0;
            reset_daggers(game);
            show_msg(game, true);
            say(game, "Step3_Text_2");
            params(game, -1.0, (10.0, 10.0, 5.0, -5.0), &[]);
            tut(game).freeze = true;
        }
    } else {
        let t = tut(game);
        t.sub = 4;
        t.freeze = true;
        show_msg(game, true);
        say(game, "Step3_Text_3");
        game.play_effect("spvoiceogg/tutorial06.ogg");
    }
}

/// Les trois poignards des attaques en chute, comme neufs.
fn reset_daggers(game: &mut Game) {
    let daggers = play(game).hud.daggers;
    for i in 0..3 {
        if let Some(d) = game.child(daggers, i) {
            game.set_scale(d, 1.0);
            game.set_opacity(d, 0xFF);
            game.set_visible(d, true);
        }
    }
}

fn remove_tagged(game: &mut Game, tag: i32) {
    let scene = play(game).scene;
    if let Some(n) = game.child(scene, tag) {
        game.g.remove(n);
    }
}

fn step3_tap(game: &mut Game) {
    if tut(game).sub == 2 {
        msg_default(game);
        remove_tagged(game, LINE);
        remove_tagged(game, ARROW);
    }
    let sub = tut(game).sub;
    if sub < 2 {
        say(game, &format!("Step3_Text_{sub}"));
        tut(game).sub = sub + 1;
    } else if sub == 2 {
        game.play_effect("spvoiceogg/tutorial04.ogg");
        show_msg(game, false);
        let t = tut(game);
        t.freeze = false;
        t.paused = false;
        t.aerials = 0;
        t.duration = -1.0;
    } else if sub == 4 || sub == 5 {
        say(game, &format!("Step3_Text_{sub}"));
        let t = tut(game);
        t.aerials = 0;
        t.sub = sub + 1;
    } else if sub == 6 {
        show_msg(game, false);
        change(game, Step::Step4);
        reset_daggers(game);
        let c = &mut play(game).c;
        c.ups = 3;
        c.aerial_gauge = 100.0;
    }
}

/// SceneTutorial::cbAerialEnd : une attaque de plus.
pub fn aerial_end(game: &mut Game) {
    if tut(game).step == Step::Step3 {
        let c = &mut play(game).c;
        c.hit_enabled = true;
        c.aerial_allowed = true;
        tut(game).aerials += 1;
    }
}

// --- Étape 4 : le spécial --------------------------------------------------------------------

fn step4_enter(game: &mut Game) {
    step_enter_common(game, "Step4_Text_0");
    tut(game).sub = 1;
    reset_tutorial_scene(game);
    show_msg(game, true);
    with_msg(game, |game, m| m.pop_in(game));
    params(game, -1.0, (7.5, 5.0, 5.0, -5.0), &[0x6b, 0x6c]);
    tut(game).freeze = true;
}

fn step4_execute(game: &mut Game, dt: f32) {
    if tut(game).paused {
        return;
    }
    control::execute(game, dt);
    let t = tut(game);
    if t.freeze {
        t.freeze = false;
        t.paused = true;
        return;
    }
    t.timer += dt;
    if !(t.duration > 0.0 && t.duration <= t.timer) {
        return;
    }
    match t.sub {
        3 => match play(game).c.special_state {
            // Réussi : le combo continue (6 s en tout).
            1 => {
                tut(game).duration = 6.0;
                msg_layout(game, 60.0, false, 0.8);
                show_msg(game, true);
                say(game, "Step4_Text_3");
                tut(game).sub = 4;
            }
            0 => {
                let t = tut(game);
                t.freeze = true;
                t.paused = true;
                show_msg(game, true);
                say(game, "Step4_Text_4");
                tut(game).sub = 5;
            }
            _ => {}
        },
        4 => {
            let t = tut(game);
            t.freeze = true;
            t.paused = true;
        }
        _ => {}
    }
}

fn step4_tap(game: &mut Game) {
    match tut(game).sub {
        s @ 1..=2 => {
            say(game, &format!("Step4_Text_{s}"));
            tut(game).sub = s + 1;
        }
        0 => {
            say(game, "Step4_Text_0");
            tut(game).sub = 1;
        }
        3 => {
            show_msg(game, false);
            let t = tut(game);
            t.freeze = false;
            t.paused = false;
            t.duration = 2.0;
        }
        4 => {
            if tut(game).paused {
                msg_default(game);
                say(game, "Step4_Text_5");
                tut(game).sub = 6;
                // La flèche vers le panneau des spéciaux.
                let arrow = blinking_arrow(game, (10.0, 0.0));
                game.set_pos(arrow, 170.0, 30.0);
                let scene = play(game).scene;
                game.add(scene, arrow, ARROW, ARROW);
            }
        }
        5 => {
            tut(game).sub = 3;
            show_msg(game, true);
            say(game, "Step4_Text_2");
            params(game, -1.0, (7.5, 5.0, 5.0, -5.0), &[0x6b, 0x6c]);
            tut(game).freeze = true;
        }
        6 => {
            remove_tagged(game, ARROW);
            show_msg(game, false);
            change(game, Step::Step5);
        }
        _ => {}
    }
}

// --- Étape 5 et SKIP : la fin ----------------------------------------------------------------

fn step5_enter(game: &mut Game) {
    step_enter_common(game, "Step5_Text_0");
    tut(game).sub = 1;
    reset_tutorial_scene(game);
    show_msg(game, true);
    with_msg(game, |game, m| m.pop_in(game));
    params(game, 3.5, (6.5, 10.0, 15.0, 0.0), &[]);
    tut(game).freeze = true;
    game.play_effect("spvoiceogg/tutorial05.ogg");
}

fn step5_tap(game: &mut Game) {
    let sub = tut(game).sub;
    if sub < 5 {
        say(game, &format!("Step5_Text_{sub}"));
        // L'image offerte (le joueur est toujours « connecté »).
        if sub == 1 {
            game.unlock(0x1d);
        }
        tut(game).sub = sub + 1;
    } else if sub == 5 {
        show_msg(game, false);
        leave(game);
    }
}

fn skip_enter(game: &mut Game) {
    step_enter_common(game, "Skip_Text_0");
    tut(game).sub = 1;
    reset_tutorial_scene(game);
    show_msg(game, true);
    with_msg(game, |game, m| m.pop_in(game));
    params(game, 3.5, (6.5, 10.0, 15.0, 0.0), &[]);
    tut(game).freeze = true;
    set_skip_enabled(game, false);
}

fn skip_tap(game: &mut Game) {
    match tut(game).sub {
        1 => {
            say(game, "Skip_Text_1");
            tut(game).sub = 2;
        }
        2 => {
            show_msg(game, false);
            leave(game);
        }
        _ => {}
    }
}

/// La fin : une partie (SceneAgain), ou les options si on en venait.
fn leave(game: &mut Game) {
    game.audio.stop_music();
    if tut(game).from_options {
        game.replace_scene_with(SceneKind::Option, Transition::CrossFade(0.6));
    } else {
        game.replace_scene_with(SceneKind::Again, Transition::Progress { duration: 0.5, midpoint: (0.5, 0.0), rate: (1.0, 0.0) });
    }
}

/// SceneTutorial::menuSkip : la démonstration se fige, Miki se tait,
/// puis l'adieu de SKIP.
fn menu_skip(game: &mut Game) {
    let step = tut(game).step;
    if matches!(step, Step::Step1Play | Step::Step2 | Step::Step3 | Step::Step4 | Step::Step5) {
        tut(game).paused = true;
    }
    if matches!(step, Step::Step1 | Step::Step1Play | Step::Step2 | Step::Step3 | Step::Step4 | Step::Step5) {
        show_msg(game, false);
        with_msg(game, |game, m| m.set_script(game, ""));
        change(game, Step::Skip);
    }
}

// --- Le toucher (SceneTutorial) ----------------------------------------------------------------

/// ccTouchBegan : pendant la jauge, la puissance commence.
pub fn touch_began(game: &mut Game, point: (f32, f32)) {
    if tut(game).step == Step::Gauge && !play(game).powering {
        hud::start_power(game);
    }
    let t = tut(game);
    t.moved = false;
    t.touch = point;
}

pub fn touch_moved(game: &mut Game, point: (f32, f32)) {
    let t = tut(game);
    let (dx, dy) = (point.0 - t.touch.0, point.1 - t.touch.1);
    if (dx * dx + dy * dy).sqrt() >= 20.0 {
        t.moved = true;
    }
}

/// ccTouchEnded : la jauge lâchée lance Nanaka ; à l'étape 3 l'attaque
/// aérienne, à l'étape 4 le spécial ; sinon (sans glisser) Miki continue.
pub fn touch_ended(game: &mut Game) {
    match tut(game).step {
        Step::Gauge => {
            if play(game).powering {
                hud::finish_power(game);
                play(game).powering = false;
                change(game, Step::Start);
            }
        }
        Step::Step3 => {
            if !tut(game).paused {
                let c = &mut play(game).c;
                if c.aerial_available {
                    let falling = c.vy < 0.0;
                    if falling {
                        c.ups -= 1;
                    } else {
                        c.aerial_gauge = 0.0;
                        control::update_aerial_state(game, true, true, 0.0);
                    }
                    play(game).c.aerial_allowed = false;
                    super::aerial_crash(game, falling);
                }
                return;
            }
        }
        Step::Step4 => {
            let paused = tut(game).paused;
            let c = &mut play(game).c;
            if !paused && c.special_timer > 0.0 {
                c.special_state = 1;
            }
        }
        _ => {}
    }
    if tut(game).moved || !msg_finished(game) {
        return;
    }
    match tut(game).step {
        Step::Step1 => step1_tap(game),
        Step::Step2 => step2_tap(game),
        Step::Step3 => step3_tap(game),
        Step::Step4 => step4_tap(game),
        Step::Step5 => step5_tap(game),
        Step::Skip => skip_tap(game),
        _ => {}
    }
}
