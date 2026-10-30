//! La galerie : SceneGallery (le panneau des icônes des succès, la
//! vignette et le décor animé) et les pages des éléments obtenus
//! (GalleryWallpaper, GalleryMusic, GalleryVoice, GalleryNA) avec leur
//! interface commune (gallery_common_ui) et leurs gestes (XTLayer). Les
//! boutons de Google Play Jeux sont retirés ; une image s'enregistre dans
//! le dossier Images.

use std::rc::Rc;

use crate::action::Action;
use crate::node::NodeId;
use crate::text::TextStyle;

use super::achievements::Content;
use super::{Game, Logic, SceneKind, Transition, backdrop, widgets};

/// Étiquettes d'origine.
const MENU: i32 = 0x9990;
const LABEL: i32 = 0x12d;
const THUMB: i32 = 500;
const TOAST: i32 = 0x1f5;
const BAR: i32 = 0x3e6;
const COUNTER: i32 = 999;
const ITEMS: i32 = 0x270e;
const BANNER: i32 = 5000;
/// L'action qui recache l'interface commune.
const HIDE_ACTION: i32 = 5000;
/// Le panneau : 6 × 6 icônes, 36 points d'écart.
const GRID: i32 = 6;
const CELL: f32 = 36.0;

// --- SceneGallery -------------------------------------------------------------------------

/// Ce que suit le toucher en cours.
#[derive(Clone, Copy)]
enum Grab {
    Thumb,
    Selector,
}

pub struct Gallery {
    layer: NodeId,
    menu: NodeId,
    thumb: NodeId,
    thumb_enabled: bool,
    /// IconSelector : son calque, ses icônes (par case), son curseur.
    selector: NodeId,
    icons: Vec<Option<NodeId>>,
    cursor: NodeId,
    selected: i32,
    selector_enabled: bool,
    back_enabled: bool,
    demo: Option<backdrop::Demo>,
    scroll: f32,
    grab: Option<Grab>,
}

fn gallery(game: &mut Game) -> &mut Gallery {
    match &mut game.logic {
        Logic::Gallery(g) => g,
        _ => unreachable!("la galerie n'est pas la scène en cours"),
    }
}

/// GalleryContents::valueInDirect : l'élément d'un numéro.
fn content(game: &Game, id: i32) -> Option<Content> {
    game.gallery.iter().find(|c| c.id == id).cloned()
}

/// La couleur des textes du skin.
fn font_color(game: &mut Game) -> [u8; 3] {
    if game.skin_key() == "firework_show" { [0xFF; 3] } else { [0; 3] }
}

/// SceneGallery::onEnter (SceneGallery.lua puis setup).
pub fn enter(game: &mut Game, root: NodeId) {
    let (w, h) = game.design;
    let layer = game.g.layer(game.design);
    game.add(root, layer, 0, 0x1234);
    let fake = game.g.node();
    game.logic = Logic::Gallery(Box::new(Gallery {
        layer,
        menu: fake,
        thumb: fake,
        thumb_enabled: true,
        selector: fake,
        icons: Vec::new(),
        cursor: fake,
        selected: -1,
        selector_enabled: true,
        back_enabled: false,
        demo: None,
        scroll: 0.0,
        grab: None,
    }));
    game.apply_volumes();
    game.assets.add_sprite_frames("gthumb.plist");
    // SceneGallery.lua : deux triangles qui encadrent la vignette.
    for (rotation, pos, step) in [(90.0, (232.0 + (w - 432.0) / 2.0 + 200.0, 80.0), (4.0, -4.0)), (270.0, (232.0 + (w - 432.0) / 2.0, 280.0), (-4.0, 4.0))] {
        let tri = game.sprite_file("triangle.png");
        game.set_anchor(tri, 0.0, 0.0);
        game.set_color(tri, [127; 3]);
        game.set_rotation(tri, rotation);
        game.set_pos(tri, pos.0, pos.1);
        game.run(tri, Action::forever(Action::sequence(vec![Action::move_by(0.5, step), Action::move_by(0.5, (-step.0, -step.1))])));
        game.add(layer, tri, 0, -1);
    }
    setup(game, w, h);
}

