//! La partie (SceneGame, SceneGameDelegate, les états et SceneGame.lua) :
//! chargement, jauge, élan de Nanaka, vol, pause et fin de partie.

mod backgrounds;
mod cast;
mod control;
mod demo;
mod effects;
mod fireworks;
mod hud;
mod msgbox;
mod taichi;
mod tutorial;

use crate::action::Action;
use crate::game::{Game, Logic, SceneKind, Transition};
use crate::node::NodeId;

use backgrounds::Background;
use cast::Cast;
use control::Control;
use hud::{Hud, HudFlags};
use taichi::Taichi;

/// Les états de la partie (CCStateMachineDelegate).
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    None,
    /// Chargement des ressources.
    Init,
    /// La jauge de lancement.
    Gauge,
    /// Nanaka percute Taichi.
    Start,
    /// Le vol.
    Play,
    Pause,
    GameOver { distance: f32, angle: f32, power: f32 },
}

pub struct Play {
    /// La couche SceneGame (étiquette 0x1234).
    pub scene: NodeId,
    pub state: State,
    /// L'état d'avant la pause (0x158).
    pub prev_state: Option<State>,
    pub c: Control,
    pub game_layer: NodeId,
    pub bg: NodeId,
    pub far: NodeId,
    pub effects: NodeId,
    pub fireworks: Option<fireworks::Fireworks>,
    pub flash: Option<NodeId>,
    pub jump_meter: NodeId,
    pub powering: bool,
    pub meter_angle: f32,
    pub meter_power: f32,
    pub taichi: Taichi,
    pub nanaca: Cast,
    /// Les personnages (Youko, Tomoki, Sakuraba, Misato, Kiri, Miki,
    /// Touko) et le blocage de Youko.
    pub casts: Vec<Cast>,
    pub block: Cast,
    /// Le son du chrono du spécial (arrêté au résultat).
    pub special_sound: u32,
    pub hud: Hud,
    pub hud_special_clip: bool,
    pub flags: HudFlags,
    pub background: Option<Background>,
    pub toy_base: f32,
    /// Couleur des textes du skin (noir, blanc pour le feu d'artifice).
    pub font_color: [u8; 3],
    /// Le record au départ.
    pub best: f32,
    /// Temps de vol.
    pub elapsed: f32,
    /// Chargement terminé (StateInit).
    pub loaded: bool,
    pub lines_timer: f32,
    /// Une animation de la galerie (GalleryDemo) : son nom, la
    /// démonstration et la page de la galerie autour.
    demo_name: Option<String>,
    demo: Option<demo::Demo>,
    pub viewer: Option<Box<crate::game::gallery::Viewer>>,
    /// Le tutoriel (SceneTutorial).
    tutorial: Option<tutorial::Tutorial>,
}

pub fn play(game: &mut Game) -> &mut Play {
    match &mut game.logic {
        Logic::Play(p) => p,
        _ => unreachable!("la partie n'est pas la scène en cours"),
    }
}

/// SceneGame::scene, init et onEnter : la couche vide, l'état de chargement.
pub fn enter(game: &mut Game, root: NodeId) {
    enter_with(game, root, None);
    game.audio.stop_music();
}

/// GalleryDemo::onEnter : la partie, chargée comme SceneGame (la musique
/// de la galerie continue), avec l'interface commune de la galerie.
pub fn enter_demo(game: &mut Game, root: NodeId, id: i32, name: &str) {
    enter_with(game, root, Some(name.to_string()));
    let scene = play(game).scene;
    play(game).viewer = Some(crate::game::gallery::demo_viewer(scene, id));
    crate::game::gallery::setup_demo_ui(game);
}

/// SceneTutorial::onEnter : la partie chargée comme SceneGame, la musique
/// arrêtée ; `from_options` : la fin ramène aux options.
pub fn enter_tutorial(game: &mut Game, root: NodeId, from_options: bool) {
    enter_with(game, root, None);
    game.audio.stop_music();
    let fake = play(game).scene;
    play(game).tutorial = Some(tutorial::new(from_options, fake));
}

