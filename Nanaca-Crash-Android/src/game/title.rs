//! L'écran titre (SceneTitle et SceneTitle.lua) : « NOW LOADING » qui
//! sautille pendant le chargement puis explose, l'entrée des lettres du
//! titre et du menu, Nanaka qui glisse, le fond qui se déroule, les néons.
//! Les boutons Google+, classements, Kamcord, Twitter, boutique et publicités
//! sont retirés ; tous les achats étant acquis, le bouton SOUND remplace
//! MORE GAMES (comme avec l'option sans publicité).

use std::rc::Rc;

use super::{Game, Logic, SceneKind, Transition};
use crate::action::{Action, Ease};
use crate::node::{Kind, Node, NodeId, Progress, Sprite};

/// Le π du script.
const PI: f32 = 3.141592;
/// Étiquettes des nœuds (celles du script).
const MENU_TAG: i32 = 100;
const TITLE_TAG: i32 = 110;
const SOUND_HOP_TAG: i32 = 200;
/// Position des boutons.
const BUTTONS_X: f32 = 30.0;
const BUTTONS_Y: f32 = 180.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Loading,
    Explode,
    TitleHop,
    SlideIn,
    Null,
}

/// Une lettre de « NOW LOADING » : son saut, puis son explosion.
struct LoadLetter {
    id: NodeId,
    px: f32,
    py: f32,
    ex: f32,
    ey: f32,
    offset: f32,
    frames: f32,
    count: f32,
    rot: f32,
    peakx: f32,
    peaky: f32,
    offset2: f32,
    frames2: f32,
    count2: f32,
}

/// Une lettre du titre ou du menu qui arrive en sautant.
struct HopLetter {
    id: NodeId,
    px: f32,
    py: f32,
    orig_scale: f32,
    rot: f32,
    startx: f32,
    starty: f32,
    offset: i32,
    frames: f32,
    count: f32,
    a: f32,
    v0: f32,
}

pub struct Title {
    layer: NodeId,
    state: State,
    assets_loaded: bool,
    no_hop: bool,
    hop_finished: bool,
    is_sound_on: bool,
    load_letters: Vec<LoadLetter>,
    hop_letters: Vec<HopLetter>,
    loading_label: Option<NodeId>,
}

/// La scène titre : la couche SceneTitle (étiquette 0x1234) et son entrée.
pub fn enter(game: &mut Game, root: NodeId, pattern: i32) {
    let layer = game.g.layer(game.design);
    game.g.attach(root, layer, 0, 0x1234);
    game.apply_volumes();
    game.logic = Logic::Title(Box::new(Title {
        layer,
        state: State::Null,
        assets_loaded: false,
        no_hop: false,
        hop_finished: false,
        is_sound_on: false,
        load_letters: Vec::new(),
        hop_letters: Vec::new(),
        loading_label: None,
    }));
    game.loader_start();
    // onEnter du script : le fond blanc, puis l'entrée.
    let white = game.g.color_layer([0xFF; 4], game.design);
    game.g.attach(layer, white, -1, 999);
    match pattern {
        0 => change_state(game, State::Loading),
        1 => {
            title(game).no_hop = true;
            change_state(game, State::TitleHop);
        }
        2 => change_state(game, State::SlideIn),
        _ => {}
    }
    if game.loader_progress() >= 1.0 {
        on_preload(game, 100);
    }
}

fn title(game: &mut Game) -> &mut Title {
    match &mut game.logic {
        Logic::Title(t) => t,
        _ => unreachable!("le titre n'est pas la scène en cours"),
    }
}

/// onPreload du script : tout est chargé à 100.
pub fn on_preload(game: &mut Game, percent: i32) {
    if percent >= 100 {
        title(game).assets_loaded = true;
    }
}

/// Une image (onEnterFrame : l'état en cours).
pub fn update(game: &mut Game) {
    match title(game).state {
        State::Loading => loading_update(game),
        State::Explode => explode_update(game),
        State::TitleHop => hop_update(game),
        State::SlideIn | State::Null => {}
    }
}