/// SceneGallery::setup.
fn setup(game: &mut Game, w: f32, h: f32) {
    let layer = gallery(game).layer;
    game.run(layer, Action::sequence(vec![Action::delay(0.5), Action::call(|game: &mut Game, _| gallery(game).back_enabled = true)]));
    let sky = game.g.gradient_layer([0x80, 0xD0, 0xFD, 0xFF], [0xFF; 4], (w, h));
    if let Some(n) = game.g.get_mut(sky) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.add(layer, sky, -1, 10);
    let title = game.bm_label("GALLERY", "arialblack64short.fnt", 0);
    let size = game.size(title);
    game.set_color(title, [0, 0, 0]);
    game.set_pos(title, 10.0 + size.0 * 0.5, h - 20.0);
    game.add(layer, title, 0, -1);
    // BACK et VIEW CONTENT.
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let action = |id: i32| -> crate::action::Callback { Rc::new(move |game: &mut Game, _| menu_test(game, id)) };
    let back = widgets::pause_item(game, "BACK", (100.0, 44.0), 1, action(1));
    game.set_pos(back, 50.0 + 5.0, 22.0 + 5.0);
    let view = widgets::pause_item(game, "VIEW CONTENT", (240.0, 38.0), 2, action(2));
    widgets::set_pause_text_scale(game, view, 0.8);
    let center = 232.0 + (w - 232.0) * 0.5;
    game.set_pos(view, center, 19.0 + 2.0 + 3.0);
    game.add(menu, back, 0, 1);
    game.add(menu, view, 0, 2);
    game.add(layer, menu, MENU, MENU);
    gallery(game).menu = menu;
    // Le titre et le texte de l'élément choisi.
    let text = game.text("Select icon on panel");
    let label = game.g.ttf_label(&text, &label_style());
    let color = font_color(game);
    game.set_color(label, color);
    game.set_anchor(label, 0.0, 0.0);
    game.set_pos(label, 232.0 + (w - 432.0) * 0.5, 20.0);
    game.add(layer, label, LABEL, LABEL);
    // La vignette (ClickableSprite).
    let thumb = game.sprite("gthumbna.png");
    game.set_pos(thumb, center, 180.0);
    game.add(layer, thumb, THUMB, THUMB);
    gallery(game).thumb = thumb;
    let toast = widgets::toast(game, (190.0, 40.0), "");
    game.set_pos(toast, center, 255.0);
    game.set_scale(toast, 0.0);
    game.add(layer, toast, TOAST, TOAST);
    // Le panneau des icônes, puis le décor.
    construct_selector(game);
    let demo = backdrop::create(game, layer, -1);
    gallery(game).demo = Some(demo);
    // Le cache des succès : les icônes obtenues apparaissent, puis la
    // dernière vue est choisie.
    if game.gallery.iter().any(|c| game.achieved(c.id)) {
        refresh(game);
    }
    let position = game.save.int("positon_gallery", -1);
    select_programmatically(game, position);
    if !game.audio.music_playing() {
        game.play_music("bgmogg/bgm1.ogg", true, 6.1);
    }
    game.play_effect("spvoiceogg/miki_gallery.ogg");
}

fn label_style() -> TextStyle<'static> {
    TextStyle { font: "Arial", size: 12.0, dimensions: (220.0, 60.0), h_align: 0, v_align: 0 }
}

/// SceneGallery::menuTest : la case choisie est retenue, puis le bouton.
fn menu_test(game: &mut Game, id: i32) {
    let selected = gallery(game).selected;
    if selected != -1 {
        game.save.set_int("positon_gallery", selected);
    }
    match id {
        1 => game.replace_scene_with(SceneKind::Title(1), Transition::CrossFade(0.6)),
        2 => {
            game.play_effect("seogg/se01.ogg");
            change_gallery_scene(game, selected);
        }
        _ => {}
    }
}

/// La touche retour (keyBackClicked) : le titre, une fois le menu actif.
pub fn back(game: &mut Game) {
    if gallery(game).back_enabled {
        game.replace_scene_with(SceneKind::Title(1), Transition::CrossFade(0.6));
    }
}

/// SceneGallery::update : le décor défile.
pub fn update(game: &mut Game, dt: f32) {
    let scroll = gallery(game).scroll;
    if let Some(demo) = gallery(game).demo.take() {
        backdrop::move_cast(game, &demo, scroll);
        gallery(game).demo = Some(demo);
    }
    backdrop::advance(&mut gallery(game).scroll, dt);
}

/// changeGalleryScene : la page de l'élément (obtenu), en fondu.
fn change_gallery_scene(game: &mut Game, id: i32) {
    if id == -1 || !game.achieved(id) {
        return;
    }
    game.replace_scene_with(SceneKind::GalleryContent(id), Transition::CrossFade(0.6));
}

// --- IconSelector -------------------------------------------------------------------------

