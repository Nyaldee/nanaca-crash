//! L'écran des options (SceneOption et SceneOption.lua) : décor animé du
//! skin, volumes (curseurs), choix du skin (liste défilante), compteur de
//! vitesse (liste de choix), crédits, tutoriel. La boutique et
//! l'enregistrement vidéo (Kamcord) sont retirés.

use std::rc::Rc;

use crate::action::Action;
use crate::data::xml;
use crate::node::{Kind, NodeId};
use crate::text::TextStyle;

use super::{Game, Logic, SceneKind, Transition, backdrop, widgets};

/// Étiquettes d'origine.
const MENU: i32 = 0x9990;
const SPEEDOMETER: i32 = 0x7d5;
const CREDITS: i32 = 0x10000;
const TOAST: i32 = 10000;
/// Largeur d'une case de la liste des skins.
const CELL: f32 = 170.0;

/// Un curseur de volume (CCHackedControlSlider).
struct Slider {
    node: NodeId,
    progress: NodeId,
    thumb: NodeId,
    /// Largeur de la piste et taille du contenu.
    track: f32,
    size: (f32, f32),
    thumb_width: f32,
    progress_px: (f32, f32),
    value: f32,
    enabled: bool,
    selected: bool,
    tag: i32,
}

/// Ce que suit le toucher en cours.
#[derive(Clone, Copy)]
enum Grab {
    Slider(usize),
    Skin(usize),
    Scroll { last: (f32, f32), moved: bool },
}

pub struct Options {
    layer: NodeId,
    menu: NodeId,
    speed_item: NodeId,
    sliders: Vec<Slider>,
    /// La liste des skins : sa fenêtre, son contenu, ses images.
    view: NodeId,
    view_size: (f32, f32),
    container: NodeId,
    cells: Vec<NodeId>,
    buttons: Vec<String>,
    /// Défilement en cours (vitesse d'élan) et décélération.
    scroll_distance: f32,
    decelerating: bool,
    demo: Option<backdrop::Demo>,
    /// Avancement du décor (0x114).
    scroll: f32,
    selected: i32,
    loading: bool,
    back_enabled: bool,
    labels: Vec<NodeId>,
    credits: NodeId,
    grab: Option<Grab>,
}

fn options(game: &mut Game) -> &mut Options {
    match &mut game.logic {
        Logic::Option(o) => o,
        _ => unreachable!("les options ne sont pas la scène en cours"),
    }
}

/// La couleur des textes du skin.
fn font_color(game: &mut Game) -> [u8; 3] {
    if game.skin_key() == "firework_show" { [0xFF; 3] } else { [0; 3] }
}

/// Les images des boutons des skins (skin.xml), par numéro.
fn skin_buttons(game: &Game) -> Vec<String> {
    let Some(root) = game.assets.apk.text("skin.xml").and_then(|t| xml::parse(&t)) else { return Vec::new() };
    let mut skins: Vec<(i32, String)> = root
        .children_named("skin")
        .map(|s| {
            let text = |name: &str| s.child(name).map_or(String::new(), |e| e.text.trim().to_string());
            (text("id").parse().unwrap_or(0), text("button"))
        })
        .collect();
    skins.sort_by_key(|s| s.0);
    skins.into_iter().map(|s| s.1).collect()
}

