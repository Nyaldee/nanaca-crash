//! La manette et le clavier dans le jeu, comme dans le remake : la croix,
//! le stick ou les flèches choisissent un bouton (il prend son aspect
//! appuyé), A ou Entrée le touche (sans bouton choisi : un toucher sans
//! viser, pour la jauge et les attaques), B ou Échap font la touche retour
//! d'Android, Start met en pause. Les icônes de la galerie, les skins et
//! les curseurs des options se choisissent aussi (un cadre les montre).

use crate::action::Action;
use crate::node::{Kind, MenuItem, NodeId};

use super::{Game, Logic, gallery, option, play};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// Ce qu'on peut choisir.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    /// Un bouton de menu.
    Item(NodeId),
    /// Un élément de la scène (icône, vignette, skin, bandeau).
    Tap(NodeId),
    /// Un curseur de volume : gauche et droite le règlent.
    Slider(NodeId),
}

impl Target {
    fn node(self) -> NodeId {
        match self {
            Target::Item(n) | Target::Tap(n) | Target::Slider(n) => n,
        }
    }
}

/// Un point loin de tout bouton : le toucher va à la scène.
const NOWHERE: (f32, f32) = (-10000.0, -10000.0);

impl Game {
    /// Les boutons des menus actifs (la boîte de dialogue seule si elle
    /// est ouverte), puis ce que la scène propose.
    fn targets(&mut self) -> Vec<Target> {
        let menus = match self.modal {
            Some(m) if self.g.alive(m) => vec![m],
            _ => self.menus.clone(),
        };
        let mut out = Vec::new();
        for menu in menus {
            if !self.menu_active(menu) {
                continue;
            }
            for &c in self.g.get(menu).map(|n| n.children.as_slice()).unwrap_or_default() {
                let usable = self.g.get(c).is_some_and(|n| matches!(n.kind, Kind::Item(MenuItem { enabled: true, .. })));
                if usable && self.g.shown(c) {
                    out.push(Target::Item(c));
                }
            }
        }
        if self.modal.is_some_and(|m| self.g.alive(m)) {
            return out;
        }
        match &self.logic {
            Logic::Gallery(_) => out.extend(gallery::pad_targets(self).into_iter().map(Target::Tap)),
            Logic::Option(_) => {
                let (cells, sliders) = option::pad_targets(self);
                out.extend(cells.into_iter().map(Target::Tap));
                out.extend(sliders.into_iter().map(Target::Slider));
            }
            Logic::Viewer(_) => out.extend(gallery::pad_targets_content(self).into_iter().map(Target::Tap)),
            Logic::Play(p) if p.viewer.is_some() => out.extend(gallery::pad_targets_content(self).into_iter().map(Target::Tap)),
            _ => {}
        }
        out.retain(|t| self.g.shown(t.node()));
        out
    }

    /// Le centre d'un élément, dans la scène.
    fn center(&self, id: NodeId) -> (f32, f32) {
        let size = self.g.get(id).map_or((0.0, 0.0), |n| n.size);
        self.g.world(id).apply(size.0 * 0.5, size.1 * 0.5)
    }

    /// Ce qui est choisi, s'il est toujours là.
    fn current_focus(&mut self) -> Option<Target> {
        let f = self.focus?;
        if self.targets().contains(&f) {
            return Some(f);
        }
        self.set_focus(None);
        None
    }

    /// Le choix change : l'ancien bouton reprend son aspect, le nouveau
    /// prend l'aspect appuyé ; le cadre suit les autres éléments, et les
    /// boutons sans aspect appuyé (ceux du titre).
    fn set_focus(&mut self, focus: Option<Target>) {
        if let Some(Target::Item(old)) = self.focus
            && self.g.alive(old)
        {
            self.set_selected(old, false);
        }
        self.focus = focus;
        match focus {
            Some(Target::Item(item)) => {
                self.set_selected(item, true);
                let styled = self.item_styles.contains_key(&item);
                self.place_frame((!styled).then_some(item));
            }
            Some(Target::Tap(n) | Target::Slider(n)) => self.place_frame(Some(n)),
            None => self.place_frame(None),
        }
    }