/// IconSelector::construct : une icône par case (le fond iconxx.png sous
/// chaque élément), visible si le succès est obtenu ; le curseur, caché.
fn construct_selector(game: &mut Game) {
    let layer = gallery(game).layer;
    let selector = game.g.layer((216.0, 216.0));
    game.set_pos(selector, 20.0, 58.0);
    let batch = game.g.node();
    let mut icons = Vec::new();
    for k in 0..GRID * GRID {
        let (x, y) = (16.0 + CELL * (k % GRID) as f32, 200.0 - CELL * (k / GRID) as f32);
        let icon = match content(game, k) {
            Some(c) => {
                let back = game.sprite("iconxx.png");
                game.set_pos(back, x, y);
                game.set_scale(back, 0.5);
                game.add(batch, back, 99, -1);
                let icon = game.sprite(&c.icon);
                let earned = game.achieved(k);
                game.set_visible(icon, earned);
                icons.push(earned.then_some(icon));
                icon
            }
            None => {
                icons.push(None);
                game.sprite("iconxx.png")
            }
        };
        game.set_pos(icon, x, y);
        game.set_scale(icon, 0.5);
        game.add(batch, icon, 100, k);
    }
    let cursor = game.sprite("icon00.png");
    game.set_color(cursor, [0xFF; 3]);
    game.set_opacity(cursor, 0x7F);
    game.set_visible(cursor, false);
    game.add(batch, cursor, 0x69, 500);
    game.add(selector, batch, 0, -1);
    game.add(layer, selector, 10, 10);
    let g = gallery(game);
    g.selector = selector;
    g.icons = icons;
    g.cursor = cursor;
}

/// IconSelector::refresh : les icônes obtenues apparaissent en grandissant.
fn refresh(game: &mut Game) {
    for k in 0..GRID * GRID {
        let Some(c) = content(game, k) else { continue };
        let cursor = gallery(game).cursor;
        let Some(batch) = game.g.get(cursor).and_then(|n| n.parent) else { return };
        let Some(icon) = game.child(batch, k) else { continue };
        let earned = game.achieved(c.id);
        game.set_visible(icon, earned);
        gallery(game).icons[k as usize] = earned.then_some(icon);
        if earned {
            game.set_scale(icon, 0.0);
            game.run(icon, Action::scale_to(0.5, 0.5));
        }
    }
}

/// programmaticallySelectIcon : la case retenue, sans animation.
fn select_programmatically(game: &mut Game, index: i32) {
    if !(0..GRID * GRID).contains(&index) {
        return;
    }
    if let Some(icon) = gallery(game).icons[index as usize] {
        game.set_scale(icon, 1.0);
        select_icon_impl(game, index, true);
    }
}

/// selectIconImpl : l'ancienne icône rapetisse, la nouvelle passe devant
/// (et grandit si on l'a touchée) ; le curseur en reprend l'image et bat.
fn select_icon_impl(game: &mut Game, index: i32, programmatic: bool) {
    let Some(icon) = gallery(game).icons[index as usize] else { return };
    let previous = gallery(game).selected;
    if previous != -1
        && let Some(old) = gallery(game).icons[previous as usize]
    {
        game.stop_actions(old);
        game.g.reorder(old, 100);
        game.run(old, Action::scale_to(0.1, 0.5));
    }
    gallery(game).selected = index;
    game.stop_actions(icon);
    game.g.reorder(icon, 0x6e);
    if !programmatic {
        game.set_scale(icon, 0.5);
        game.run(icon, Action::scale_to(0.2, 1.0));
    }
    gallery_changed(game, index);
    let cursor = gallery(game).cursor;
    game.set_visible(cursor, true);
    game.stop_actions(cursor);
    if let Some(c) = content(game, index) {
        let assets = game.assets.clone();
        game.g.set_frame(&assets, cursor, &c.icon);
    }
    let pos = game.pos(icon);
    game.set_pos(cursor, pos.0, pos.1);
    game.run(cursor, Action::forever(Action::sequence(vec![Action::scale_to(0.5, 1.1), Action::scale_to(0.5, 1.0)])));
}

/// IconSelector::selectIcon : la case sous le toucher. La rangée vient de
/// |y / 36 − 5| : au-dessus du panneau, le toucher retombe sur les
/// rangées du bas (comme l'original).
fn select_icon(game: &mut Game, point: (f32, f32)) {
    if !gallery(game).selector_enabled {
        return;
    }
    let selector = gallery(game).selector;
    let (x, y) = game.g.world(selector).invert().apply(point.0, point.1);
    let col = (x / CELL) as i32;
    let row = ((y / CELL) as i32 - 5).abs();
    let col = if (0..GRID).contains(&col) && x >= 0.0 { col } else { -1 };
    if row >= GRID || y < 0.0 || col == -1 {
        return;
    }
    let index = col + row * GRID;
    if gallery(game).selected != index {
        select_icon_impl(game, index, false);
    }
}