/// SceneOption::onEnter (onEnter du script, setup, chargement).
pub fn enter(game: &mut Game, root: NodeId) {
    let (w, h) = game.design;
    let layer = game.g.layer(game.design);
    game.add(root, layer, 0, 0x1234);
    let fake = game.g.node();
    let selected = game.skin();
    game.logic = Logic::Option(Box::new(Options {
        layer,
        menu: fake,
        speed_item: fake,
        sliders: Vec::new(),
        view: fake,
        view_size: (w - 20.0, 100.0),
        container: fake,
        cells: Vec::new(),
        buttons: skin_buttons(game),
        scroll_distance: 0.0,
        decelerating: false,
        demo: None,
        scroll: 0.0,
        selected,
        loading: false,
        back_enabled: false,
        labels: Vec::new(),
        credits: fake,
        grab: None,
    }));
    // La touche retour n'est active qu'après 0.5 s.
    game.run(layer, Action::sequence(vec![Action::delay(0.5), Action::call(|game: &mut Game, _| options(game).back_enabled = true)]));
    let (e, s) = (game.effect_volume(), game.sound_volume());
    game.audio.set_effects_volume(e);
    game.audio.set_music_volume(s);
    // SceneOption.lua : le titre OPTION.
    let title = game.bm_label("OPTION", "arialblack64short.fnt", 0);
    if let Some(f) = game.assets.font("arialblack64short.fnt") {
        game.assets.set_alias(&f.page);
    }
    let size = game.size(title);
    game.set_color(title, [0, 0, 0]);
    game.set_pos(title, size.0 / 2.0 + 10.0, h - 20.0);
    game.add(layer, title, 0, -1);
    setup(game);
    game.loader_start();
}

/// SceneOption::setup.
fn setup(game: &mut Game) {
    let (w, h) = game.design;
    let layer = options(game).layer;
    let white = game.g.color_layer([0xFF; 4], (w, h));
    if let Some(n) = game.g.get_mut(white) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.add(layer, white, -1, 10);
    // Les boutons : retour, crédits (« i »), tutoriel, compteur de vitesse.
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let action = |id: i32| -> crate::action::Callback { Rc::new(move |game: &mut Game, item| menu_test(game, id, item)) };
    let back = widgets::pause_item(game, "BACK", (100.0, 44.0), 1, action(1));
    game.set_pos(back, 5.0 + 50.0, 5.0 + 22.0);
    let info = widgets::pause_item(game, "i", (50.0, 44.0), 2, action(2));
    game.set_pos(info, (w - 25.0) - 5.0, 5.0 + 22.0);
    let tutorial = widgets::pause_item(game, "TUTORIAL", (140.0, 44.0), 3, action(3));
    game.set_pos(tutorial, (w - 70.0) - 60.0, 5.0 + 22.0);
    widgets::set_pause_text_scale(game, tutorial, 0.8);
    let state = game.save.int("speedmeter_state", 0);
    let speed_text = game.text(&format!("speed_meter_st{state}"));
    let speed = widgets::thin_item(game, &speed_text, (150.0, 40.0), SPEEDOMETER, action(SPEEDOMETER));
    game.set_opacity(speed, 0xAA);
    game.set_pos(speed, 75.0 + 20.0, 80.0);
    for (item, tag) in [(back, 1), (info, 2), (tutorial, 3), (speed, SPEEDOMETER)] {
        game.add(menu, item, 0, tag);
    }
    game.add(layer, menu, 999, MENU);
    options(game).menu = menu;
    options(game).speed_item = speed;
    // Les intitulés, à la couleur du skin.
    let color = font_color(game);
    let mut labels = Vec::new();
    for (key, size, x, y) in [("SOUND VOLUME", 16.0, w - 280.0, 135.0), ("EFFECT VOLUME", 16.0, w - 280.0, 80.0), ("SELECT SKIN", 18.0, 20.0, h - 75.0), ("SPEEDOMETER", 16.0, 20.0, 90.0)] {
        let text = game.text(key);
        let label = game.g.ttf_label(&text, &TextStyle { font: "Arial", size, dimensions: (w, 30.0), h_align: 0, v_align: 0 });
        game.set_color(label, color);
        game.set_anchor(label, 0.0, 0.0);
        game.set_pos(label, x, y);
        game.add(layer, label, 0, -1);
        labels.push(label);
    }
    options(game).labels = labels;
    // La liste des skins.
    let view_size = options(game).view_size;
    let stencil = game.g.color_layer([0xFF; 4], view_size);
    if let Some(n) = game.g.get_mut(stencil) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    let mut clip = crate::node::Node::new(Kind::Clip { stencil: Some(stencil), inverted: false });
    clip.size = view_size;
    let view = game.g.add(clip);
    game.set_pos(view, 10.0, h - 135.0);
    let container = game.g.node();
    game.add(view, container, 0, -1);
    game.add(layer, view, 0, 500);
    options(game).view = view;
    options(game).container = container;
    reload_cells(game);
    let selected = options(game).selected;
    set_offset_animated(game, selected as f32 * -85.0);
    // Les curseurs : musique (1) et effets (2).
    let (music, effects) = (game.sound_volume(), game.effect_volume());
    let s1 = slider(game, 1, music, (w - 155.0, 150.0));
    let s2 = slider(game, 2, effects, (w - 155.0, 95.0));
    options(game).sliders = vec![s1, s2];
    // Le décor du skin, sous tout le reste.
    change_background(game);
    setup_credits(game);
    if !game.audio.music_playing() {
        game.play_music("bgmogg/bgm0.ogg", true, 6.1);
    }
    game.play_effect("spvoiceogg/miki_option.ogg");
    add_toast(game);
    // Rien ne répond avant la fin du chargement.
    game.set_menu_enabled(menu, false);
    set_sliders_enabled(game, false);
    options(game).loading = true;
}