fn enter_with(game: &mut Game, root: NodeId, demo_name: Option<String>) {
    // SceneGame::init relit ses feuilles (oubliées après un changement de
    // skin, avec l'image de Taichi).
    game.assets.add_sprite_frames("gamesheet.plist");
    game.assets.add_sprite_frames("backgrounds.plist");
    let scene = game.g.layer(game.design);
    game.add(root, scene, 0, 0x1234);
    let fake = game.g.node();
    let hud = Hud {
        record: fake,
        best: fake,
        speed: fake,
        speed_label: fake,
        speed_arrow: fake,
        speed_mode: 2,
        out_marker: fake,
        out_icon: fake,
        out_label: fake,
        special: fake,
        indicator: fake,
        contact: fake,
        contact_draw: fake,
        contact_frame: 0,
        pause_button: fake,
        pause_layer: fake,
        pause_menu: fake,
        aerial_indicator: fake,
        daggers: fake,
        aerial_label: fake,
    };
    let taichi = Taichi { node: fake, sprite: fake, lights: fake, counter: fake, c_frame: -1, d_frame: -1, c_sound: 0, hidden: false };
    game.logic = Logic::Play(Box::new(Play {
        scene,
        state: State::None,
        prev_state: None,
        c: Control::default(),
        game_layer: fake,
        bg: fake,
        far: fake,
        effects: fake,
        fireworks: None,
        flash: None,
        jump_meter: fake,
        powering: false,
        meter_angle: 0.0,
        meter_power: 0.0,
        taichi,
        nanaca: Cast::placeholder(fake),
        casts: Vec::new(),
        block: Cast::placeholder(fake),
        special_sound: 0,
        hud,
        hud_special_clip: false,
        flags: HudFlags::default(),
        background: None,
        toy_base: 400.0,
        font_color: [0, 0, 0],
        best: 0.0,
        elapsed: 0.0,
        loaded: false,
        lines_timer: 0.0,
        demo_name,
        demo: None,
        viewer: None,
        tutorial: None,
    }));
    game.loader_start();
    game.apply_volumes();
    change_state(game, State::Init);
}

/// changeState : sortie de l'ancien état, entrée du nouveau.
pub fn change_state(game: &mut Game, state: State) {
    let old = play(game).state;
    exit_state(game, old);
    play(game).state = state;
    enter_state(game, state);
}

fn exit_state(game: &mut Game, state: State) {
    let scene = play(game).scene;
    match state {
        State::Gauge => game.stop_actions(scene),
        State::Start => {
            game.stop_actions(scene);
            game.audio.pause_music(false);
        }
        _ => {}
    }
}

fn enter_state(game: &mut Game, state: State) {
    match state {
        State::Gauge => {
            let (record, color) = (play(game).hud.record, play(game).font_color);
            hud::set_score_color(game, record, color);
        }
        State::Start => start_enter(game),
        State::Play => control::start(game),
        State::Pause => {
            game.audio.pause_music(true);
            game.audio.stop_all_effects();
        }
        State::GameOver { distance, angle, power } => game_over_enter(game, distance, angle, power),
        State::None | State::Init => {}
    }
}

/// SceneGame::update : l'état en cours ; puis les traits de vitesse.
pub fn update(game: &mut Game, dt: f32) {
    let demo = play(game).demo.is_some();
    if play(game).tutorial.is_some() {
        // StateInitTutorial : les couches une fois chargé, puis les étapes.
        if play(game).state == State::Init && game.loader_progress() == 1.0 && !play(game).loaded {
            play(game).loaded = true;
            tutorial::complete_loaded(game);
        }
        tutorial::update(game, dt);
        if play(game).loaded {
            effects::update(game, dt);
            fireworks::update(game, dt);
        }
        return;
    }
    match play(game).state {
        State::Init => {
            if game.loader_progress() == 1.0 && !play(game).loaded {
                play(game).loaded = true;
                match play(game).demo_name.clone() {
                    // StateInitDemo::completeLoaded.
                    Some(name) => {
                        setup_layers(game);
                        demo::enter(game, &name);
                    }
                    None => complete_loaded(game),
                }
            }
        }
        State::Play if demo => demo::execute(game, dt),
        State::Play => control::execute(game, dt),
        _ => {}
    }
    if play(game).viewer.is_some() {
        crate::game::gallery::update_content(game, dt);
    }
    if play(game).loaded {
        effects::update(game, dt);
        fireworks::update(game, dt);
    }
}