/// SceneGallery::cbGalleryChanged : la vignette, le texte et le toast de
/// l'élément choisi.
fn gallery_changed(game: &mut Game, index: i32) {
    let Some(c) = content(game, index) else { return };
    let layer = gallery(game).layer;
    let thumb = gallery(game).thumb;
    let frame = match c.kind {
        0 => c.thumb.as_str(),
        2 => "gthumbanim.png",
        3 => "gthumbmusic.png",
        4 => "gthumbvoice.png",
        _ => "gthumbna.png",
    };
    let assets = game.assets.clone();
    game.g.set_frame(&assets, thumb, frame);
    if let Some(label) = game.child(layer, LABEL) {
        game.g.set_ttf_text(label, &format!("{}\n{}", c.title, c.earned_text), &label_style());
    }
    if let Some(toast) = game.child(layer, TOAST) {
        widgets::set_toast_text(game, toast, &c.content_title);
        game.stop_actions(toast);
        game.set_scale(toast, 0.0);
        game.run(toast, Action::scale_to(0.2, 1.0));
    }
}

// --- Le toucher de la galerie ---------------------------------------------------------------

/// La vignette (ClickableSprite, avant le panneau) puis le panneau, qui
/// prend tout autre toucher.
pub fn touch_began(game: &mut Game, point: (f32, f32)) -> bool {
    let thumb = gallery(game).thumb;
    if gallery(game).thumb_enabled && game.g.shown(thumb) && game.g.contains(thumb, point) {
        game.run(thumb, Action::scale_to(0.2, 0.9));
        gallery(game).grab = Some(Grab::Thumb);
        return true;
    }
    if !gallery(game).selector_enabled {
        return false;
    }
    select_icon(game, point);
    gallery(game).grab = Some(Grab::Selector);
    true
}

pub fn touch_moved(game: &mut Game, point: (f32, f32)) {
    if let Some(Grab::Selector) = gallery(game).grab {
        select_icon(game, point);
    }
}

pub fn touch_ended(game: &mut Game, point: (f32, f32)) {
    if let Some(Grab::Thumb) = gallery(game).grab.take() {
        let thumb = gallery(game).thumb;
        if !gallery(game).thumb_enabled {
            return;
        }
        game.stop_actions(thumb);
        game.set_scale(thumb, 1.0);
        if game.g.contains(thumb, point) {
            game.run(thumb, Action::sequence(vec![Action::delay(0.1), Action::call(thumbnail_clicked)]));
        }
    }
}

/// cbThumbnailClicked : la page de l'élément choisi.
fn thumbnail_clicked(game: &mut Game, _: NodeId) {
    let selected = gallery(game).selected;
    if selected == -1 {
        return;
    }
    game.play_effect("seogg/se01.ogg");
    gallery(game).thumb_enabled = false;
    change_gallery_scene(game, selected);
}

// --- La manette (galerie) ----------------------------------------------------------------------

/// Ce qu'on peut choisir en plus des boutons : les icônes obtenues et la
/// vignette.
pub fn pad_targets(game: &mut Game) -> Vec<NodeId> {
    let g = gallery(game);
    let mut targets: Vec<NodeId> = g.icons.iter().flatten().copied().collect();
    if g.thumb_enabled {
        targets.push(g.thumb);
    }
    targets
}

/// Une icône choisie à la manette est sélectionnée, comme touchée.
pub fn pad_focus(game: &mut Game, node: NodeId) {
    let g = gallery(game);
    let Some(k) = g.icons.iter().position(|&i| i == Some(node)) else { return };
    if g.selector_enabled && g.selected != k as i32 {
        select_icon_impl(game, k as i32, false);
    }
}

/// A sur une icône ou la vignette : la page de l'élément choisi.
pub fn pad_activate(game: &mut Game, node: NodeId) {
    if gallery(game).thumb_enabled {
        thumbnail_clicked(game, node);
    }
}

// --- Les pages des éléments -----------------------------------------------------------------

/// XTLayer : un toucher en cours ou fini, pour reconnaître glissés et
/// tapotements.
#[derive(Default)]
struct Gestures {
    start: (f32, f32),
    /// Début du toucher, en millisecondes.
    began: f32,
    moved: bool,
    taps: u32,
    /// Le tapotement est jugé 250 ms après (tapHandler).
    tap_due: Option<f32>,
}

pub struct Viewer {
    layer: NodeId,
    id: i32,
    kind: i32,
    /// La touche retour ramène à la galerie (pas sur la page NA).
    back_enabled: bool,
    demo: Option<backdrop::Demo>,
    scroll: f32,
    /// Horloge de la page, en millisecondes (millisecondNow).
    clock: f32,
    gestures: Gestures,
    /// Le bandeau de Funczion (musique) est touché.
    banner: Option<NodeId>,
}