fn change_state(game: &mut Game, state: State) {
    title(game).state = state;
    match state {
        State::Loading => loading_enter(game),
        State::Explode => {}
        State::TitleHop => hop_enter(game),
        State::SlideIn => slide_in_enter(game),
        State::Null => {}
    }
}

/// Une étiquette « impact48t » orange à la croix rouge (le « } » est la
/// croix de NANACA†CRASH!!).
fn impact_label(game: &mut Game, text: &str, cross: i32) -> NodeId {
    let w = game.design.0;
    let assets = game.assets.clone();
    let label = game.g.bm_label(&assets, text, "impact48t.fnt", 2);
    let _ = w;
    game.g.set_color(label, [0xFF, 0x66, 0x00]);
    if let Some(c) = game.g.child(label, cross) {
        game.g.set_color(c, [0xFF, 0, 0]);
    }
    label
}

// --- NOW LOADING ---------------------------------------------------------------

fn loading_enter(game: &mut Game) {
    let (w, h) = game.design;
    let layer = title(game).layer;
    let label = impact_label(game, "NOW}LOADING", 3);
    let size = game.g.get(label).map_or((0.0, 0.0), |n| n.size);
    if let Some(n) = game.g.get_mut(label) {
        n.anchor = (0.0, 0.0);
        n.pos = (w - size.0 - 20.0, 10.0);
    }
    game.g.attach(layer, label, 2, 2);
    let mut letters = Vec::new();
    for i in 0.."NOW}LOADING".len() as i32 {
        let Some(id) = game.g.child(label, i) else { continue };
        let (px, py) = game.g.get(id).map_or((0.0, 0.0), |n| n.pos);
        let rot = (game.rng.random() * 4.0).floor() - 1.0;
        let peakx = game.rng.random() * 400.0 - 400.0;
        let peaky = game.rng.random() * h;
        // Départs de l'explosion un peu décalés.
        let offset2 = (game.rng.random() * 15.0).floor();
        letters.push(LoadLetter {
            id,
            px,
            py,
            ex: 125.0,
            ey: 15.0,
            offset: ((px + 150.0) / 40.0).floor(),
            frames: 40.0,
            count: 40.0,
            rot,
            peakx,
            peaky,
            offset2,
            frames2: 80.0 - offset2,
            count2: 80.0 - offset2,
        });
    }
    let t = title(game);
    t.loading_label = Some(label);
    t.load_letters = letters;
}

fn loading_update(game: &mut Game) {
    let loaded = title(game).assets_loaded;
    let mut explode = false;
    let Logic::Title(t) = &mut game.logic else { return };
    for l in &mut t.load_letters {
        if l.count > -20.0 {
            let x = l.px;
            let y = 37.5 * (PI * (l.offset + l.count) / l.frames).sin().max(0.0) + l.py;
            let xscale = (2.0 * PI * (l.offset + l.count).max(0.0).min(l.frames) / l.frames).cos();
            l.count -= 1.0;
            if let Some(n) = game.g.get_mut(l.id) {
                n.pos = (x, y);
                n.scale.0 = xscale;
            }
            if l.count <= -20.0 {
                l.count = l.frames;
                if loaded {
                    explode = true;
                    break;
                }
            }
        }
    }
    if explode {
        change_state(game, State::Explode);
    }
}

fn explode_update(game: &mut Game) {
    let mut done = false;
    let Logic::Title(t) = &mut game.logic else { return };
    for l in &mut t.load_letters {
        if l.count2 >= 0.0 {
            if l.offset2 > 0.0 {
                l.offset2 -= 1.0;
            } else {
                let k = l.count2 / l.frames2;
                let scale = 50.0 * (2.0 * PI * (1.0 - k)).sin() + 50.0 * k + 50.0;
                let x = l.peakx * (PI * (1.0 - k)).sin() + l.ex * (1.0 - k) + l.px * (scale / 100.0);
                let y = l.peaky * (PI * (1.0 - k)).sin() + l.ey * (1.0 - k) + l.py;
                let rotation = -360.0 * l.rot * k;
                if let Some(n) = game.g.get_mut(l.id) {
                    n.pos = (x, y);
                    n.set_scale(scale / 100.0);
                    n.set_rotation(rotation);
                }
                l.count2 -= 1.0;
            }
        } else {
            done = true;
            break;
        }
    }
    if done {
        if let Some(label) = title(game).loading_label
            && let Some(c) = game.g.child(label, 3)
        {
            game.g.set_color(c, [0xFF, 0x66, 0x00]);
        }
        title(game).no_hop = false;
        change_state(game, State::TitleHop);
    }
}