/// Le toast LOADING au centre.
fn add_toast(game: &mut Game) {
    let (w, h) = game.design;
    let layer = options(game).layer;
    let text = game.text("LOADING");
    let toast = widgets::toast(game, (190.0, 40.0), &text);
    game.set_pos(toast, w * 0.5, h * 0.5);
    game.add(layer, toast, TOAST, TOAST);
}

/// setupCreditsOverLay : les crédits sur un voile noir, et CLOSE.
fn setup_credits(game: &mut Game) {
    let (w, h) = game.design;
    let layer = options(game).layer;
    let overlay = game.g.color_layer([0, 0, 0, 0xCC], (w, h));
    if let Some(n) = game.g.get_mut(overlay) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.add(layer, overlay, CREDITS, CREDITS);
    for (key, size, x) in [("Credits0", 14.0, (w - 480.0) + 20.0), ("Credits1", 12.0, (w - 480.0) + 260.0)] {
        let text: String = game.text(key).lines().map(str::trim).collect::<Vec<_>>().join("\n");
        let label = game.g.ttf_label(&text, &TextStyle { font: "monospace", size, dimensions: (220.0, h), h_align: 0, v_align: 0 });
        game.set_color(label, [0xFF; 3]);
        game.set_anchor(label, 0.0, 0.0);
        game.set_pos(label, x, -30.0);
        game.add(overlay, label, 0, -1);
    }
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let close = widgets::pause_item(game, "CLOSE", (140.0, 38.0), 100, Rc::new(|game: &mut Game, item| menu_test(game, 100, item)));
    widgets::set_pause_text_scale(game, close, 0.8);
    game.set_pos(close, w * 0.5, 35.0);
    game.add(menu, close, 0, 100);
    game.add(overlay, menu, 0x9991, 0x9991);
    game.set_visible(overlay, false);
    options(game).credits = overlay;
}

/// SceneOption::menuTest : les volumes sont enregistrés, puis l'action
/// du bouton.
fn menu_test(game: &mut Game, id: i32, _item: NodeId) {
    let (e, s) = (game.audio.effects_volume(), game.audio.music_volume());
    game.save.set_float("effect_volume", e);
    game.save.set_float("sound_volume", s);
    let menu = options(game).menu;
    match id {
        1 => game.replace_scene_with(SceneKind::Title(1), Transition::CrossFade(0.6)),
        2 => {
            game.play_effect("seogg/se01.ogg");
            let credits = options(game).credits;
            game.set_menu_enabled(menu, false);
            game.set_visible(credits, true);
            set_sliders_enabled(game, false);
        }
        3 => {
            game.play_effect("seogg/se01.ogg");
            game.replace_scene_with(SceneKind::Tutorial(true), Transition::CrossFade(0.6));
        }
        100 => {
            let credits = options(game).credits;
            game.set_visible(credits, false);
            game.set_menu_enabled(menu, true);
            set_sliders_enabled(game, true);
        }
        SPEEDOMETER => {
            let choices: Vec<(i32, String)> = (0..3).map(|i| (i, game.text(&format!("speed_meter_st{i}")))).collect();
            let desc = game.text("SPEED_METER_DESC");
            let dialog = widgets::select_list(game, SPEEDOMETER, "SPEEDOMETER", &desc, &choices, Rc::new(list_closed));
            let layer = options(game).layer;
            game.add(layer, dialog, 0x2712, 0x2712);
        }
        _ => {}
    }
}