fn viewer(game: &mut Game) -> &mut Viewer {
    match &mut game.logic {
        Logic::Viewer(v) => v,
        Logic::Play(p) if p.viewer.is_some() => p.viewer.as_mut().expect("page de la galerie"),
        _ => unreachable!("aucune page de la galerie n'est affichée"),
    }
}

/// La page d'une animation (GalleryDemo), sur la couche de la partie.
pub fn demo_viewer(layer: NodeId, id: i32) -> Box<Viewer> {
    Box::new(Viewer { layer, id, kind: 2, back_enabled: false, demo: None, scroll: 0.0, clock: 0.0, gestures: Gestures::default(), banner: None })
}

/// GalleryDemo::onEnter : l'interface commune, la touche retour active.
pub fn setup_demo_ui(game: &mut Game) {
    setup_common_ui(game, false);
    viewer(game).back_enabled = true;
}

/// La page d'un élément, selon son type : 0 image, 2 animation (une
/// partie de démonstration), 3 musique, 4 voix ; les autres : la page NA.
pub fn enter_content(game: &mut Game, root: NodeId, id: i32) {
    let Some(c) = content(game, id) else { return };
    if c.kind == 2 {
        super::play::enter_demo(game, root, id, &c.hint);
        return;
    }
    let (w, h) = game.design;
    let layer = game.g.layer(game.design);
    game.add(root, layer, 0, 0x1234);
    game.logic = Logic::Viewer(Box::new(Viewer {
        layer,
        id,
        kind: c.kind,
        back_enabled: false,
        demo: None,
        scroll: 0.0,
        clock: 0.0,
        gestures: Gestures::default(),
        banner: None,
    }));
    match c.kind {
        0 => {
            let sky = game.g.gradient_layer([0x80, 0xD0, 0xFD, 0xFF], [0xFF; 4], (w, h));
            if let Some(n) = game.g.get_mut(sky) {
                n.anchor = (0.0, 0.0);
                n.ignore_anchor = true;
            }
            game.add(layer, sky, -1, 10);
            picture(game, layer, &c.hint);
            setup_common_ui(game, true);
            viewer(game).back_enabled = true;
        }
        3 => enter_music(game, &c),
        4 => enter_voice(game, &c),
        _ => {
            let white = game.g.color_layer([0xFF; 4], (w, h));
            if let Some(n) = game.g.get_mut(white) {
                n.anchor = (0.0, 0.0);
                n.ignore_anchor = true;
            }
            game.add(layer, white, -1, 10);
            picture(game, layer, "gi/sky.jpg");
            setup_common_ui(game, false);
        }
    }
}

/// L'image de la page, au centre, à la moitié de ses pixels.
fn picture(game: &mut Game, layer: NodeId, file: &str) {
    let (w, h) = game.design;
    let sprite = game.sprite_file(file);
    game.set_pos(sprite, w * 0.5, h * 0.5);
    game.set_scale(sprite, crate::assets::CONTENT_SCALE * 0.5);
    game.add(layer, sprite, 1, 0xb);
}

/// gallery_common_ui::setup_common_ui : BACK (et SAVE pour une image),
/// le bandeau « glisser » en haut, le compteur en bas à droite ; tout se
/// cache après 2.5 s.
fn setup_common_ui(game: &mut Game, save: bool) {
    let (w, h) = game.design;
    let (layer, id) = (viewer(game).layer, viewer(game).id);
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let back = widgets::pause_item(game, "BACK", (100.0, 44.0), 1, Rc::new(|game: &mut Game, _| menu_back(game)));
    game.set_pos(back, 5.0 + 50.0, 5.0 + 22.0);
    game.add(menu, back, 0, 1);
    if save {
        let item = widgets::pause_item(game, "", (80.0, 44.0), 2, Rc::new(|game: &mut Game, _| save_picture(game)));
        let icon = game.sprite("saveicon.png");
        game.set_color(icon, [0, 0, 0]);
        game.set_pos(icon, 40.0, 22.0);
        game.set_scale(icon, 0.8);
        game.add(item, icon, 0, -1);
        game.set_pos(item, 100.0 + 5.0 + 40.0, 5.0 + 22.0);
        game.add(menu, item, 0, 2);
    }
    game.add(layer, menu, MENU, MENU);
    let bar = game.g.color_layer([0, 0, 0, 0x7F], (w, 30.0));
    if let Some(n) = game.g.get_mut(bar) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.set_pos(bar, 0.0, h - 30.0);
    let text = game.text("Swipe to change screen");
    let label = game.g.ttf_label(&text, &TextStyle { font: "Arial", size: 16.0, dimensions: (w, 22.0), h_align: 1, v_align: 0 });
    game.set_pos(label, w * 0.5, 15.0);
    game.add(bar, label, 0, -1);
    let counter = game.sprite_file("roundsquare.png");
    game.set_color(counter, [0, 0, 0]);
    game.set_opacity(counter, 0x7F);
    game.set_pos(counter, w - 55.0, 25.0);
    let number = game.g.ttf_label(&format!("{:02}/31", id + 1), &TextStyle { font: "Arial", size: 16.0, dimensions: (100.0, 38.0), h_align: 1, v_align: 1 });
    game.set_anchor(number, 0.0, 0.0);
    game.set_pos(number, 0.0, 0.0);
    // Le texte reste blanc et opaque sous le fond assombri.
    if let Some(n) = game.g.get_mut(counter) {
        n.cascade_color = false;
        n.cascade_opacity = false;
    }
    game.add(counter, number, 0, -1);
    game.add(layer, bar, BAR, BAR);
    game.add(layer, counter, COUNTER, COUNTER);
    // Le premier effacement n'a pas d'étiquette : un tapotement ne
    // l'arrête pas (comme l'original).
    game.run(layer, Action::sequence(vec![Action::delay(2.5), Action::call(|game: &mut Game, _| hide_common_ui(game))]));
}

