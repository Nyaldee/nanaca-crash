//! Les éléments d'interface du jeu partagés par plusieurs écrans :
//! MyMenuItemPause, MyMenuItemSquareThin, ToastLabel, SelectListDialog.

use std::rc::Rc;

use crate::action::{Action, Callback};
use crate::node::{Kind, MenuItem, Node, NodeId};
use crate::text::TextStyle;

use super::Game;

/// Un bouton de menu (CCMenuItemSprite) autour de son image.
fn item(game: &mut Game, image: NodeId, size: (f32, f32), tag: i32, action: Callback) -> NodeId {
    let mut n = Node::new(Kind::Item(MenuItem { enabled: true, selected: false }));
    n.size = size;
    n.anchor = (0.5, 0.5);
    n.cascade_color = true;
    n.cascade_opacity = true;
    let item = game.g.add(n);
    game.set_anchor(image, 0.0, 0.0);
    game.set_pos(image, 0.0, 0.0);
    game.add(item, image, 0, -1);
    game.register_item(item, action);
    if let Some(n) = game.g.get_mut(item) {
        n.tag = tag;
    }
    item
}

/// MyMenuItemPause : menu_f.png étiré, texte noir en arialblack32 ;
/// pressé, il rétrécit (0.95) et son texte rougit.
pub fn pause_item(game: &mut Game, text: &str, size: (f32, f32), tag: i32, action: Callback) -> NodeId {
    let assets = game.assets.clone();
    let back = game.g.scale9(&assets, "menu_f.png", Some([20.0, 0.0, 80.0, 38.0]), size);
    let label = game.bm_label(text, "arialblack32.fnt", 1);
    game.set_pos(label, size.0 * 0.5, size.1 * 0.5 + 2.0);
    game.set_color(label, [0, 0, 0]);
    game.add(back, label, 1, 1);
    let item = item(game, back, size, tag, action);
    // Désactivé, seule l'image se grise (CCScale9Sprite ne transmet pas).
    if let Some(n) = game.g.get_mut(back) {
        n.cascade_color = false;
    }
    game.style_item(
        item,
        Rc::new(move |game: &mut Game, item, on| {
            game.set_scale(item, if on { 0.95 } else { 1.0 });
            game.set_color(label, if on { [0xFF, 0, 0] } else { [0, 0, 0] });
        }),
    );
    item
}

/// MyMenuItemPause::setTextScale.
pub fn set_pause_text_scale(game: &mut Game, item: NodeId, scale: f32) {
    if let Some(label) = pause_label(game, item) {
        game.set_scale(label, scale);
    }
}

fn pause_label(game: &Game, item: NodeId) -> Option<NodeId> {
    let back = game.g.get(item)?.children.first().copied()?;
    game.child(back, 1)
}

/// La police des boutons fins (Arial 12, dans tout le bouton).
fn thin_style(size: (f32, f32), h_align: u8) -> TextStyle<'static> {
    TextStyle { font: "Arial", size: 12.0, dimensions: size, h_align, v_align: 1 }
}

/// MyMenuItemSquareThin : thinembossframe.png étiré et un texte noir ;
/// pressé, il s'assombrit (l'opacité reste ensuite à moitié, comme
/// l'original). Texte centré (setTextAlignmentCenter).
pub fn thin_item(game: &mut Game, text: &str, size: (f32, f32), tag: i32, action: Callback) -> NodeId {
    let assets = game.assets.clone();
    let back = game.g.scale9(&assets, "thinembossframe.png", Some([2.0, 2.0, 27.0, 27.0]), size);
    let label = game.g.ttf_label(text, &thin_style(size, 1));
    game.set_anchor(label, 0.0, 0.0);
    game.set_pos(label, 0.0, 0.0);
    game.set_color(label, [0, 0, 0]);
    game.add(back, label, 1, 1);
    let item = item(game, back, size, tag, action);
    // Couleur et opacité vont à l'image, pas au texte.
    if let Some(n) = game.g.get_mut(back) {
        n.cascade_color = false;
        n.cascade_opacity = false;
    }
    game.style_item(
        item,
        Rc::new(|game: &mut Game, item, on| {
            game.set_scale(item, 1.0);
            if on {
                game.set_opacity(item, 0x7F);
                game.set_color(item, [0x7F; 3]);
            } else {
                game.set_color(item, [0xFF; 3]);
            }
        }),
    );
    item
}