/// StateInit::completeLoaded : les couches, les compteurs, l'entrée.
fn complete_loaded(game: &mut Game) {
    setup_layers(game);
    let best = game.save.float("best_record", 0.0);
    let hud = play(game).hud;
    hud::set_score(game, hud.best, &format!("{best:9.2}:"));
    hud::set_score(game, hud.record, &format!("{:9.2}:", 1e-5));
    effects::scene_enter(game);
    let scene = play(game).scene;
    game.run(scene, Action::sequence(vec![Action::delay(0.5), Action::call(|game: &mut Game, _| opening_finished(game))]));
    play(game).best = best;
    let game_layer = play(game).game_layer;
    if let Some(allow) = game.child(game_layer, 202) {
        game.run(allow, Action::forever(Action::jump_by(0.4, (0.0, 0.0), -6.0, 1)));
    }
}

/// SceneGame::cbOpeningFinished : l'aiguille de la jauge, les compteurs.
fn opening_finished(game: &mut Game) {
    let jm = play(game).jump_meter;
    hud::start_needle(game);
    game.set_visible(jm, true);
    change_state(game, State::Gauge);
    let hud = play(game).hud;
    game.set_visible(hud.record, true);
    game.set_visible(hud.best, true);
}

/// StateStart::enter : Nanaka percute Taichi.
/// Taichi en vol se pose à la moitié de sa propre hauteur (sa taille, pas
/// celle de l'écran), au niveau de Taichi debout.
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
    game.run(scene, Action::sequence(vec![Action::delay(1.5), Action::call(|game: &mut Game, _| crash_end(game))]));
    game.run(t, Action::sequence(vec![Action::delay(0.5), Action::move_by(0.5, (-100.0, 0.0))]));
    game.unlock(0);
}

/// SceneGame::cbCrashEnd : le vol commence.
fn crash_end(game: &mut Game) {
    if play(game).tutorial.is_some() {
        tutorial::crash_end(game);
        return;
    }
    if play(game).demo.is_some() {
        demo::crash_end(game);
        return;
    }
    change_state(game, State::Play);
    let contact = play(game).hud.contact;
    game.set_visible(contact, true);
}

// --- Les couches (SceneGameDelegate::SetupLayers et setupLayers) ---------------------------