/// hide_common_ui : le bandeau monte, le compteur et les boutons
/// descendent, puis tout se cache.
fn hide_common_ui(game: &mut Game) {
    let layer = viewer(game).layer;
    for (tag, dy) in [(BAR, 30.0), (COUNTER, -50.0), (MENU, -50.0)] {
        if let Some(node) = game.child(layer, tag) {
            game.run(node, Action::sequence(vec![Action::move_by(0.5, (0.0, dy)), Action::hide()]));
        }
    }
}

/// show_common_ui (xtTapGesture) : tout revient à sa place, et se recache
/// 2.5 s plus tard.
fn show_common_ui(game: &mut Game) {
    let (w, h) = game.design;
    let layer = viewer(game).layer;
    game.stop_action_tag(layer, HIDE_ACTION);
    for (tag, pos) in [(BAR, (0.0, h - 30.0)), (COUNTER, (w - 55.0, 25.0)), (MENU, (0.0, 0.0))] {
        if let Some(node) = game.child(layer, tag) {
            game.set_pos(node, pos.0, pos.1);
            game.set_visible(node, true);
        }
    }
    game.run_tagged(layer, Action::sequence(vec![Action::delay(2.5), Action::call(|game: &mut Game, _| hide_common_ui(game))]), HIDE_ACTION);
}

/// Le retour à la galerie (BACK ou back_to_gallery), l'élément retenu ;
/// les animations, la musique et les voix jouent le son du bouton.
fn to_gallery(game: &mut Game) {
    let id = viewer(game).id;
    game.replace_scene_with(SceneKind::Gallery, Transition::CrossFade(0.6));
    game.save.set_int("positon_gallery", id);
}

fn menu_back(game: &mut Game) {
    if matches!(viewer(game).kind, 2..=4) {
        game.play_effect("seogg/se01.ogg");
    }
    to_gallery(game);
}

/// La touche retour : la galerie, sauf sur la page NA (son drapeau n'est
/// jamais levé).
pub fn back_content(game: &mut Game) {
    if viewer(game).back_enabled {
        to_gallery(game);
    }
}

/// GalleryWallpaper::menuTest (SAVE) : l'image va dans le dossier Images
/// (l'original l'ajoutait à la galerie du téléphone) ; un toast dit si
/// c'est fait.
fn save_picture(game: &mut Game) {
    let id = viewer(game).id;
    let Some(c) = content(game, id) else { return };
    let saved = (|| {
        let path = game.assets.resolve(&c.hint)?;
        let bytes = game.assets.bytes(&path)?;
        let dir = crate::platform::pictures()?.join("Nanaca Crash Android");
        std::fs::create_dir_all(&dir).ok()?;
        let name = std::path::Path::new(&c.hint).file_name()?;
        std::fs::write(dir.join(name), bytes).ok()
    })()
    .is_some();
    let text = game.text(if saved { "SAVE_IMAGE_TO_CAMERAROLL" } else { "SAVE_IMAGE_TO_CAMERAROLL_FAILED" });
    let (w, h) = game.design;
    let layer = viewer(game).layer;
    let toast = widgets::toast(game, (190.0, 40.0), &text);
    game.set_pos(toast, w * 0.5, h * 0.5);
    game.add(layer, toast, 10000, -1);
    game.run(toast, Action::sequence(vec![Action::delay(2.0), Action::remove_self()]));
}