// --- Entrée glissée --------------------------------------------------------------

fn slide_in_enter(game: &mut Game) {
    let w = game.design.0;
    let layer = title(game).layer;
    let label = impact_label(game, "NANACA}CRASH!!", 6);
    let size = game.g.get(label).map_or((0.0, 0.0), |n| n.size);
    if let Some(n) = game.g.get_mut(label) {
        n.anchor = (0.0, 0.0);
        n.pos = (0.0, 100.0);
        n.set_scale(1.0);
    }
    game.g.attach(layer, label, 2, 2);
    let time = 0.5;
    let spawn = Action::spawn(vec![Action::scale_to(time, 0.5), Action::move_to(time, (w - size.0 / 2.0 - 10.0, 40.0))]);
    let call = Action::call(|game: &mut Game, _| change_state(game, State::TitleHop));
    game.run(label, Action::sequence(vec![spawn, call]));
}

// --- Le titre et son menu ----------------------------------------------------------

/// Un bouton du titre : étiquette « arialblack64short » noire, sans lissage.
fn title_button(game: &mut Game, text: &str, x: f32, y: f32, action: fn(&mut Game)) -> NodeId {
    let (w, assets) = (game.design.0, game.assets.clone());
    let label = game.g.bm_label(&assets, text, "arialblack64short.fnt", 0);
    let _ = w;
    alias_font(game, "arialblack64short.fnt");
    game.g.set_color(label, [0, 0, 0]);
    let item = game.item_label(label, Some(Rc::new(move |game: &mut Game, _| action(game))));
    let size = game.g.get(item).map_or((0.0, 0.0), |n| n.size);
    if let Some(n) = game.g.get_mut(item) {
        n.anchor = (0.5, 0.5);
        n.pos = (size.0 / 2.0 + x, size.1 / 2.0 + y);
    }
    item
}

/// setAliasTexParameters sur la page d'une police.
fn alias_font(game: &mut Game, font: &str) {
    if let Some(f) = game.assets.font(font) {
        game.assets.set_alias(&f.page);
    }
}