fn setup_layers(game: &mut Game) {
    let (w, h) = game.design;
    let scene = play(game).scene;
    let font_color = if game.skin_key() == "firework_show" { [0xFF; 3] } else { [0; 3] };
    play(game).font_color = font_color;
    let (sky_top, sky_bottom) = if game.skin_key() == "firework_show" { ([0x0e, 0x06, 0x6c, 0xff], [0xb4, 0x94, 0xbb, 0xff]) } else { ([0x34, 0xd0, 0xfd, 0xff], [0xff; 4]) };
    let sky = game.g.gradient_layer(sky_top, sky_bottom, (w, h));
    let (bg, far, background) = backgrounds::create(game, (w, h));
    game.set_pos(bg, 0.0, 0.0);
    game.set_pos(far, 0.0, 0.0);
    let p = play(game);
    p.bg = bg;
    p.far = far;
    p.background = Some(background);
    if game.skin_key() == "firework_show" {
        let fw = fireworks::create(game);
        let flash = game.g.color_layer([0xFF; 4], (w, h));
        game.set_visible(flash, false);
        game.add(scene, flash, 1, -1);
        game.set_opacity(flash, 0);
        play(game).fireworks = Some(fw);
        play(game).flash = Some(flash);
    }
    let effects_layer = effects::effects_layer(game);
    play(game).effects = effects_layer;
    let game_layer = game.g.layer((w, h));
    play(game).game_layer = game_layer;
    for id in [game_layer, bg, far] {
        game.set_anchor(id, 0.3, 0.0);
    }
    game.add(scene, sky, -2, -1);
    if let Some(fw) = play(game).fireworks.as_ref().map(|f| f.layer) {
        game.add(scene, fw, 0, -1);
    }
    game.add(scene, far, -1, 0x9999);
    game.add(scene, bg, 1, 2);
    game.add(scene, effects_layer, 2, 3);
    game.add(scene, game_layer, 3, 1);
    // La jauge, le marqueur de hauteur, SPECIAL.
    let jm = hud::jump_meter(game);
    game.set_pos(jm, (f64::from(w) * 0.5 - 100.0) as f32, (f64::from(h) * 0.5 - 35.0) as f32);
    game.set_visible(jm, false);
    game.add(scene, jm, 0x50, 0x66);
    play(game).jump_meter = jm;
    let out = game.g.color_layer([0, 0, 0, 0], (0.0, 0.0));
    let arrow = game.sprite("out.png");
    game.set_anchor(arrow, 0.5, 1.0);
    game.add(out, arrow, 1, 1);
    let fly = game.skin_image("c_taichifly.png", "c_taichifly_summer.png", "c_taichifly_firework.png");
    let icon = game.sprite(fly);
    game.set_pos(icon, 0.0, -90.0);
    game.add(out, icon, 2, 2);
    let out_label = game.atlas_label("0000.00:", "arialblack32-atlas.png", 16.0, 18.0, ' ');
    game.set_color(out_label, font_color);
    game.set_anchor(out_label, 0.0, 0.0);
    game.set_scale(out_label, 0.75);
    game.set_pos(out_label, 55.0, -90.0);
    game.add(out, out_label, 3, 3);
    let arrow_size = game.size(arrow);
    if let Some(n) = game.g.get_mut(out) {
        n.size = arrow_size;
    }
    game.set_visible(out, false);
    game.set_pos(out, w * 0.5, h);
    game.add(scene, out, 0x50, 0x67);
    let with_shine = game.save.int("kamcord_state", -1) < 1;
    let special = hud::special_layer(game, with_shine);
    game.set_pos(special, w * 0.5, h * 0.5 + 20.0);
    game.set_visible(special, false);
    game.add(scene, special, 0x50, 0x72);
    // Taichi, Nanaka, et les personnages.
    let taichi = taichi::create(game);
    game.add(game_layer, taichi.node, 0x110, 100);
    game.set_pos(taichi.node, w * 0.5 + 50.0, h * 0.5);
    game.set_anchor(taichi.node, 0.5, 0.5);
    play(game).taichi = taichi;
    let nanaca = cast::create_nanaca(game);
    game.set_pos(nanaca.node, w * 0.5, 50.0);
    game.add(game_layer, nanaca.node, 100, 0x69);
    play(game).nanaca = nanaca;
    let block = cast::create_youko_block(game);
    game.add(game_layer, block.node, 0x78, 0x71);
    play(game).block = block;
    let order = [cast::Who::Youko, cast::Who::Tomoki, cast::Who::Sakuraba, cast::Who::Misato, cast::Who::Kiri, cast::Who::Miki, cast::Who::Touko];
    for (k, who) in order.into_iter().enumerate() {
        let c = cast::create(game, who, k);
        game.set_pos(c.node, w * 0.5, 0.0);
        let tag = c.tag;
        game.add(game_layer, c.node, 99, tag);
        play(game).casts.push(c);
    }
    game.set_scale(game_layer, 1.0);
    game.set_scale(bg, 1.0);
    // Taichi debout, TARGET, la flèche.
    let stand_name = game.skin_image("taichi_stand.png", "taichi_stand_summer.png", "taichi_stand_firework.png");
    let stand = game.sprite(stand_name);
    game.set_anchor(stand, 0.5, 0.0);
    let sx = (f64::from(w) * 0.6).floor() as f32;
    game.set_pos(stand, sx, 0.0);
    game.set_scale(stand, 0.75);
    game.add(game_layer, stand, 0x100, 0x68);
    let target = game.bm_label("TARGET", "arialblack64short.fnt", 2);
    game.set_anchor(target, 0.5, 0.5);
    game.set_color(target, font_color);
    game.set_pos(target, sx, 210.0);
    game.add(game_layer, target, 0x100, 0xc9);
    // setupLayers (Lua) : HUD aérien, flèche de la cible.
    let (aerial, daggers) = hud::aerial_hud(game, scene);
    let allow = game.sprite("t_targetallow.png");
    game.set_anchor(allow, 0.5, 0.5);
    game.set_pos(allow, sx, 170.0);
    game.set_scale(allow, 0.75);
    game.add(game_layer, allow, 0x100, 202);
    game.set_visible(taichi.node, false);
    // Le bouton pause.
    let pause = game.sprite_file("pause.png");
    let (pw, ph) = game.size(pause);
    let pause_item = game.image_item(pause, |game: &mut Game| menu_pause(game));
    let pause_menu = game.menu();
    game.set_pos(pause_item, (w - pw) * 0.5, (ph - h) * 0.5);
    game.add(pause_menu, pause_item, 0, -1);
    game.add(scene, pause_menu, 1000, 1000);
    // Les compteurs de score, placés d'après leur largeur pour 100000000.00
    // (comme l'original) : les vrais nombres, plus courts, tiennent à
    // l'écran.
    let record = hud::score_label(game, "RECORD:", &format!("{:9.2}:", 1e8));
    hud::set_score_color(game, record, font_color);
    game.set_anchor(record, 1.0, 0.5);
    game.set_scale(record, 0.75);
    game.set_visible(record, false);
    let rs = hud::score_size(game, record);
    game.set_pos(record, (w + 10.0) - rs.0, h - 50.0);
    game.add(scene, record, 0x1f5, 300);
    let best = hud::score_label(game, "BEST RECORD:", &format!("{:9.2}:", 1e8));
    hud::set_score_color(game, best, font_color);
    game.set_anchor(best, 1.0, 0.5);
    game.set_scale(best, 0.75);
    game.set_visible(best, false);
    let bs = hud::score_size(game, best);
    game.set_pos(best, (w + 10.0) - bs.0, h - 30.0);
    game.add(scene, best, 0x1f5, 0x12d);
    // Le compteur de vitesse.
    let speed = game.g.color_layer([0, 0, 0, 0], (0.0, 0.0));
    let speed_arrow = game.sprite("varrow.png");
    game.set_pos(speed_arrow, -100.0, 9.0);
    game.set_rotation(speed_arrow, 0.0);
    game.add(speed, speed_arrow, 2, 2);
    let speed_label = game.atlas_label("0.00:/;", "arialblack32-atlas.png", 16.0, 18.0, ' ');
    game.set_color(speed_label, font_color);
    game.set_scale(speed_label, 0.6);
    game.set_anchor(speed_label, 1.0, 0.5);
    game.set_pos(speed_label, 0.0, 9.0);
    game.add(speed, speed_label, 1, 1);
    game.add(scene, speed, 500, 0x195);
    game.set_pos(speed, w - 5.0, h - 70.0);
    let speed_mode = match game.save.int("speedmeter_state", 0) {
        1 => 0,
        2 => 1,
        _ => 2,
    };
    // La jauge aérienne.
    let aerial_label = game.atlas_label("100:", "digital30-atlas.png", 13.0, 18.0, '0');
    game.add(scene, aerial_label, 0x193, 0x193);
    game.set_color(aerial_label, font_color);
    game.set_pos(aerial_label, 32.0, h - 78.0);
    // Visages, indicateur des spéciaux, menu pause.
    let (contact, contact_draw) = hud::contact_layer(game);
    game.add(scene, contact, 500, 0x194);
    game.set_pos(contact, w - 86.0, h - 90.0);
    let indicator = hud::special_indicator_layer(game);
    game.add(scene, indicator, 500, 0x196);
    game.set_pos(indicator, 20.0, 20.0);
    play(game).hud = Hud {
        record,
        best,
        speed,
        speed_label,
        speed_arrow,
        speed_mode,
        out_marker: out,
        out_icon: icon,
        out_label,
        special,
        indicator,
        contact,
        contact_draw,
        contact_frame: 0,
        pause_button: pause_menu,
        pause_layer: contact,
        pause_menu: contact,
        aerial_indicator: aerial,
        daggers,
        aerial_label,
    };
    let (pause_layer, menu) = hud::pause_layer(game);
    game.add(scene, pause_layer, 0x3e9, 4);
    play(game).hud.pause_layer = pause_layer;
    play(game).hud.pause_menu = menu;
    // L'éclair blanc du feu d'artifice (flash4a) ou le flash (flash4).
    let metu_image = if game.skin_key() == "firework_show" { "flash4a.png" } else { "flash4.png" };
    let metu = game.sprite_file(metu_image);
    game.set_pos(metu, w * 0.5, h * 0.5);
    game.set_visible(metu, false);
    game.add(effects_layer, metu, 7, 7);
}