/// GalleryMusic::onEnter : le titre du morceau, PLAY, le bandeau du
/// compositeur, le décor.
fn enter_music(game: &mut Game, c: &Content) {
    let (w, h) = game.design;
    let layer = viewer(game).layer;
    let sky = game.g.gradient_layer([0x80, 0xD0, 0xFD, 0xFF], [0xFF; 4], (w, h));
    if let Some(n) = game.g.get_mut(sky) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.add(layer, sky, -1, 10);
    let title = game.bm_label(&c.thumb, "arialblack48short.fnt", 2);
    let size = game.size(title);
    game.set_color(title, [0, 0, 0]);
    game.set_anchor(title, 0.0, 1.0);
    game.set_pos(title, (w - size.0) - 30.0, h - 30.0);
    game.add(layer, title, 9999, 0x270d);
    setup_common_ui(game, false);
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let play = widgets::pause_item(game, "PLAY", (200.0, 44.0), 1, Rc::new(|game: &mut Game, _| play_music(game)));
    game.set_pos(play, (w - 100.0) - 30.0, 80.0);
    game.add(menu, play, 0, 1);
    game.add(layer, menu, 9999, ITEMS);
    let banner = game.sprite_file("funczion_banner.png");
    let bw = game.size(banner).0;
    game.set_pos(banner, (w - bw * 0.5) - 30.0, 180.0);
    game.add(layer, banner, 10, BANNER);
    viewer(game).banner = Some(banner);
    let composed = game.g.ttf_label("COMPOSED BY", &TextStyle { font: "Arial", size: 16.0, dimensions: (200.0, 38.0), h_align: 1, v_align: 1 });
    game.set_anchor(composed, 0.0, 0.0);
    game.set_color(composed, [0, 0, 0]);
    game.set_pos(composed, w - 200.0, 210.0);
    game.add(layer, composed, 0xb, -1);
    let demo = backdrop::create(game, layer, 1);
    let v = viewer(game);
    v.demo = Some(demo);
    v.back_enabled = true;
}

/// GalleryMusic::menuTest (PLAY) : le morceau, en boucle depuis son
/// point de reprise (On Air 6.1 s, Air Waves Arr. 24.049 s).
fn play_music(game: &mut Game) {
    let id = viewer(game).id;
    let Some(c) = content(game, id) else { return };
    let loop_start = match id {
        8 => 6.1,
        9 => 24.049,
        _ => 0.0,
    };
    game.audio.stop_music();
    game.play_music(&format!("bgmogg/{}.ogg", c.hint), true, loop_start);
}

/// GalleryVoice::onEnter : le portrait, puis sept voix (MAIL, CALL…) ; les
/// trois premières sont grisées pour Tomoki.
fn enter_voice(game: &mut Game, c: &Content) {
    let (w, h) = game.design;
    let layer = viewer(game).layer;
    let white = game.g.color_layer([0xFF; 4], (w, h));
    if let Some(n) = game.g.get_mut(white) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.add(layer, white, -1, 10);
    picture(game, layer, &c.hint);
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    let size = (120.0, 44.0);
    for i in 0..7 {
        let tag = i + 1;
        let text = game.text(&format!("Voice{tag}"));
        let item = widgets::pause_ttf_item(game, &text, size, tag, Rc::new(move |game: &mut Game, _| play_voice(game, tag)));
        let y = (220 - (i / 2) * 50) as f32;
        let x = if i % 2 == 0 { (w - size.0 * 1.5) - 20.0 } else { (w - size.0 * 0.5) - 10.0 };
        game.set_pos(item, x, y);
        game.add(menu, item, 0, tag);
        if c.id == 26 && i < 3 {
            widgets::set_item_enabled(game, item, false);
        }
    }
    game.add(layer, menu, 9999, ITEMS);
    setup_common_ui(game, false);
    viewer(game).back_enabled = true;
}

/// GalleryVoice::menuTest : la voix du bouton (« nmiki01.ogg »…), ou
/// l'appel du titre pour le septième.
fn play_voice(game: &mut Game, tag: i32) {
    let id = viewer(game).id;
    let Some(c) = content(game, id) else { return };
    let parts: Vec<&str> = c.thumb.split(',').collect();
    game.audio.stop_all_effects();
    if parts.len() >= 2 {
        let path = if tag < 7 { format!("spvoiceogg/{}{:02}.ogg", parts[0], tag) } else { format!("spvoiceogg/{}.ogg", parts[1]) };
        game.play_effect(&path);
    }
}

/// La page suit son décor (musique) et juge les tapotements en attente.
pub fn update_content(game: &mut Game, dt: f32) {
    let v = viewer(game);
    v.clock += dt * 1000.0;
    let scroll = v.scroll;
    if let Some(demo) = viewer(game).demo.take() {
        backdrop::move_cast(game, &demo, scroll);
        let v = viewer(game);
        v.demo = Some(demo);
        backdrop::advance(&mut v.scroll, dt);
    }
    let v = viewer(game);
    if v.gestures.tap_due.is_some_and(|t| v.clock >= t) {
        v.gestures.tap_due = None;
        tap_handler(game);
    }
}

