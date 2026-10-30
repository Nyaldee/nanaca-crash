//! La boîte de dialogue du tutoriel (MessageBoxLayer et son
//! ScriptProcessor) : Miki parle, une lettre par image ; « $fN » change son
//! visage, « $tN » la mise en page (1 centré, 2 centré en grand, 3 en
//! petit). Le texte fini, un poignard tourne en bas à droite.

use crate::action::Action;
use crate::game::Game;
use crate::node::NodeId;
use crate::text::TextStyle;

/// La boîte et le texte en cours.
pub struct MsgBox {
    pub layer: NodeId,
    back: NodeId,
    label: NodeId,
    face: NodeId,
    dagger: NodeId,
    size: (f32, f32),
    with_face: bool,
    /// Le script : ses lignes, la ligne et l'octet en cours, le texte
    /// déjà affiché.
    lines: Vec<String>,
    line: usize,
    cursor: usize,
    text: String,
    /// Alignements (horizontal, vertical) et taille de la police.
    style: (u8, u8, f32),
    running: bool,
    /// Le texte est entièrement affiché (0x194).
    pub finished: bool,
}

impl MsgBox {
    /// MessageBoxLayer::create puis onEnter : 400 × 125, transparent, en
    /// haut au milieu de l'écran.
    pub fn new(game: &mut Game, scene: NodeId) -> MsgBox {
        let (w, h) = game.design;
        let size = (400.0, 125.0);
        let layer = game.g.color_layer([0, 0, 0, 0], size);
        if let Some(n) = game.g.get_mut(layer) {
            n.anchor = (0.5, 0.5);
            n.ignore_anchor = true;
        }
        game.set_pos(layer, (w - 400.0) * 0.5, h * 0.6);
        game.add(scene, layer, 8000, 8000);
        let assets = game.assets.clone();
        let back = game.g.scale9(&assets, "mblayer.png", Some([4.0, 4.0, 120.0, 120.0]), size);
        game.set_anchor(back, 0.0, 0.0);
        game.add(layer, back, 0x32, 0x32);
        let label = game.g.ttf_label("", &TextStyle { font: "Arial", size: 18.0, dimensions: (0.0, 0.0), h_align: 0, v_align: 0 });
        game.set_color(label, [0xFF; 3]);
        game.set_anchor(label, 0.0, 0.0);
        game.add(layer, label, 100, 100);
        let face = face_sprite(game, 1);
        game.add(layer, face, 200, 200);
        let dagger = game.sprite("dagger.png");
        game.set_rotation(dagger, 30.0);
        game.set_visible(dagger, false);
        game.add(layer, dagger, 300, 300);
        let mut b = MsgBox {
            layer,
            back,
            label,
            face,
            dagger,
            size,
            with_face: true,
            lines: Vec::new(),
            line: 0,
            cursor: 0,
            text: String::new(),
            style: (0, 0, 18.0),
            running: false,
            finished: false,
        };
        b.resize(game, size, true);
        b
    }

    /// changeWindowSize : la taille, avec ou sans le visage de Miki.
    pub fn resize(&mut self, game: &mut Game, size: (f32, f32), with_face: bool) {
        self.size = size;
        self.with_face = with_face;
        if let Some(n) = game.g.get_mut(self.layer) {
            n.size = size;
        }
        if let Some(n) = game.g.get_mut(self.back) {
            n.size = size;
        }
        game.set_y(self.back, 0.0);
        game.set_pos(self.dagger, size.0 - 20.0, 20.0);
        game.set_visible(self.face, with_face);
        let x = if with_face { 110.0 } else { 10.0 };
        game.set_pos(self.label, x, 10.0);
        self.render(game);
    }

    /// La hauteur de la boîte à l'écran (setPositionY).
    pub fn set_y(&self, game: &mut Game, y: f32) {
        game.set_y(self.layer, y);
    }

    pub fn show(&self, game: &mut Game, on: bool) {
        game.set_visible(self.layer, on);
    }

    /// L'apparition : de rien à sa taille en 0.2 s.
    pub fn pop_in(&self, game: &mut Game) {
        game.set_scale(self.layer, 0.0);
        game.run(self.layer, Action::scale_to(0.2, 1.0));
    }

    /// setScript : un nouveau texte (lignes nettoyées, vides retirées),
    /// affiché depuis le début.
    pub fn set_script(&mut self, game: &mut Game, script: &str) {
        self.lines = script.split('\n').map(str::trim).filter(|l| !l.is_empty()).map(String::from).collect();
        self.line = 0;
        self.cursor = 0;
        self.text.clear();
        game.set_visible(self.dagger, false);
        game.stop_actions(self.dagger);
        self.finished = false;
        self.style = (0, 0, 18.0);
        self.running = true;
        self.render(game);
    }

    /// cbUpdateOnPeriod (chaque image) : une lettre ou une commande.
    pub fn update(&mut self, game: &mut Game) {
        if !self.running {
            return;
        }
        let more = match self.lines.get(self.line) {
            Some(line) => {
                let rest = &line[self.cursor..];
                let mut command = None;
                let consumed = if let Some(cmd) = rest.strip_prefix('$') {
                    // Une commande : « $ », sa lettre, son chiffre.
                    let mut chars = cmd.chars();
                    let kind = chars.next();
                    let value = chars.next().and_then(|c| c.to_digit(10)).unwrap_or(0) as i32;
                    command = kind.map(|k| (k, value));
                    1 + kind.map_or(0, char::len_utf8) + cmd.chars().nth(1).map_or(0, char::len_utf8)
                } else {
                    let c = rest.chars().next().unwrap_or(' ');
                    self.text.push(c);
                    c.len_utf8()
                };
                self.cursor += consumed;
                match command {
                    Some(('f', n)) => {
                        // Le visage remplacé (visible, comme l'original).
                        game.g.remove(self.face);
                        self.face = face_sprite(game, n);
                        game.add(self.layer, self.face, 200, 200);
                    }
                    Some(('t', 1)) => self.style.0 = 1,
                    Some(('t', 2)) => self.style = (1, 1, 24.0),
                    Some(('t', 3)) => self.style.2 = 14.0,
                    _ => {}
                }
                let len = self.lines[self.line].len();
                if self.cursor >= len {
                    self.line += 1;
                    if self.line < self.lines.len() {
                        self.cursor = 0;
                        self.text.push('\n');
                        true
                    } else {
                        false
                    }
                } else {
                    true
                }
            }
            None => false,
        };
        self.render(game);
        if !more {
            // Fini : le poignard tourne sur lui-même.
            self.running = false;
            game.set_visible(self.dagger, true);
            game.run(self.dagger, Action::forever(Action::orbit(2.5, 0.0, 180.0)));
            self.finished = true;
        }
    }

    /// Le texte affiché, dans la place laissée par le visage.
    fn render(&self, game: &mut Game) {
        let (w, h) = self.size;
        let dims = if self.with_face { (w - 120.0, h - 20.0) } else { (w - 20.0, h - 20.0) };
        let (h_align, v_align, size) = self.style;
        game.g.set_ttf_text(self.label, &self.text, &TextStyle { font: "Arial", size, dimensions: dims, h_align, v_align });
    }
}

/// Le visage de Miki « mikifaceN.png », en bas à gauche de la boîte.
fn face_sprite(game: &mut Game, n: i32) -> NodeId {
    let face = game.sprite(&format!("mikiface{n}.png"));
    game.set_anchor(face, 0.0, 0.0);
    game.set_pos(face, 10.0, 10.0);
    face
}