// --- Le toucher ------------------------------------------------------------------------------

/// ccTouchBegan : pendant la jauge, la puissance commence à pulser.
pub fn touch_began(game: &mut Game, point: (f32, f32)) {
    if play(game).tutorial.is_some() {
        tutorial::touch_began(game, point);
        return;
    }
    if play(game).state == State::Gauge && !play(game).powering {
        hud::start_power(game);
    }
}

/// Le doigt glisse (seul le tutoriel le suit).
pub fn touch_moved(game: &mut Game, point: (f32, f32)) {
    if play(game).tutorial.is_some() {
        tutorial::touch_moved(game, point);
    }
}

/// ccTouchEnded : le lancement, ou en vol le spécial / l'attaque aérienne.
pub fn touch_ended(game: &mut Game) {
    if play(game).tutorial.is_some() {
        tutorial::touch_ended(game);
        return;
    }
    match play(game).state {
        State::Gauge => {
            if play(game).powering {
                hud::finish_power(game);
                change_state(game, State::Start);
            }
        }
        State::Play => {
            if play(game).fireworks.is_some() {
                fireworks::ignite(game);
            }
            let c = &mut play(game).c;
            if c.special_timer > 0.0 {
                c.special_state = 1;
                c.special_timer = -1.0;
            } else if c.aerial_available {
                c.aerial_available = false;
                let falling = c.vy < 0.0;
                if falling {
                    c.ups -= 1;
                } else {
                    c.aerial_gauge = 0.0;
                    control::update_aerial_state(game, true, true, 0.0);
                    let label = play(game).hud.aerial_label;
                    game.g.set_text(label, "  0:");
                    game.set_color(label, [0x66; 3]);
                }
                play(game).c.aerial_allowed = false;
                aerial_crash(game, falling);
            }
        }
        _ => {}
    }
}