/// cbListViewClosed (MSG_SELECTLIST_SELECTED « liste:choix »).
fn list_closed(game: &mut Game, tag: i32, index: i32) {
    if tag != SPEEDOMETER {
        return;
    }
    let text = game.text(&format!("speed_meter_st{index}"));
    let item = options(game).speed_item;
    widgets::set_thin_text(game, item, &text);
    game.save.set_int("speedmeter_state", index);
}

/// La touche retour (keyBackClicked) : le titre, sans enregistrer les
/// volumes (comme l'original).
pub fn back(game: &mut Game) {
    if options(game).back_enabled {
        game.replace_scene_with(SceneKind::Title(1), Transition::CrossFade(0.6));
    }
}

/// onPreload : chargement fini, tout répond.
pub fn on_preload(game: &mut Game, progress: f32) {
    if progress < 1.0 {
        return;
    }
    let (layer, menu) = (options(game).layer, options(game).menu);
    set_sliders_enabled(game, true);
    game.set_menu_enabled(menu, true);
    if let Some(t) = game.child(layer, TOAST) {
        game.g.remove(t);
    }
    options(game).loading = false;
}

/// SceneOption::update : le décor défile (10 par seconde).
pub fn update(game: &mut Game, dt: f32) {
    let scroll = options(game).scroll;
    if let Some(demo) = options(game).demo.take() {
        backdrop::move_cast(game, &demo, scroll);
        options(game).demo = Some(demo);
    }
    let o = options(game);
    backdrop::advance(&mut o.scroll, dt);
    if o.decelerating {
        decelerate(game);
    }
}

// --- La liste des skins (CCTableView) -----------------------------------------------------

/// Les cases : un bouton par skin, plein s'il est choisi, grisé sinon.
fn reload_cells(game: &mut Game) {
    let container = options(game).container;
    for c in std::mem::take(&mut options(game).cells) {
        if let Some(cell) = game.g.get(c).and_then(|n| n.parent).and_then(|p| game.g.get(p)).and_then(|n| n.parent) {
            game.g.remove(cell);
        }
    }
    let (buttons, selected) = (options(game).buttons.clone(), options(game).selected);
    let mut cells = Vec::new();
    for (k, name) in buttons.iter().enumerate() {
        let cell = game.g.node();
        game.set_pos(cell, k as f32 * CELL, 0.0);
        let back = game.g.color_layer([0, 0, 0, 0], (150.0, 70.0));
        if let Some(n) = game.g.get_mut(back) {
            n.anchor = (0.0, 0.0);
            n.ignore_anchor = true;
        }
        game.set_pos(back, 10.0, 0.0);
        game.add(cell, back, 0, 100);
        let sprite = game.sprite(name);
        game.set_pos(sprite, 75.0, 35.0);
        game.add(back, sprite, 100, 100);
        if k as i32 == selected {
            game.set_opacity(sprite, 0xFF);
            game.set_color(sprite, [0xFF; 3]);
        } else {
            game.set_opacity(sprite, 0x7F);
            game.set_color(sprite, [0x7F; 3]);
        }
        game.add(container, cell, 0, k as i32);
        cells.push(sprite);
    }
    options(game).cells = cells;
}

/// Le défilement (setContentOffset) : la position du contenu.
fn set_offset(game: &mut Game, x: f32) {
    let container = options(game).container;
    game.stop_actions(container);
    game.set_x(container, x);
}

