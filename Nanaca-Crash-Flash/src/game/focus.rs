//! La manette dans le jeu, comme dans le remake : la croix et le stick
//! choisissent un bouton (il s'allume comme au survol de la souris), A le
//! clique (sans bouton choisi : un clic sans viser, pour la jauge, l'attaque
//! aérienne et les attaques spéciales), B ferme HOW TO PLAY ou retire le
//! choix, Start met en pause.

use super::Game;
use crate::display::{Kind, Obj};
use crate::geom::Rect;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

impl Game {
    /// Les boutons qu'on peut choisir : visibles, actifs, avec une action.
    fn focusable(&self) -> Vec<(Obj, Rect)> {
        let mut out = Vec::new();
        let mut stack = vec![self.d.root];
        while let Some(o) = stack.pop() {
            let Some(x) = self.d.get(o) else { continue };
            if !x.visible || x.clip_depth != 0 {
                continue;
            }
            let button = matches!(x.kind, Kind::Button(_)) || self.events.contains_key(&o);
            if button {
                if self.enabled(o) {
                    let b = self.d.bounds(o, self.d.world_matrix(o));
                    if !b.is_empty() {
                        out.push((o, b));
                    }
                }
                continue;
            }
            if matches!(x.kind, Kind::Clip(_)) {
                stack.extend(self.d.children(o));
            }
        }
        out
    }

    /// Le bouton choisi, s'il est toujours là.
    fn current_focus(&mut self) -> Option<(Obj, Rect)> {
        let f = self.focus?;
        let found = self.focusable().into_iter().find(|c| c.0 == f);
        if found.is_none() {
            self.focus = None;
        }
        found
    }

    /// Croix ou stick : le bouton le plus proche dans cette direction (le
    /// plus près du coin haut-gauche s'il n'y en a pas de choisi).
    pub fn pad_nav(&mut self, dir: Dir) {
        let list = self.focusable();
        let center = |r: &Rect| ((r.x_min + r.x_max) / 2.0, (r.y_min + r.y_max) / 2.0);
        let next = match self.current_focus() {
            None => list.iter().min_by(|a, b| {
                let (ax, ay) = center(&a.1);
                let (bx, by) = center(&b.1);
                (ax + ay).partial_cmp(&(bx + by)).unwrap_or(std::cmp::Ordering::Equal)
            }),
            Some((f, r)) => {
                let (fx, fy) = center(&r);
                list.iter()
                    .filter(|c| c.0 != f)
                    .filter_map(|c| {
                        let (cx, cy) = center(&c.1);
                        let (dx, dy) = (cx - fx, cy - fy);
                        // L'écart dans la direction, et l'écart de côté qui
                        // compte moitié ; à gauche et à droite, seulement à
                        // peu près sur la même ligne.
                        let (along, side) = match dir {
                            Dir::Up => (-dy, dx),
                            Dir::Down => (dy, dx),
                            Dir::Left => (-dx, dy),
                            Dir::Right => (dx, dy),
                        };
                        let horizontal = matches!(dir, Dir::Left | Dir::Right);
                        (along > 1.0 && (!horizontal || side.abs() <= along / 2.0)).then_some((c, along + 0.5 * side.abs()))
                    })
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
                    .map(|(c, _)| c)
            }
        };
        if let Some(&(o, _)) = next {
            self.focus = Some(o);
            self.set_hover(Some(o));
            self.run_scripts();
        }
    }

    /// A : appuyé, relâché.
    pub fn pad_confirm(&mut self, down: bool) {
        if down {
            let focus = self.current_focus().map(|f| f.0);
            self.set_hover(focus);
            self.mouse_down();
        } else {
            self.mouse_up();
        }
    }

    /// B : ferme HOW TO PLAY s'il est ouvert, sinon retire le choix.
    pub fn pad_back(&mut self) {
        let howto = self.child(self.d.root, "title_mc").and_then(|t| self.child(t, "howto_mc"));
        if let Some(h) = howto
            && self.d.visible(h)
            && let Some(panel) = self.child(h, "panel_mc")
        {
            self.button_action(148, panel);
            self.run_scripts();
            return;
        }
        self.focus = None;
        self.set_hover(None);
        self.run_scripts();
    }

    /// Start : pause, ou reprise, pendant la partie (le bouton PAUSE).
    pub fn pad_pause(&mut self) {
        let Some(menu) = self.hud.menu.filter(|&m| self.d.exists(m)) else { return };
        if self.pause_available(menu) {
            self.button_action(448, menu);
            self.run_scripts();
        }
    }
}