/// StatePlay::aerial_crash : Nanaka frappe ; le coup à 0.833 s, la fin à 2 s.
fn aerial_crash(game: &mut Game, upper: bool) {
    cast::nanaca_aerial_crash(game, upper);
    let scene = play(game).scene;
    game.run(scene, Action::sequence(vec![Action::delay(0.833), Action::call(|game: &mut Game, _| aerial_unpause(game))]));
    game.run(scene, Action::sequence(vec![Action::delay(2.0), Action::call(|game: &mut Game, _| aerial_end(game))]));
}

fn aerial_unpause(game: &mut Game) {
    if play(game).flying() {
        control::aerial_boost(game);
        control::slow_with_distance(game, 50.0, 1.0);
    }
}

/// La fin de l'attaque (rien dans les démonstrations : cbAerialEnd vide).
fn aerial_end(game: &mut Game) {
    if play(game).tutorial.is_some() {
        tutorial::aerial_end(game);
        return;
    }
    if play(game).flying() && play(game).demo.is_none() {
        let c = &mut play(game).c;
        c.hit_enabled = true;
        c.aerial_allowed = true;
    }
}

// --- Pause, AGAIN, QUIT -------------------------------------------------------------------------

/// SceneGame::menuPause : seulement en vol ou pendant la jauge.
pub fn menu_pause(game: &mut Game) {
    let state = play(game).state;
    if state != State::Play && state != State::Gauge {
        return;
    }
    play(game).prev_state = Some(state);
    play(game).state = State::Pause;
    enter_state(game, State::Pause);
    game.director_paused = true;
    hud::pause_show(game);
}

/// Start (manette) : la pause, ou RESUME si elle est affichée.
pub fn toggle_pause(game: &mut Game) {
    // Pas de pause dans le tutoriel (SKIP à la place).
    if play(game).tutorial.is_some() {
        return;
    }
    if play(game).state == State::Pause {
        resume(game);
    } else {
        menu_pause(game);
    }
}

/// MSG_PAUSE_HIDE (RESUME) : l'état d'avant reprend.
pub fn resume(game: &mut Game) {
    let layer = play(game).hud.pause_layer;
    game.set_visible(layer, false);
    if let Some(prev) = play(game).prev_state.take() {
        play(game).state = prev;
    }
    game.director_paused = false;
    if play(game).flying() {
        game.audio.pause_music(false);
    }
}