/// Le texte d'un bouton fin (setString de son étiquette).
pub fn set_thin_text(game: &mut Game, item: NodeId, text: &str) {
    let Some(back) = game.g.get(item).and_then(|n| n.children.first().copied()) else { return };
    let size = game.size(item);
    if let Some(label) = game.child(back, 1) {
        game.g.set_ttf_text(label, text, &thin_style(size, 1));
    }
}

/// MyMenuItemPauseTTF : comme MyMenuItemPause, texte noir en Arial 20 ;
/// désactivé, son image se grise (pas le texte).
pub fn pause_ttf_item(game: &mut Game, text: &str, size: (f32, f32), tag: i32, action: Callback) -> NodeId {
    let assets = game.assets.clone();
    let back = game.g.scale9(&assets, "menu_f.png", Some([20.0, 0.0, 80.0, 38.0]), size);
    let label = game.g.ttf_label(text, &TextStyle { font: "Arial", size: 20.0, dimensions: size, h_align: 1, v_align: 1 });
    game.set_pos(label, size.0 * 0.5, size.1 * 0.5 + 2.0);
    game.set_color(label, [0, 0, 0]);
    game.add(back, label, 1, 1);
    let item = item(game, back, size, tag, action);
    if let Some(n) = game.g.get_mut(back) {
        n.cascade_color = false;
    }
    game.style_item(
        item,
        Rc::new(move |game: &mut Game, item, on| {
            game.set_scale(item, if on { 0.95 } else { 1.0 });
            game.set_color(label, if on { [0xFF, 0, 0] } else { [0, 0, 0] });
        }),
    );
    item
}

/// MyMenuItemPauseTTF::setEnabled : désactivé, l'image passe à 0x7f.
pub fn set_item_enabled(game: &mut Game, item: NodeId, on: bool) {
    if let Some(n) = game.g.get_mut(item)
        && let Kind::Item(i) = &mut n.kind
    {
        i.enabled = on;
    }
    game.set_color(item, if on { [0xFF; 3] } else { [0x7F; 3] });
}

/// La police des toasts : Arial 14, centrée.
fn toast_style(size: (f32, f32)) -> TextStyle<'static> {
    TextStyle { font: "Arial", size: 14.0, dimensions: size, h_align: 1, v_align: 1 }
}

/// ToastLabel : roundsquare.png noir à moitié transparent, texte blanc
/// centré (Arial 14).
pub fn toast(game: &mut Game, size: (f32, f32), text: &str) -> NodeId {
    let assets = game.assets.clone();
    let back = game.g.scale9(&assets, "roundsquare.png", None, size);
    game.set_color(back, [0, 0, 0]);
    game.set_opacity(back, 0x7F);
    let label = game.g.ttf_label(text, &toast_style(size));
    game.set_anchor(label, 0.0, 0.0);
    game.set_pos(label, 0.0, 0.0);
    game.add(back, label, 1, 1);
    back
}

/// Le texte d'un toast (son étiquette, étiquette 1).
pub fn set_toast_text(game: &mut Game, toast: NodeId, text: &str) {
    let size = game.size(toast);
    if let Some(label) = game.child(toast, 1) {
        game.g.set_ttf_text(label, text, &toast_style(size));
    }
}

/// Le choix fait dans une liste : étiquette de la liste, numéro choisi.
pub type ListChoice = Rc<dyn Fn(&mut Game, i32, i32)>;