// --- Les gestes (XTLayer) -------------------------------------------------------------------

/// Le bandeau du compositeur (ClickableSprite) d'abord, puis la page qui
/// prend tout toucher.
pub fn touch_began_content(game: &mut Game, point: (f32, f32)) -> bool {
    if let Some(banner) = viewer(game).banner
        && game.g.shown(banner)
        && game.g.contains(banner, point)
    {
        game.run(banner, Action::scale_to(0.2, 0.9));
        viewer(game).gestures.taps = u32::MAX;
        return true;
    }
    let v = viewer(game);
    v.gestures.moved = false;
    v.gestures.began = v.clock;
    v.gestures.start = point;
    true
}

pub fn touch_moved_content(game: &mut Game, point: (f32, f32)) {
    let g = &mut viewer(game).gestures;
    if g.taps == u32::MAX {
        return;
    }
    let (dx, dy) = (point.0 - g.start.0, point.1 - g.start.1);
    if (dx * dx + dy * dy).sqrt() >= 20.0 {
        g.moved = true;
    }
}

/// ccTouchesEnded : un glissé (plus de 30 % de l'écran en moins de
/// 500 ms), un appui long (2 s), ou un tapotement jugé 250 ms après.
pub fn touch_ended_content(game: &mut Game, point: (f32, f32)) {
    if viewer(game).gestures.taps == u32::MAX {
        viewer(game).gestures.taps = 0;
        let Some(banner) = viewer(game).banner else { return };
        game.stop_actions(banner);
        game.set_scale(banner, 1.0);
        if game.g.contains(banner, point) {
            game.run(banner, Action::sequence(vec![Action::delay(0.1), Action::call(|game: &mut Game, _| crate::platform::open_url(&game.text("url_funczions")))]));
        }
        return;
    }
    let (w, h) = game.design;
    let v = viewer(game);
    let elapsed = v.clock - v.gestures.began;
    let (dx, dy) = (v.gestures.start.0 - point.0, v.gestures.start.1 - point.1);
    let (ax, ay) = (dx.abs(), dy.abs());
    if ax > ay && (dx / w * 100.0).abs() > 30.0 && elapsed < 500.0 {
        swipe(game, if dx < 0.0 { 0 } else { 1 });
        return;
    }
    if ay > ax && (dy / h * 100.0).abs() > 30.0 && elapsed < 500.0 {
        swipe(game, if dy < 0.0 { 3 } else { 2 });
        return;
    }
    if elapsed >= 2000.0 {
        // xtLongTapGesture : rien sur ces pages.
        return;
    }
    let v = viewer(game);
    if v.gestures.taps == 0 {
        v.gestures.tap_due = Some(v.clock + 250.0);
    }
    v.gestures.taps += 1;
}

/// La manette sur une page : le bandeau du compositeur à choisir.
pub fn pad_targets_content(game: &mut Game) -> Vec<NodeId> {
    let banner = viewer(game).banner;
    banner.into_iter().filter(|&b| game.g.shown(b)).collect()
}

/// A sur le bandeau : la page de Funczion.
pub fn pad_activate_content(game: &mut Game, _node: NodeId) {
    crate::platform::open_url(&game.text("url_funczions"));
}

/// Gauche ou droite sans bouton de ce côté : l'élément précédent ou
/// suivant, comme un glissé.
pub fn pad_swipe(game: &mut Game, right: bool) {
    swipe(game, if right { 1 } else { 0 });
}

/// tapHandler : un seul tapotement sans glisser montre l'interface ; deux
/// (xtDoubleTapGesture) ne font rien.
fn tap_handler(game: &mut Game) {
    let g = &mut viewer(game).gestures;
    let single = g.taps == 1 && !g.moved;
    g.taps = 0;
    if single {
        show_common_ui(game);
    }
}

/// xtSwipeGesture : vers la droite l'élément obtenu précédent, vers la
/// gauche le suivant (en boucle) ; rien de haut en bas.
fn swipe(game: &mut Game, direction: i32) {
    if direction > 1 {
        return;
    }
    let id = viewer(game).id;
    let earned: Vec<i32> = {
        let mut ids: Vec<i32> = game.gallery.iter().map(|c| c.id).filter(|&i| game.achieved(i)).collect();
        ids.sort_unstable();
        ids
    };
    if earned.len() < 2 {
        return;
    }
    let Some(k) = earned.iter().position(|&i| i == id) else { return };
    let next = if direction == 0 { earned[(k + earned.len() - 1) % earned.len()] } else { earned[(k + 1) % earned.len()] };
    change_gallery_scene(game, next);
}