fn hop_enter(game: &mut Game) {
    let (w, h) = game.design;
    let skin = game.skin();
    let assets = game.assets.clone();
    let t = title(game);
    t.hop_letters.clear();
    let (layer, no_hop) = (t.layer, t.no_hop);
    if let Some(old) = game.g.child(layer, 2) {
        game.g.remove(old);
    }

    let start = title_button(game, "START", BUTTONS_X, BUTTONS_Y, start_pressed);
    // Ancré dans le « A » de START, pour l'agrandissement.
    let (ax, ay) = (0.2797, 0.50);
    let size = game.g.get(start).map_or((0.0, 0.0), |n| n.size);
    if let Some(n) = game.g.get_mut(start) {
        n.anchor = (ax, ay);
        n.tag = 1;
        n.pos = (size.0 * ax + BUTTONS_X, ay * size.1 + BUTTONS_Y);
    }
    let gallery = title_button(game, "GALLERY", 30.0, BUTTONS_Y - 40.0, gallery_pressed);
    let option = title_button(game, "OPTION", 30.0, BUTTONS_Y - 80.0, option_pressed);
    let sound_on = game.audio.music_volume() + game.audio.effects_volume() != 0.0;
    title(game).is_sound_on = sound_on;
    let sound = title_button(game, if sound_on { "SOUND:ON" } else { "SOUND:OFF" }, 30.0, BUTTONS_Y - 120.0, sound_pressed);

    let title_label = impact_label(game, "NANACA}CRASH!!", 6);
    alias_font(game, "impact48t.fnt");
    let tsize = game.g.get(title_label).map_or((0.0, 0.0), |n| n.size);
    if let Some(n) = game.g.get_mut(title_label) {
        n.pos = (tsize.0 / 2.0 + 5.0, h - 40.0);
    }
    game.g.attach(layer, title_label, 100, TITLE_TAG);

    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
    }
    for (item, tag) in [(sound, 4), (start, 1), (gallery, 2), (option, 3)] {
        game.g.attach(menu, item, 0, tag);
    }
    game.g.attach(layer, menu, 99, MENU_TAG);

    let record_label = game.g.bm_label(&assets, "BEST RECORD:", "arialblack24short.fnt", 0);
    alias_font(game, "arialblack24short.fnt");
    game.g.set_color(record_label, [0xFF, 0, 0]);
    if let Some(n) = game.g.get_mut(record_label) {
        n.anchor = (0.0, 0.5);
        n.pos = (40.0, 235.0);
    }
    game.g.attach(layer, record_label, 98, TITLE_TAG + 1);
    let rsize = game.g.get(record_label).map_or((0.0, 0.0), |n| n.size);
    let record = game.save.float("best_record", 0.0);
    let record_value = game.g.bm_label(&assets, &format!("{record:8.2}m"), "arialblack24short.fnt", 0);
    if let Some(n) = game.g.get_mut(record_value) {
        n.anchor = (0.0, 0.5);
        n.pos = (40.0 + rsize.0, 235.0);
    }
    game.g.set_color(record_value, [0, 0, 0]);
    game.g.attach(layer, record_value, 98, TITLE_TAG + 2);

    let nanaca = game.g.sprite_file(&assets, ["nanaca.png", "nanaca_summer.png", "nanaca_firework.png"][skin.clamp(0, 2) as usize]);
    if let Some(n) = game.g.get_mut(nanaca) {
        n.pos = (w + 200.0, h / 2.0);
    }
    game.g.attach(layer, nanaca, 10, 10);
    let xpos = 250f32.min(w - 320.0 + 50.0);

    title(game).hop_finished = false;
    if !no_hop {
        for item in [start, gallery, option, sound] {
            set_label_hop(game, item, true, 0.0, 0.0);
        }
        set_label_hop(game, title_label, false, 0.0, 0.0);
        set_label_hop(game, record_label, false, -50.0, -30.0);
        set_label_hop(game, record_value, false, -50.0 - rsize.0, -30.0);
        let slide = Action::move_by(0.2, (-xpos, 0.0));
        let call = Action::call(|game: &mut Game, _| {
            game.play_effect("seogg/se01.ogg");
        });
        game.run(nanaca, Action::sequence(vec![slide, call]));
        update_label_hop(game);
    } else {
        if let Some(n) = game.g.get_mut(nanaca) {
            n.pos.0 = w + 200.0 - xpos;
        }
        update_label_hop(game);
        enable_menus(game);
        title_shown(game, 0);
    }

    let background = ["topbg.jpg", "topbg_summer.jpg", "topbg_firework.jpg"][skin.clamp(0, 2) as usize];
    let topbg = game.g.sprite_file(&assets, background);
    if !no_hop {
        // Le fond se déroule depuis le milieu, de haut en bas.
        let bg = progress_bar(game, topbg);
        if let Some(n) = game.g.get_mut(bg) {
            n.pos = (w / 2.0, h / 2.0);
        }
        game.g.attach(layer, bg, 0, -1);
        let call = Action::call(|game: &mut Game, _| text_lightings(game, false));
        game.run(bg, Action::sequence(vec![Action::progress_to(0.8, 100.0), call]));
    } else {
        if let Some(n) = game.g.get_mut(topbg) {
            n.pos = (w / 2.0, h / 2.0);
        }
        game.g.attach(layer, topbg, 0, -1);
        text_lightings(game, true);
    }
}