/// setContentOffset(…, animé) : 0.15 s.
fn set_offset_animated(game: &mut Game, x: f32) {
    let container = options(game).container;
    game.stop_actions(container);
    game.run(container, Action::move_to(0.15, (x, 0.0)));
}

/// Les bornes du défilement : du plus à gauche au plus à droite.
fn offset_bounds(game: &mut Game) -> (f32, f32) {
    let o = options(game);
    let content = o.buttons.len() as f32 * CELL;
    (o.view_size.0 - content, 0.0)
}

/// relocateContainer(animé) : remis dans ses bornes.
fn relocate(game: &mut Game) {
    let (min, max) = offset_bounds(game);
    let container = options(game).container;
    let x = game.pos(container).0;
    let to = x.max(min).min(max);
    if to != x {
        set_offset_animated(game, to);
    }
}

/// deaccelerateScrolling : l'élan s'amortit (×0.95 par image).
fn decelerate(game: &mut Game) {
    let (min, max) = offset_bounds(game);
    let inset = options(game).view_size.0 * 0.2;
    let (max_inset, min_inset) = (max + inset, min - inset);
    let container = options(game).container;
    let x = game.pos(container).0 + options(game).scroll_distance;
    let new_x = x.min(max_inset).max(min_inset);
    let o = options(game);
    o.scroll_distance = (o.scroll_distance - (new_x - x)) * 0.95;
    let distance = o.scroll_distance;
    set_offset(game, new_x);
    if distance.abs() <= 1.0 || new_x >= max_inset || new_x <= min_inset {
        options(game).decelerating = false;
        relocate(game);
    }
}

/// tableCellSpriteTapped : un autre skin est choisi et chargé.
fn skin_tapped(game: &mut Game, index: usize) {
    let index = index as i32;
    if options(game).selected == index || options(game).loading {
        return;
    }
    game.save.set_int("skin_selected", index);
    options(game).selected = index;
    change_background(game);
    game.play_effect("seogg/se14.ogg");
    add_toast(game);
    let menu = options(game).menu;
    set_sliders_enabled(game, false);
    game.set_menu_enabled(menu, false);
    game.loader_reload();
    let color = font_color(game);
    for label in options(game).labels.clone() {
        game.set_color(label, color);
    }
    reload_cells(game);
    set_offset_animated(game, index as f32 * -85.0);
    options(game).loading = true;
}

// --- Les curseurs -------------------------------------------------------------------------

/// CCControlSlider::initWithSprites : piste, remplissage, poignée.
fn slider(game: &mut Game, tag: i32, value: f32, pos: (f32, f32)) -> Slider {
    let layer = options(game).layer;
    let track = game.sprite_file("sliderTrack.png");
    let progress = game.sprite_file("sliderProgress.png");
    let thumb = game.sprite_file("sliderThumb.png");
    let (ts, ps, hs) = (game.size(track), game.size(progress), game.size(thumb));
    let size = (ts.0.max(hs.0), ts.1.max(hs.1));
    let node = game.g.node();
    if let Some(n) = game.g.get_mut(node) {
        n.size = size;
        n.anchor = (0.5, 1.0);
    }
    game.set_pos(track, size.0 / 2.0, size.1 / 2.0);
    game.set_anchor(progress, 0.0, 0.5);
    game.set_pos(progress, 0.0, size.1 / 2.0);
    game.set_pos(thumb, 0.0, size.1 / 2.0);
    game.add(node, track, 0, -1);
    game.add(node, progress, 0, -1);
    game.add(node, thumb, 0, -1);
    game.set_pos(node, pos.0, pos.1);
    game.add(layer, node, 0, tag);
    let mut s = Slider {
        node,
        progress,
        thumb,
        track: ts.0,
        size,
        thumb_width: hs.0,
        progress_px: (ps.0 * crate::assets::CONTENT_SCALE, ps.1 * crate::assets::CONTENT_SCALE),
        value,
        enabled: true,
        selected: false,
        tag,
    };
    layout(game, &mut s);
    s
}