    /// Le cadre de choix (blanc, battant) sur un élément de la scène.
    fn place_frame(&mut self, target: Option<NodeId>) {
        let Some(root) = self.scene else { return };
        let frame = match self.focus_frame.filter(|&f| self.g.alive(f)) {
            Some(f) => f,
            None => {
                let f = self.g.color_layer([0xFF; 4], (0.0, 0.0));
                self.set_opacity(f, 0x70);
                if let Some(n) = self.g.get_mut(f) {
                    n.anchor = (0.0, 0.0);
                    n.ignore_anchor = true;
                }
                self.add(root, f, i32::MAX, -1);
                self.run(f, Action::forever(Action::sequence(vec![Action::fade_to(0.4, 0x30), Action::fade_to(0.4, 0x70)])));
                self.focus_frame = Some(f);
                f
            }
        };
        let Some(id) = target else {
            self.set_visible(frame, false);
            return;
        };
        // Le rectangle de l'élément dans la scène, à l'échelle de ses parents.
        let size = self.g.get(id).map_or((0.0, 0.0), |n| n.size);
        let w = self.g.world(id);
        let corners = [w.apply(0.0, 0.0), w.apply(size.0, 0.0), w.apply(0.0, size.1), w.apply(size.0, size.1)];
        let (x0, x1) = corners.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.0), b.max(p.0)));
        let (y0, y1) = corners.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p.1), b.max(p.1)));
        if let Some(n) = self.g.get_mut(frame) {
            n.pos = (x0 - 2.0, y0 - 2.0);
            n.size = (x1 - x0 + 4.0, y1 - y0 + 4.0);
        }
        self.set_visible(frame, true);
    }

    /// Chaque image : le cadre suit son élément (listes qui défilent,
    /// icônes qui grandissent), et disparaît avec lui.
    pub(super) fn follow_focus(&mut self) {
        match self.focus {
            Some(Target::Tap(n) | Target::Slider(n)) if self.g.alive(n) && self.g.shown(n) => self.place_frame(Some(n)),
            Some(Target::Tap(_) | Target::Slider(_)) => self.set_focus(None),
            // Un bouton du titre : tant qu'il se touche (le menu se coupe
            // quand un bouton part).
            Some(t @ Target::Item(n)) if !self.item_styles.contains_key(&n) => {
                if self.targets().contains(&t) {
                    self.place_frame(Some(n));
                } else {
                    self.set_focus(None);
                }
            }
            _ => {}
        }
    }

    /// Croix, stick ou flèches : l'élément le plus proche dans cette
    /// direction (le plus en haut à gauche s'il n'y a pas de choix).
    pub fn pad_nav(&mut self, dir: Dir) {
        if self.input_blocked() || self.tracking.is_some() {
            return;
        }
        let current = self.current_focus();
        // Un curseur choisi se règle à gauche et à droite.
        if let Some(Target::Slider(s)) = current
            && matches!(dir, Dir::Left | Dir::Right)
            && let Logic::Option(_) = self.logic
        {
            option::pad_slide(self, s, if dir == Dir::Left { -0.1 } else { 0.1 });
            return;
        }
        let list: Vec<(Target, (f32, f32))> = self.targets().into_iter().map(|t| (t, self.center(t.node()))).collect();
        let next = match current {
            None => list.iter().min_by(|a, b| (a.1.0 - a.1.1).total_cmp(&(b.1.0 - b.1.1))).map(|c| c.0),
            Some(f) => {
                let (fx, fy) = self.center(f.node());
                list.iter()
                    .filter(|c| c.0 != f)
                    .filter_map(|&(t, (cx, cy))| {
                        // L'écart dans la direction (y vers le haut), l'écart
                        // de côté compte moitié ; à gauche et à droite,
                        // seulement à peu près sur la même ligne.
                        let (dx, dy) = (cx - fx, cy - fy);
                        let (along, side) = match dir {
                            Dir::Up => (dy, dx),
                            Dir::Down => (-dy, dx),
                            Dir::Left => (-dx, dy),
                            Dir::Right => (dx, dy),
                        };
                        let horizontal = matches!(dir, Dir::Left | Dir::Right);
                        (along > 1.0 && (!horizontal || side.abs() <= along / 2.0)).then_some((t, along + 0.5 * side.abs()))
                    })
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|c| c.0)
            }
        };
        match next {
            Some(t) => {
                self.set_focus(Some(t));
                if let Target::Tap(n) = t {
                    match self.logic {
                        Logic::Gallery(_) => gallery::pad_focus(self, n),
                        Logic::Option(_) => option::pad_focus(self, n),
                        _ => {}
                    }
                }
            }
            // Rien de ce côté sur une page de la galerie : l'élément voisin.
            None if matches!(dir, Dir::Left | Dir::Right) && self.is_gallery_page() => gallery::pad_swipe(self, dir == Dir::Right),
            None => {}
        }
    }

    /// La souris reprend la main : plus de choix.
    pub fn clear_focus(&mut self) {
        if self.focus.is_some() {
            self.set_focus(None);
        }
    }

    fn is_gallery_page(&self) -> bool {
        match &self.logic {
            Logic::Viewer(_) => true,
            Logic::Play(p) => p.viewer.is_some(),
            _ => false,
        }
    }

    /// A ou Entrée, appuyé puis relâché : le bouton choisi, l'élément
    /// choisi, ou un toucher sans viser.
    pub fn pad_confirm(&mut self, down: bool) {
        match self.current_focus() {
            Some(Target::Item(item)) => {
                let p = self.center(item);
                if down {
                    self.touch_began(p);
                } else {
                    self.touch_ended(p);
                    // L'action faite, le bouton garde son aspect choisi.
                    if self.focus == Some(Target::Item(item)) && self.targets().contains(&Target::Item(item)) {
                        self.set_selected(item, true);
                    }
                }
            }
            Some(Target::Tap(n)) => {
                if !down {
                    match self.logic {
                        Logic::Gallery(_) => gallery::pad_activate(self, n),
                        Logic::Option(_) => option::pad_activate(self, n),
                        Logic::Viewer(_) | Logic::Play(_) => gallery::pad_activate_content(self, n),
                        _ => {}
                    }
                }
            }
            Some(Target::Slider(_)) => {}
            None => {
                if down {
                    self.touch_began(NOWHERE);
                } else {
                    self.touch_ended(NOWHERE);
                }
            }
        }
    }

    /// B ou Échap : la touche retour d'Android. Une liste ouverte se
    /// ferme (sa croix) ; sinon la scène fait comme l'original (rien au
    /// titre, où l'original ouvrait la boîte « quitter », ni en partie).
    pub fn pad_back(&mut self) {
        if self.input_blocked() || self.tracking.is_some() {
            return;
        }
        if let Some(m) = self.modal.filter(|&m| self.g.alive(m)) {
            if let Some(&cross) = self.g.get(m).and_then(|n| n.children.first())
                && let Some(a) = self.item_actions.get(&cross).cloned()
            {
                a(self, cross);
            }
            return;
        }
        match &self.logic {
            Logic::Option(_) => option::back(self),
            Logic::Gallery(_) => gallery::back(self),
            Logic::Viewer(_) => gallery::back_content(self),
            Logic::Play(p) if p.viewer.is_some() => gallery::back_content(self),
            _ => {}
        }
    }

    /// Start : la pause pendant la partie, ou la reprise.
    pub fn pad_pause(&mut self) {
        if self.input_blocked() {
            return;
        }
        if let Logic::Play(p) = &self.logic
            && p.viewer.is_none()
        {
            play::toggle_pause(self);
        }
    }
}