/// Une barre de progression verticale (CCProgressTimer, barre, taux
/// (0, 1)) faite du sprite donné (détruit).
fn progress_bar(game: &mut Game, sprite: NodeId) -> NodeId {
    let (kind, size, color, opacity) = game.g.get(sprite).map_or((Kind::Node, (0.0, 0.0), [255; 3], 255), |n| (n.kind.clone(), n.size, n.color, n.opacity));
    game.g.remove(sprite);
    let Kind::Sprite(s) = kind else { return game.g.node() };
    let mut n = Node::new(Kind::Progress(Progress { sprite: s, percent: 0.0, midpoint: (0.5, 0.5), bar_rate: (0.0, 1.0) }));
    n.size = size;
    n.anchor = (0.5, 0.5);
    n.color = color;
    n.opacity = opacity;
    n.displayed_color = color;
    n.displayed_opacity = opacity;
    game.g.add(n)
}

/// showTextLightings : le reflet, les néons derrière les boutons, le cadre
/// du record.
fn text_lightings(game: &mut Game, no_hop: bool) {
    let (w, assets) = (game.design.0, game.assets.clone());
    let layer = title(game).layer;
    let shine = game.g.sprite_frame(&assets, "hshine.png");
    if let Some(n) = game.g.get_mut(shine) {
        n.scale = (12.0, 0.0);
        n.pos = (w / 2.0, 280.0);
    }
    let pulse = Action::sequence(vec![Action::scale_to_xy(1.0, 12.0, 2.0), Action::scale_to_xy(1.0, 12.0, 1.0)]);
    game.run(shine, Action::forever(pulse));
    game.g.attach(layer, shine, 0, -1);
    let xpos = [110.0, 150.0, 125.0, 200.0];
    let wide = [190.0, 270.0, 220.0, 370.0];
    for i in 0..4 {
        let neon = scale9(game, "neon.png", (wide[i], 40.0));
        if let Some(n) = game.g.get_mut(neon) {
            n.pos = (xpos[i], 202.0 - 40.0 * i as f32);
            n.set_scale(if no_hop { 1.0 } else { 0.0 });
        }
        game.g.attach(layer, neon, 0, -1);
        game.run(neon, Action::scale_to(0.2, 1.0));
    }
    let frame = scale9(game, "insetframe.png", (270.0, 20.0));
    if let Some(n) = game.g.get_mut(frame) {
        n.anchor = (0.0, 0.5);
        n.pos = (30.0, 233.0);
    }
    game.g.attach(layer, frame, 0, -1);
}

/// CCScale9Sprite::createWithSpriteFrameName, à la taille donnée.
fn scale9(game: &mut Game, name: &str, size: (f32, f32)) -> NodeId {
    let assets = game.assets.clone();
    game.g.scale9(&assets, name, None, size)
}

/// setLabelHop : chaque lettre part d'en bas à droite et arrive en
/// sautant.
fn set_label_hop(game: &mut Game, item: NodeId, is_menu_item: bool, xoffset: f32, yoffset: f32) {
    let w = game.design.0;
    let (label, mx, my) = if is_menu_item {
        let label = game.g.get(item).and_then(|n| n.children.first().copied()).unwrap_or(item);
        (label, 0.0, game.g.get(item).map_or(0.0, |n| n.pos.1))
    } else {
        (item, 0.0, game.g.get(item).map_or(0.0, |n| n.pos.1))
    };
    let (text, scale) = match game.g.get(label) {
        Some(Node { kind: Kind::Label(l), scale, .. }) => (l.text.clone(), scale.0),
        _ => return,
    };
    for i in 0..text.len() as i32 {
        let Some(id) = game.g.child(label, i) else { continue };
        let Some(n) = game.g.get(id) else { continue };
        let (px, py, orig_scale) = (n.pos.0, n.pos.1, n.scale.0);
        let rot = game.rng.random() * 20.0 + 5.0;
        let startx = (w - 150.0 + px / 4.0 - mx) / scale + xoffset;
        let starty = (80.0 - my) / scale + yoffset;
        let frames = 40.0;
        let peak = game.rng.random() * ((0.0 + starty) / 2.0 + 200.0);
        title(game).hop_letters.push(HopLetter { id, px, py, orig_scale, rot, startx, starty, offset: 10, frames, count: frames, a: -peak / (frames * frames) * 4.0, v0: peak / frames * 4.0 });
    }
}