/// needsLayout : la poignée à sa place, le remplissage jusqu'à elle.
fn layout(game: &mut Game, s: &mut Slider) {
    let x = s.value.clamp(0.0, 1.0) * s.track;
    game.set_x(s.thumb, x);
    if let Some(n) = game.g.get_mut(s.progress)
        && let Kind::Sprite(sp) = &mut n.kind
    {
        sp.rect[2] = (x * crate::assets::CONTENT_SCALE).min(s.progress_px.0);
        n.size.0 = x;
    }
}

fn set_sliders_enabled(game: &mut Game, on: bool) {
    for k in 0..options(game).sliders.len() {
        let thumb = options(game).sliders[k].thumb;
        options(game).sliders[k].enabled = on;
        game.set_opacity(thumb, if on { 0xFF } else { 0x80 });
    }
}

/// Le point du toucher dans le curseur (locationFromTouch : borné à la piste).
fn slider_location(game: &mut Game, k: usize, point: (f32, f32)) -> f32 {
    let s = &options(game).sliders[k];
    let (node, track) = (s.node, s.track);
    let (x, _) = game.g.world(node).invert().apply(point.0, point.1);
    x.clamp(0.0, track)
}

/// setValue, puis ValueChanged (0x100) ou la fin du toucher (0x10).
fn slider_value(game: &mut Game, k: usize, value: f32) {
    let mut s = options(game).sliders.remove(k);
    s.value = value.clamp(0.0, 1.0);
    layout(game, &mut s);
    let (tag, value) = (s.tag, s.value);
    options(game).sliders.insert(k, s);
    slider_changed(game, tag, value, 0x100);
}

/// SceneOption::sliderChanged : la musique suit le curseur, les effets
/// changent au relâchement (avec la voix de Miki).
fn slider_changed(game: &mut Game, tag: i32, value: f32, event: u32) {
    if tag == 1 && event == 0x100 {
        game.audio.set_music_volume(value);
    } else if tag == 2 && event == 0x10 {
        game.audio.set_effects_volume(value);
        game.play_effect("spvoiceogg/miki_sfx.ogg");
    }
}

// --- Le toucher ---------------------------------------------------------------------------

/// Le toucher qui n'a pas pris de bouton : images des skins (−128), liste
/// (0), curseurs (1).
pub fn touch_began(game: &mut Game, point: (f32, f32)) -> bool {
    for (k, sprite) in options(game).cells.clone().into_iter().enumerate() {
        if k as i32 != options(game).selected && game.g.shown(sprite) && game.g.contains(sprite, point) {
            game.run(sprite, Action::scale_to(0.2, 0.9));
            options(game).grab = Some(Grab::Skin(k));
            return true;
        }
    }
    let view = options(game).view;
    if game.g.shown(view) && game.g.contains(view, point) {
        options(game).decelerating = false;
        options(game).scroll_distance = 0.0;
        options(game).grab = Some(Grab::Scroll { last: point, moved: false });
        return true;
    }
    for k in 0..options(game).sliders.len() {
        let s = &options(game).sliders[k];
        let (node, size, thumb_width, enabled) = (s.node, s.size, s.thumb_width, s.enabled);
        if !enabled || !game.g.shown(node) {
            continue;
        }
        let (x, y) = game.g.world(node).invert().apply(point.0, point.1);
        if x >= -thumb_width / 2.0 && x < size.0 + thumb_width / 2.0 && y >= 0.0 && y < size.1 {
            options(game).sliders[k].selected = true;
            let thumb = options(game).sliders[k].thumb;
            game.set_color(thumb, [166; 3]);
            let x = slider_location(game, k, point);
            let track = options(game).sliders[k].track;
            slider_value(game, k, x / track);
            options(game).grab = Some(Grab::Slider(k));
            return true;
        }
    }
    false
}