/// SelectListDialog : un panneau de 300 × 300 au centre, titre,
/// explication, un bouton fin par choix et une croix ; il s'ouvre en
/// grandissant (0.3 s), prend tout le toucher, et se referme de même.
pub fn select_list(game: &mut Game, tag: i32, title: &str, desc: &str, choices: &[(i32, String)], on_choice: ListChoice) -> NodeId {
    let (vw, vh) = game.design;
    let layer = game.g.layer(game.design);
    let (pw, ph) = (300.0, 300.0);
    let panel = game.g.color_layer([0xFF, 0xFF, 0xFF, 0x1F], (pw, ph));
    if let Some(n) = game.g.get_mut(panel) {
        n.anchor = (0.0, 0.0);
        n.ignore_anchor = true;
    }
    game.set_pos(panel, (vw - pw) * 0.5, (vh - ph) * 0.5);
    game.add(layer, panel, 0, -1);
    let assets = game.assets.clone();
    let frame = game.g.scale9(&assets, "squarebutton.png", Some([4.0, 4.0, 30.0, 28.0]), (pw, ph));
    game.set_pos(frame, pw * 0.5, ph * 0.5);
    game.add(panel, frame, -2, -1);
    let label = game.bm_label(title, "arialblack48short.fnt", 0);
    game.set_color(label, [0, 0, 0]);
    game.set_anchor(label, 0.0, 0.0);
    game.set_pos(label, 20.0, 270.0);
    game.set_scale(label, 0.8);
    game.add(panel, label, 0, -1);
    let text = game.g.ttf_label(desc, &TextStyle { font: "Arial", size: 12.0, dimensions: (0.0, 0.0), h_align: 0, v_align: 1 });
    game.set_anchor(text, 0.0, 0.0);
    game.set_color(text, [0, 0, 0]);
    game.set_pos(text, 20.0, 255.0);
    game.add(panel, text, 0, -1);
    let menu = game.menu();
    if let Some(n) = game.g.get_mut(menu) {
        n.pos = (0.0, 0.0);
        n.anchor = (0.0, 0.0);
    }
    // La croix, en haut à droite du panneau.
    let cross = game.sprite("close1.png");
    game.set_color(cross, [0, 0, 0]);
    let cross_size = game.size(cross);
    let close = item(game, cross, cross_size, -1, Rc::new(move |game: &mut Game, _| close_list(game, layer)));
    game.style_item(close, Rc::new(move |game: &mut Game, _, on| game.set_color(cross, if on { [0xFF, 0x66, 0] } else { [0, 0, 0] })));
    game.set_pos(close, (vw + pw - cross_size.0) * 0.5 - 10.0, (vh + ph - cross_size.1) * 0.5 - 10.0);
    game.add(menu, close, 0, -1);
    let mut y = choices.len() as f32 * 22.0 + 130.0;
    for (key, text) in choices {
        let key = *key;
        let on_choice = on_choice.clone();
        let choice = thin_item(
            game,
            text,
            (270.0, 40.0),
            key,
            Rc::new(move |game: &mut Game, _| {
                on_choice(game, tag, key);
                close_list(game, layer);
            }),
        );
        game.set_opacity(choice, 0xAA);
        game.set_pos(choice, ((vw + pw) - 270.0) * 0.5 - 15.0, y);
        game.add(menu, choice, 0, key);
        y -= 45.0;
    }
    game.add(layer, menu, 0, -1);
    game.set_scale(layer, 0.0);
    game.run(layer, Action::scale_to(0.3, 1.0));
    game.modal = Some(menu);
    layer
}

/// menuCloseCallback : la boîte rapetisse, puis disparaît (cleanupSelf) ;
/// elle prend le toucher jusque-là.
fn close_list(game: &mut Game, layer: NodeId) {
    let release = Action::call(|game: &mut Game, layer| {
        if game.modal.is_some_and(|m| game.g.get(m).is_some_and(|n| n.parent == Some(layer))) {
            game.modal = None;
        }
    });
    game.run(layer, Action::sequence(vec![Action::scale_to(0.3, 0.0), release, Action::remove_self()]));
}