/// updateLabelHop : faux quand les lettres sont arrivées.
fn update_label_hop(game: &mut Game) -> bool {
    let Logic::Title(t) = &mut game.logic else { return false };
    for l in &mut t.hop_letters {
        if l.count < 0.0 {
            return false;
        }
        let x = (l.startx - 0.0 - l.px) * l.count / l.frames + l.px;
        let y = l.a * l.count * l.count + l.v0 * l.count + (l.starty - l.py) * l.count / l.frames + l.py;
        if let Some(n) = game.g.get_mut(l.id) {
            n.pos = (x, y);
            n.set_rotation(l.rot * l.count);
            n.set_scale(l.orig_scale * (1.0 - 50.0 / 100.0 * l.count / l.frames));
        }
        if l.offset <= 0 {
            l.count -= 1.0;
        } else {
            l.offset -= 1;
        }
    }
    true
}

fn hop_update(game: &mut Game) {
    if title(game).hop_finished {
        return;
    }
    if !update_label_hop(game) && !title(game).hop_finished {
        title(game).hop_finished = true;
        enable_menus(game);
        title_shown(game, 1);
    }
}

fn enable_menus(game: &mut Game) {
    title(game).hop_finished = true;
    let layer = title(game).layer;
    if let Some(menu) = game.g.child(layer, MENU_TAG) {
        game.set_menu_enabled(menu, true);
    }
}

/// SceneTitle::onTitleShown : la musique, et à l'arrivée animée une voix
/// d'appel au hasard.
fn title_shown(game: &mut Game, param: i32) {
    if !game.audio.music_playing() {
        game.play_music("bgmogg/bgm0.ogg", true, 6.1);
    }
    if param == 1 {
        let n = (game.rng.lrand48() as f32 * 4.656_613e-7) as i32 % 7 + 1;
        game.apply_volumes();
        if game.effect_volume() > 0.0 {
            game.play_effect(&format!("spvoiceogg/titlecall{n:02}.ogg"));
        }
    }
}

// --- Les boutons -------------------------------------------------------------------

fn menu_item(game: &mut Game, tag: i32) -> Option<(NodeId, NodeId)> {
    let layer = title(game).layer;
    let menu = game.g.child(layer, MENU_TAG)?;
    Some((menu, game.g.child(menu, tag)?))
}

fn start_pressed(game: &mut Game) {
    game.audio.stop_music();
    game.play_effect("seogg/se03.ogg");
    let layer = title(game).layer;
    let Some((menu, item)) = menu_item(game, 1) else { return };
    game.set_menu_enabled(menu, false);
    game.g.reorder(menu, 110);
    let _ = layer;
    let zoom = Action::ease(Action::scale_to(0.666, 70.0), Ease::In(4.0));
    let call = Action::call(|game: &mut Game, _| {
        // replaceSceneFromNumber(0) : le tutoriel à la première partie.
        let next = if game.save.bool("is_first_game", true) {
            game.save.set_bool("is_first_game", false);
            SceneKind::Tutorial(false)
        } else {
            SceneKind::Game
        };
        game.replace_scene(next);
    });
    game.run(item, Action::sequence(vec![zoom, call]));
}

fn gallery_pressed(game: &mut Game) {
    scene_change_animation(game, 2, SceneKind::Gallery);
}

fn option_pressed(game: &mut Game) {
    scene_change_animation(game, 3, SceneKind::Option);
}