pub fn touch_moved(game: &mut Game, point: (f32, f32)) {
    match options(game).grab {
        Some(Grab::Slider(k)) => {
            let x = slider_location(game, k, point);
            let track = options(game).sliders[k].track;
            slider_value(game, k, x / track);
        }
        Some(Grab::Scroll { last, moved }) => {
            let dx = point.0 - last.0;
            let dy = point.1 - last.1;
            // Un petit glissement ne compte pas (7/160 de pouce).
            if !moved && (dx * dx + dy * dy).sqrt() < 7.0 / 2.25 {
                return;
            }
            let dx = if moved { dx } else { 0.0 };
            options(game).grab = Some(Grab::Scroll { last: point, moved: true });
            let view = options(game).view;
            if game.g.contains(view, point) {
                options(game).scroll_distance = dx;
                let container = options(game).container;
                let x = game.pos(container).0 + dx;
                set_offset(game, x);
            }
        }
        _ => {}
    }
}

pub fn touch_ended(game: &mut Game, point: (f32, f32)) {
    match options(game).grab.take() {
        Some(Grab::Slider(k)) => {
            let s = &options(game).sliders[k];
            let (selected, thumb, track) = (s.selected, s.thumb, s.track);
            if selected {
                let x = game.pos(thumb).0;
                slider_value(game, k, x / track);
            }
            game.set_color(thumb, [0xFF; 3]);
            options(game).sliders[k].selected = false;
            // Le curseur « bidouillé » du jeu signale aussi le relâchement.
            let (tag, value) = (options(game).sliders[k].tag, options(game).sliders[k].value);
            slider_changed(game, tag, value, 0x10);
        }
        Some(Grab::Skin(k)) => {
            let sprite = options(game).cells[k];
            game.stop_actions(sprite);
            game.set_scale(sprite, 1.0);
            if game.g.contains(sprite, point) {
                game.run(sprite, Action::sequence(vec![Action::delay(0.1), Action::call(move |game: &mut Game, _| skin_tapped(game, k))]));
            }
        }
        Some(Grab::Scroll { moved, .. }) => {
            if moved {
                options(game).decelerating = true;
            }
        }
        None => {}
    }
}

// --- La manette ---------------------------------------------------------------------------

/// Ce qu'on peut choisir en plus des boutons : les skins non choisis, et
/// les curseurs actifs.
pub fn pad_targets(game: &mut Game) -> (Vec<NodeId>, Vec<NodeId>) {
    if options(game).loading {
        return (Vec::new(), Vec::new());
    }
    let o = options(game);
    let selected = o.selected;
    let cells = o.cells.iter().enumerate().filter(|&(k, _)| k as i32 != selected).map(|(_, &c)| c).collect();
    let sliders = o.sliders.iter().filter(|s| s.enabled).map(|s| s.node).collect();
    (cells, sliders)
}

/// Un skin choisi à la manette : la liste défile jusqu'à lui.
pub fn pad_focus(game: &mut Game, node: NodeId) {
    let Some(k) = options(game).cells.iter().position(|&c| c == node) else { return };
    let (min, max) = offset_bounds(game);
    set_offset_animated(game, (k as f32 * -85.0).max(min).min(max));
}

/// A sur un skin : il est choisi et chargé.
pub fn pad_activate(game: &mut Game, node: NodeId) {
    if let Some(k) = options(game).cells.iter().position(|&c| c == node) {
        skin_tapped(game, k);
    }
}

/// Gauche ou droite sur un curseur : un dixième de moins ou de plus,
/// comme un curseur relâché.
pub fn pad_slide(game: &mut Game, node: NodeId, delta: f32) {
    let Some(k) = options(game).sliders.iter().position(|s| s.node == node) else { return };
    let value = options(game).sliders[k].value + delta;
    slider_value(game, k, value);
    let (tag, value) = (options(game).sliders[k].tag, options(game).sliders[k].value);
    slider_changed(game, tag, value, 0x10);
}

// --- Le décor -----------------------------------------------------------------------------

/// cbChangeBackground : le décor du skin choisi remplace l'ancien.
fn change_background(game: &mut Game) {
    if let Some(old) = options(game).demo.take() {
        game.g.remove(old.layer);
    }
    let layer = options(game).layer;
    let demo = backdrop::create(game, layer, -1);
    options(game).demo = Some(demo);
}