/// Le directeur reprend si la partie était en pause (AGAIN, QUIT).
fn leave_pause(game: &mut Game) {
    if play(game).state == State::Pause {
        if let Some(prev) = play(game).prev_state.take() {
            play(game).state = prev;
        }
        game.director_paused = false;
    }
}

/// SceneGame::cbAgain : une nouvelle partie (SceneAgain).
pub fn again(game: &mut Game) {
    game.audio.stop_music();
    game.audio.stop_all_effects();
    game.play_effect("seogg/se03.ogg");
    let layer = play(game).hud.pause_layer;
    game.set_visible(layer, false);
    leave_pause(game);
    // SceneAgain, l'ancienne image se refermant vers le milieu en largeur.
    game.replace_scene_with(SceneKind::Again, Transition::Progress { duration: 0.5, midpoint: (0.5, 0.0), rate: (1.0, 0.0) });
}

/// SceneGame::cbQuit : retour au titre (entrée glissée).
pub fn quit(game: &mut Game) {
    game.audio.stop_music();
    game.audio.stop_all_effects();
    let layer = play(game).hud.pause_layer;
    game.set_visible(layer, false);
    leave_pause(game);
    // Le titre, l'ancienne image se refermant vers le milieu en hauteur.
    game.replace_scene_with(SceneKind::Title(2), Transition::Progress { duration: 0.5, midpoint: (0.0, 0.5), rate: (0.0, 1.0) });
}

// --- Fin de partie ------------------------------------------------------------------------------

/// StateGameOver::enter : record, score au centre, menu, succès.
fn game_over_enter(game: &mut Game, distance: f32, angle: f32, power: f32) {
    let (w, h) = game.design;
    game.audio.stop_music();
    let mut best = game.save.float("best_record", 0.0);
    if best < distance {
        game.save.set_float("best_record", distance);
        best = distance;
    }
    let hud = play(game).hud;
    hud::set_score(game, hud.record, &format!("{distance:9.2}:"));
    hud::set_score(game, hud.best, &format!("{best:9.2}:"));
    effects::game_over_motion(game);
    hud::pause_show(game);
    let total = game.save.float("stat_total_distance", 0.0) + distance;
    game.save.set_float("stat_total_distance", total);
    set_menu_enabled(game, false);
    // Les succès (comme s'ils étaient envoyés à Google Play), puis le menu.
    let scene = play(game).scene;
    game.run(scene, Action::sequence(vec![Action::delay(0.1), Action::call(move |game: &mut Game, _| send_achievements(game, distance, angle, power))]));
    game.set_visible(hud.pause_button, false);
    // Le lancement parfait : feu d'artifice, ou l'ovni qui tombe.
    if angle >= 89.0 && power >= 90.0 && distance <= 5.0 {
        if game.skin() == 2 {
            fireworks::ignite_special(game);
        } else {
            let ufo = game.skin_image("ufo.png", "tinu.png", "ufo.png");
            let u = game.sprite(ufo);
            game.set_rotation(u, 160.0);
            game.set_pos(u, 50.0 + w * 0.5, h + 50.0);
            game.add(scene, u, 0x32, -1);
            let fall = Action::ease(Action::move_to(1.5, (50.0 + w * 0.5, 0.0)), crate::action::Ease::BounceOut);
            game.run(u, Action::spawn(vec![Action::rotate_by(1.5, 15.0), fall]));
            game.play_effect("seogg/se09.ogg");
        }
    }
}

fn set_menu_enabled(game: &mut Game, on: bool) {
    let menu = play(game).hud.pause_menu;
    game.set_menu_enabled(menu, on);
}

/// sendGameCenterMessagesImpl : les dix vérifications, puis le menu.
fn send_achievements(game: &mut Game, distance: f32, angle: f32, power: f32) {
    for (k, goal) in [1500.0, 3000.0, 7500.0, 15000.0, 42195.0].into_iter().enumerate() {
        if distance >= goal {
            game.unlock(k as i32 + 1);
        }
    }
    let total = game.save.float("stat_total_distance", 0.0);
    for (k, goal) in [10000.0, 100000.0, 500000.0, 1e6].into_iter().enumerate() {
        game.update_achievement(8 + k as i32, (distance * 0.01) as i32, total, goal);
    }
    if angle >= 89.0 && power >= 90.0 && distance <= 5.0 {
        game.unlock(6);
    }
    set_menu_enabled(game, true);
}