/// runSceneChangeAnimation : le bouton monte en haut à gauche, les autres
/// disparaissent, un voile blanc se déroule, puis la scène change.
fn scene_change_animation(game: &mut Game, menu_id: i32, next: SceneKind) {
    let (w, h) = game.design;
    let layer = title(game).layer;
    let Some((menu, item)) = menu_item(game, menu_id) else { return };
    game.set_menu_enabled(menu, false);
    game.g.reorder(menu, 110);
    let size = game.g.get(item).map_or((0.0, 0.0), |n| n.size);
    game.run(item, Action::move_to(0.5, (size.0 / 2.0 + 10.0, h - 20.0)));
    let other = if menu_id == 3 { 2 } else { 3 };
    for tag in [1, other, 4] {
        if let Some(i) = game.g.child(menu, tag)
            && let Some(n) = game.g.get_mut(i)
        {
            n.visible = false;
        }
    }
    let mut white = Sprite::empty();
    white.rect = [0.0, 0.0, w * crate::assets::CONTENT_SCALE, h * crate::assets::CONTENT_SCALE];
    let mut s = Node::new(Kind::Sprite(white));
    s.size = (w, h);
    let sprite = game.g.add(s);
    let veil = progress_bar(game, sprite);
    if let Some(n) = game.g.get_mut(veil) {
        n.pos = (w / 2.0, h / 2.0);
    }
    game.g.attach(layer, veil, 109, -1);
    // replaceSceneFromNumber : options et galerie en fondu enchaîné.
    let call = Action::call(move |game: &mut Game, _| game.replace_scene_with(next, Transition::CrossFade(0.6)));
    game.run(veil, Action::sequence(vec![Action::progress_to(0.5, 100.0), call]));
}

/// SOUND:ON / SOUND:OFF : les deux volumes à 0 ou 0,5 ; l'ancien mot tombe.
fn sound_pressed(game: &mut Game) {
    let layer = title(game).layer;
    let Some((_, item)) = menu_item(game, 4) else { return };
    let on = !title(game).is_sound_on;
    title(game).is_sound_on = on;
    let volume = if on { 0.5 } else { 0.0 };
    game.save.set_float("effect_volume", volume);
    game.save.set_float("sound_volume", volume);
    game.apply_volumes();
    let (text, old) = if on { ("SOUND:ON", "SOUND:OFF") } else { ("SOUND:OFF", "SOUND:ON") };
    if on {
        game.play_effect("seogg/se01.ogg");
    }
    game.set_item_text(item, text);
    let size = game.g.get(item).map_or((0.0, 0.0), |n| n.size);
    if let Some(n) = game.g.get_mut(item) {
        n.anchor = (0.5, 0.5);
        n.pos = (size.0 / 2.0 + 30.0, BUTTONS_Y - 120.0 + size.1 / 2.0);
    }
    let assets = game.assets.clone();
    let falling = game.g.bm_label(&assets, old, "arialblack64short.fnt", 2);
    let fsize = game.g.get(falling).map_or((0.0, 0.0), |n| n.size);
    game.g.set_color(falling, [0x10, 0x10, 0x10]);
    game.g.set_opacity(falling, 0);
    if let Some(n) = game.g.get_mut(falling) {
        n.pos = (fsize.0 / 2.0 + 30.0, BUTTONS_Y - 120.0 + fsize.1 / 2.0);
    }
    if let Some(prev) = game.g.child(layer, SOUND_HOP_TAG + 1000) {
        game.g.remove(prev);
    }
    game.g.attach(layer, falling, 1000, SOUND_HOP_TAG + 1000);
    // Les lettres après le deux-points (position 1 du script) tombent.
    let colon = old.find(':').map_or(0, |p| p + 1) as i32;
    let count = game.g.get(falling).map_or(0, |n| n.children.len()) as i32;
    for i in 0..count {
        if i < colon {
            continue;
        }
        let Some(letter) = game.g.child(falling, i) else { continue };
        let x = game.g.get(letter).map_or(0.0, |n| n.pos.0);
        game.g.set_opacity(letter, 255);
        let spin = game.rng.random() * 40.0;
        let fall = Action::spawn(vec![Action::move_by(0.666, (x + 30.0, -100.0)), Action::rotate_by(0.666, spin), Action::scale_to(0.666, 1.25)]);
        game.run(letter, fall);
    }
}
