//! Le graphe de scène, comme CCNode de Cocos2d 2.x : position, point
//! d'ancrage, échelle, rotation (sens horaire), inclinaison, ordre z,
//! étiquette, opacité et couleur (transmises aux enfants si la cascade est
//! active), et ce que le nœud dessine.

use std::rc::Rc;

use crate::action::Running;
use crate::assets::{Assets, CONTENT_SCALE, Font, Frame};
use crate::geom::Affine;
use crate::render::Blend;
use crate::text::{TextImage, TextStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct NodeId {
    index: u32,
    generation: u32,
}

/// Une image : un fichier des ressources (chemin résolu) ou une image
/// fabriquée (texte).
#[derive(Clone)]
pub enum Image {
    File(Rc<str>),
    Text(Rc<TextImage>),
}

#[derive(Clone)]
pub struct Sprite {
    pub image: Option<Image>,
    /// L'image de feuille affichée, s'il y en a une.
    pub frame: Option<Rc<Frame>>,
    /// Rectangle dans la texture, en pixels (largeur, hauteur non tournées).
    pub rect: [f32; 4],
    pub rotated: bool,
    /// Coin bas gauche du rectangle dans le nœud, en points (rognage).
    pub offset: (f32, f32),
    /// Décalage du centre (images rognées), en points, avant retournement.
    pub center_offset: (f32, f32),
    pub flip_x: bool,
    pub flip_y: bool,
    pub blend: Blend,
}

/// Une couche de couleur (CCLayerColor) ou un dégradé (CCLayerGradient) :
/// couleurs des coins bas gauche, bas droite, haut gauche, haut droite.
#[derive(Clone)]
pub struct ColorLayer {
    pub corners: [[u8; 4]; 4],
    pub blend: Blend,
}

/// Une étiquette BMFont (CCLabelBMFont). Ses lettres sont des sprites
/// enfants, étiquetés par leur rang dans le texte, que la logique peut
/// déplacer une à une.
#[derive(Clone)]
pub struct BmLabel {
    pub font: Rc<Font>,
    pub text: String,
    /// 0 gauche, 1 centre, 2 droite.
    pub align: u8,
}

/// Une image étirée en neuf parties (CCScale9Sprite) : coins fixes, bords
/// et centre étirés.
#[derive(Clone)]
pub struct Scale9 {
    pub sprite: Sprite,
    /// Taille de l'image en points.
    pub original: (f32, f32),
    /// Le centre étiré (x, y, largeur, hauteur en points, y vers le bas
    /// comme dans Cocos2d).
    pub insets: [f32; 4],
}

/// Un bouton de menu (CCMenuItem) : l'action est gardée par le jeu.
#[derive(Clone, Copy)]
pub struct MenuItem {
    pub enabled: bool,
    pub selected: bool,
}

/// Une étiquette à cases (CCLabelAtlas).
#[derive(Clone)]
pub struct AtlasLabel {
    pub image: Rc<str>,
    pub text: String,
    /// Taille d'une case en points, et le premier caractère.
    pub item: (f32, f32),
    pub start: char,
}

/// Une barre de progression (CCProgressTimer en barre, le seul employé).
#[derive(Clone)]
pub struct Progress {
    pub sprite: Sprite,
    pub percent: f32,
    pub midpoint: (f32, f32),
    pub bar_rate: (f32, f32),
}

#[derive(Clone)]
pub enum Kind {
    Node,
    Sprite(Sprite),
    Color(ColorLayer),
    Label(BmLabel),
    Atlas(AtlasLabel),
    Progress(Progress),
    Scale9(Scale9),
    /// Polygones (CCDrawNode) : triangles et leur couleur (non
    /// prémultipliée).
    Draw(Vec<([(f32, f32); 3], [f32; 4])>),
    /// Menu (CCMenu) : ses enfants sont des boutons.
    Menu { enabled: bool },
    Item(MenuItem),
    /// Découpe (CCClippingNode) : le nœud dont la forme découpe.
    Clip { stencil: Option<NodeId>, inverted: bool },
    Armature(Box<crate::armature::Armature>),
}

#[derive(Clone)]
pub struct Node {
    pub parent: Option<NodeId>,
    pub children: Vec<NodeId>,
    pub tag: i32,
    pub z: i32,
    arrival: u32,
    pub pos: (f32, f32),
    pub anchor: (f32, f32),
    pub size: (f32, f32),
    /// Armatures : le coin du rectangle englobant vu de l'origine
    /// (m_pOffsetPoint), retiré du point d'ancrage.
    pub offset: (f32, f32),
    /// Couches et scènes : la position désigne le coin, pas l'ancre.
    pub ignore_anchor: bool,
    pub scale: (f32, f32),
    /// Rotation en degrés, sens horaire (x et y, comme CCNode).
    pub rotation: (f32, f32),
    pub skew: (f32, f32),
    pub visible: bool,
    /// Couleur et opacité données au nœud (réelles)…
    pub color: [u8; 3],
    pub opacity: u8,
    /// … et affichées : multipliées par celles du parent quand il les
    /// transmet, au moment où elles changent (CCNodeRGBA de Cocos2d 2.x).
    pub displayed_color: [u8; 3],
    pub displayed_opacity: u8,
    pub cascade_color: bool,
    pub cascade_opacity: bool,
    pub kind: Kind,
    pub actions: Vec<Running>,
    /// Actions et mises à jour suspendues (pauseSchedulerAndActions).
    pub paused: bool,
}

impl Node {
    pub fn new(kind: Kind) -> Node {
        Node {
            parent: None,
            children: Vec::new(),
            tag: -1,
            z: 0,
            arrival: 0,
            pos: (0.0, 0.0),
            anchor: (0.0, 0.0),
            size: (0.0, 0.0),
            offset: (0.0, 0.0),
            ignore_anchor: false,
            scale: (1.0, 1.0),
            rotation: (0.0, 0.0),
            skew: (0.0, 0.0),
            visible: true,
            color: [255; 3],
            opacity: 255,
            displayed_color: [255; 3],
            displayed_opacity: 255,
            cascade_color: false,
            cascade_opacity: false,
            kind,
            actions: Vec::new(),
            paused: false,
        }
    }

    pub fn set_rotation(&mut self, degrees: f32) {
        self.rotation = (degrees, degrees);
    }

    pub fn set_scale(&mut self, s: f32) {
        self.scale = (s, s);
    }

    /// Le point d'ancrage en points.
    pub fn anchor_points(&self) -> (f32, f32) {
        (self.anchor.0 * self.size.0 - self.offset.0, self.anchor.1 * self.size.1 - self.offset.1)
    }

    /// Du nœud vers son parent (CCNode::nodeToParentTransform).
    pub fn transform(&self) -> Affine {
        let (ax, ay) = self.anchor_points();
        let (mut x, mut y) = self.pos;
        if self.ignore_anchor {
            x += ax;
            y += ay;
        }
        let (cx, sx) = if self.rotation.0 != 0.0 { let r = -self.rotation.0.to_radians(); (r.cos(), r.sin()) } else { (1.0, 0.0) };
        let (cy, sy) = if self.rotation.1 != 0.0 { let r = -self.rotation.1.to_radians(); (r.cos(), r.sin()) } else { (1.0, 0.0) };
        let skew = self.skew.0 != 0.0 || self.skew.1 != 0.0;
        let (scx, scy) = self.scale;
        if !skew && (ax != 0.0 || ay != 0.0) {
            x += cy * -ax * scx + -sx * -ay * scy;
            y += sy * -ax * scx + cx * -ay * scy;
        }
        let mut t = Affine { a: cy * scx, b: sy * scx, c: -sx * scy, d: cx * scy, tx: x, ty: y };
        if skew {
            let k = Affine { a: 1.0, b: self.skew.1.to_radians().tan(), c: self.skew.0.to_radians().tan(), d: 1.0, tx: 0.0, ty: 0.0 };
            t = k.then(&t);
            if ax != 0.0 || ay != 0.0 {
                t = t.translate(-ax, -ay);
            }
        }
        t
    }
}

impl Sprite {
    /// Un sprite d'une image entière (CCSprite::create).
    pub fn file(path: Rc<str>, width: f32, height: f32) -> Sprite {
        Sprite { image: Some(Image::File(path)), frame: None, rect: [0.0, 0.0, width, height], rotated: false, offset: (0.0, 0.0), center_offset: (0.0, 0.0), flip_x: false, flip_y: false, blend: Blend::Normal }
    }

    /// Un sprite sans image.
    pub fn empty() -> Sprite {
        Sprite { image: None, ..Sprite::file("".into(), 0.0, 0.0) }
    }

    /// Un sprite d'une image de feuille, et sa taille en points.
    pub fn frame(frame: &Rc<Frame>) -> (Sprite, (f32, f32)) {
        let mut s = Sprite::empty();
        let size = s.set_frame(frame);
        (s, size)
    }

    /// Change d'image (setDisplayFrame) ; rend la nouvelle taille.
    pub fn set_frame(&mut self, frame: &Rc<Frame>) -> (f32, f32) {
        self.frame = Some(frame.clone());
        self.image = Some(Image::File(frame.texture.as_str().into()));
        self.rect = frame.rect;
        self.rotated = frame.rotated;
        self.center_offset = (frame.offset.0 / CONTENT_SCALE, frame.offset.1 / CONTENT_SCALE);
        let size = (frame.source.0 / CONTENT_SCALE, frame.source.1 / CONTENT_SCALE);
        self.place(size);
        size
    }

    /// Le coin de l'image dans le nœud (setTextureRect).
    pub fn place(&mut self, size: (f32, f32)) {
        let (w, h) = (self.rect[2] / CONTENT_SCALE, self.rect[3] / CONTENT_SCALE);
        let (mut ox, mut oy) = self.center_offset;
        if self.flip_x {
            ox = -ox;
        }
        if self.flip_y {
            oy = -oy;
        }
        self.offset = (ox + (size.0 - w) / 2.0, oy + (size.1 - h) / 2.0);
    }
}

/// Une lettre placée : son rang dans le texte, son rectangle dans la page
/// (pixels) et son centre (points).
struct Letter {
    tag: i32,
    rect: [f32; 4],
    center: (f32, f32),
}

/// La mise en page d'un texte (CCLabelBMFont::createFontChars puis
/// l'alignement d'updateLabel) ; rend les lettres et la taille.
fn layout(font: &Font, text: &str, align: u8) -> (Vec<Letter>, (f32, f32)) {
    let common = font.line_height;
    let chars: Vec<char> = text.chars().collect();
    let lines = 1 + chars.iter().filter(|&&c| c == '\n').count();
    let (mut x, mut y) = (0.0f32, -(common - common * lines as f32));
    let mut longest = 0.0f32;
    let mut prev = None;
    let mut letters = Vec::new();
    for (i, &c) in chars.iter().enumerate() {
        if c == '\n' {
            x = 0.0;
            y -= common;
            continue;
        }
        let kerning = prev.and_then(|p| font.kerning.get(&(p, c)).copied()).unwrap_or(0.0);
        let Some(g) = font.glyphs.get(&c) else { continue };
        let center = (x + g.xoffset + g.rect[2] * 0.5 + kerning, y + (common - g.yoffset) - g.rect[3] * 0.5);
        letters.push(Letter { tag: i as i32, rect: g.rect, center: (center.0 / CONTENT_SCALE, center.1 / CONTENT_SCALE) });
        x += g.xadvance + kerning;
        longest = longest.max(x);
        prev = Some(c);
    }
    let size = (longest / CONTENT_SCALE, common * lines as f32 / CONTENT_SCALE);
    if align != 0 {
        // Chaque ligne décalée d'après le bord droit de sa dernière lettre.
        let mut start = 0;
        let mut line_start_tag = 0;
        for line in text.split('\n') {
            let n = line.chars().count() as i32;
            let end_tag = line_start_tag + n;
            let end = start + letters[start..].iter().take_while(|l| l.tag < end_tag).count();
            if let Some(last) = letters[start..end].last() {
                let width = last.center.0 + last.rect[2] / CONTENT_SCALE / 2.0;
                let shift = if align == 1 { size.0 / 2.0 - width / 2.0 } else { size.0 - width };
                for l in &mut letters[start..end] {
                    l.center.0 += shift;
                }
            }
            start = end;
            line_start_tag = end_tag + 1;
        }
    }
    (letters, size)
}

impl AtlasLabel {
    pub fn size(&self) -> (f32, f32) {
        (self.text.chars().count() as f32 * self.item.0, self.item.1)
    }
}

struct Slot {
    generation: u32,
    node: Option<Node>,
}

/// Tous les nœuds, rangés par identifiant.
#[derive(Default)]
pub struct Graph {
    slots: Vec<Slot>,
    free: Vec<u32>,
    arrival: u32,
}

impl Graph {
    pub fn add(&mut self, node: Node) -> NodeId {
        if let Some(index) = self.free.pop() {
            let slot = &mut self.slots[index as usize];
            slot.node = Some(node);
            NodeId { index, generation: slot.generation }
        } else {
            self.slots.push(Slot { generation: 0, node: Some(node) });
            NodeId { index: self.slots.len() as u32 - 1, generation: 0 }
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&Node> {
        self.slots.get(id.index as usize).filter(|s| s.generation == id.generation).and_then(|s| s.node.as_ref())
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut Node> {
        self.slots.get_mut(id.index as usize).filter(|s| s.generation == id.generation).and_then(|s| s.node.as_mut())
    }

    pub fn alive(&self, id: NodeId) -> bool {
        self.get(id).is_some()
    }

    /// Ajoute `child` à `parent` (addChild) avec son ordre z et son étiquette.
    pub fn attach(&mut self, parent: NodeId, child: NodeId, z: i32, tag: i32) {
        self.detach(child);
        self.arrival += 1;
        let arrival = self.arrival;
        if let Some(c) = self.get_mut(child) {
            c.parent = Some(parent);
            c.z = z;
            c.tag = tag;
            c.arrival = arrival;
        }
        if let Some(p) = self.get_mut(parent) {
            p.children.push(child);
        }
    }

    /// Retire un nœud de son parent, sans le détruire.
    pub fn detach(&mut self, id: NodeId) {
        let Some(parent) = self.get(id).and_then(|n| n.parent) else { return };
        if let Some(p) = self.get_mut(parent) {
            p.children.retain(|&c| c != id);
        }
        if let Some(n) = self.get_mut(id) {
            n.parent = None;
        }
    }

    /// Détruit un nœud et ses descendants (removeFromParent).
    pub fn remove(&mut self, id: NodeId) {
        self.detach(id);
        let mut stack = vec![id];
        while let Some(n) = stack.pop() {
            let Some(slot) = self.slots.get_mut(n.index as usize).filter(|s| s.generation == n.generation) else { continue };
            if let Some(node) = slot.node.take() {
                stack.extend(node.children);
                if let Kind::Clip { stencil: Some(s), .. } = node.kind {
                    stack.push(s);
                }
                slot.generation += 1;
                self.free.push(n.index);
            }
        }
    }

    /// Le premier enfant à cette étiquette (getChildByTag).
    pub fn child(&self, id: NodeId, tag: i32) -> Option<NodeId> {
        self.get(id)?.children.iter().copied().find(|&c| self.get(c).is_some_and(|n| n.tag == tag))
    }

    /// Change l'ordre z (reorderChild).
    pub fn reorder(&mut self, id: NodeId, z: i32) {
        self.arrival += 1;
        let arrival = self.arrival;
        if let Some(n) = self.get_mut(id) {
            n.z = z;
            n.arrival = arrival;
        }
    }

    /// Les enfants dans l'ordre de dessin (z, puis arrivée).
    pub fn sorted_children(&self, id: NodeId) -> Vec<NodeId> {
        let mut c = self.get(id).map(|n| n.children.clone()).unwrap_or_default();
        c.sort_by_key(|&k| self.get(k).map_or((0, 0), |n| (n.z, n.arrival)));
        c
    }

    /// Du nœud vers la scène (nodeToWorldTransform).
    pub fn world(&self, id: NodeId) -> Affine {
        let mut t = Affine::IDENTITY;
        let mut cur = Some(id);
        while let Some(n) = cur.and_then(|c| self.get(c)) {
            t = t.then(&n.transform());
            cur = n.parent;
        }
        t
    }

    /// Visible, ainsi que tous ses parents.
    pub fn shown(&self, id: NodeId) -> bool {
        let mut cur = Some(id);
        while let Some(n) = cur.and_then(|c| self.get(c)) {
            if !n.visible {
                return false;
            }
            cur = n.parent;
        }
        true
    }

    /// Un point de la scène est-il dans le rectangle du nœud ?
    pub fn contains(&self, id: NodeId, point: (f32, f32)) -> bool {
        let Some(n) = self.get(id) else { return false };
        let (x, y) = self.world(id).invert().apply(point.0, point.1);
        x >= 0.0 && y >= 0.0 && x < n.size.0 && y < n.size.1
    }

    /// setOpacity : l'opacité réelle et affichée ; si le nœud la transmet,
    /// celle du parent (s'il la transmet aussi) s'applique, puis descend.
    pub fn set_opacity(&mut self, id: NodeId, opacity: u8) {
        let Some(n) = self.get_mut(id) else { return };
        n.opacity = opacity;
        n.displayed_opacity = opacity;
        if n.cascade_opacity {
            let parent = n.parent.and_then(|p| self.get(p)).filter(|p| p.cascade_opacity).map_or(255, |p| p.displayed_opacity);
            self.update_displayed_opacity(id, parent);
        }
    }

    /// updateDisplayedOpacity.
    pub fn update_displayed_opacity(&mut self, id: NodeId, parent: u8) {
        let Some(n) = self.get_mut(id) else { return };
        n.displayed_opacity = (u32::from(n.opacity) * u32::from(parent) / 255) as u8;
        if n.cascade_opacity {
            let (shown, children) = (n.displayed_opacity, n.children.clone());
            for c in children {
                self.update_displayed_opacity(c, shown);
            }
        }
    }

    /// setColor, de la même façon que l'opacité.
    pub fn set_color(&mut self, id: NodeId, color: [u8; 3]) {
        let Some(n) = self.get_mut(id) else { return };
        n.color = color;
        n.displayed_color = color;
        if n.cascade_color {
            let parent = n.parent.and_then(|p| self.get(p)).filter(|p| p.cascade_color).map_or([255; 3], |p| p.displayed_color);
            self.update_displayed_color(id, parent);
        }
    }

    /// updateDisplayedColor.
    pub fn update_displayed_color(&mut self, id: NodeId, parent: [u8; 3]) {
        let Some(n) = self.get_mut(id) else { return };
        for i in 0..3 {
            n.displayed_color[i] = (u32::from(n.color[i]) * u32::from(parent[i]) / 255) as u8;
        }
        if n.cascade_color {
            let (shown, children) = (n.displayed_color, n.children.clone());
            for c in children {
                self.update_displayed_color(c, shown);
            }
        }
    }

    /// Tous les nœuds vivants (actions).
    pub fn ids(&self) -> Vec<NodeId> {
        self.slots.iter().enumerate().filter(|(_, s)| s.node.is_some()).map(|(i, s)| NodeId { index: i as u32, generation: s.generation }).collect()
    }
}

// --- Fabriques ---------------------------------------------------------------

impl Graph {
    pub fn node(&mut self) -> NodeId {
        self.add(Node::new(Kind::Node))
    }

    /// Une couche plein écran (CCLayer) : ancre au centre, ignorée.
    pub fn layer(&mut self, size: (f32, f32)) -> NodeId {
        let mut n = Node::new(Kind::Node);
        n.size = size;
        n.anchor = (0.5, 0.5);
        n.ignore_anchor = true;
        self.add(n)
    }

    /// Une couche de couleur unie (CCLayerColor).
    pub fn color_layer(&mut self, rgba: [u8; 4], size: (f32, f32)) -> NodeId {
        let mut n = Node::new(Kind::Color(ColorLayer { corners: [[255; 4]; 4], blend: Blend::Normal }));
        n.size = size;
        n.anchor = (0.5, 0.5);
        n.ignore_anchor = true;
        n.color = [rgba[0], rgba[1], rgba[2]];
        n.opacity = rgba[3];
        n.displayed_color = n.color;
        n.displayed_opacity = n.opacity;
        self.add(n)
    }

    /// Un dégradé vertical (CCLayerGradient, vecteur (0, -1)) : `start` en
    /// haut, `end` en bas.
    pub fn gradient_layer(&mut self, start: [u8; 4], end: [u8; 4], size: (f32, f32)) -> NodeId {
        let id = self.color_layer([255; 4], size);
        if let Some(n) = self.get_mut(id)
            && let Kind::Color(c) = &mut n.kind
        {
            c.corners = [end, end, start, start];
        }
        id
    }

    /// Un sprite d'une image de feuille (createWithSpriteFrameName).
    pub fn sprite_frame(&mut self, assets: &Assets, name: &str) -> NodeId {
        let mut n = Node::new(Kind::Node);
        n.anchor = (0.5, 0.5);
        if let Some(f) = assets.frame(name) {
            let (s, size) = Sprite::frame(&f);
            n.kind = Kind::Sprite(s);
            n.size = size;
        } else {
            n.kind = Kind::Sprite(Sprite::empty());
        }
        self.add(n)
    }

    /// Un sprite d'une image entière des ressources (CCSprite::create).
    pub fn sprite_file(&mut self, assets: &Assets, name: &str) -> NodeId {
        let mut n = Node::new(Kind::Sprite(Sprite::empty()));
        n.anchor = (0.5, 0.5);
        if let Some(path) = assets.resolve(name)
            && let Some((w, h)) = assets.image_size(&path)
        {
            n.kind = Kind::Sprite(Sprite::file(path.into(), w, h));
            n.size = (w / CONTENT_SCALE, h / CONTENT_SCALE);
        }
        self.add(n)
    }

    /// Une étiquette BMFont, ancrée au centre.
    pub fn bm_label(&mut self, assets: &Assets, text: &str, font: &str, align: u8) -> NodeId {
        let mut n = Node::new(Kind::Node);
        n.anchor = (0.5, 0.5);
        n.cascade_color = true;
        n.cascade_opacity = true;
        if let Some(f) = assets.font(font) {
            n.kind = Kind::Label(BmLabel { font: f, text: String::new(), align });
        }
        let id = self.add(n);
        self.set_text(id, text);
        id
    }

    /// Les lettres d'une étiquette BMFont : toutes cachées, puis celles du
    /// texte réutilisées (ou créées) et replacées.
    fn place_letters(&mut self, id: NodeId, text: &str) {
        let Some(Kind::Label(l)) = self.get(id).map(|n| &n.kind) else { return };
        let (font, align) = (l.font.clone(), l.align);
        let (letters, size) = layout(&font, text, align);
        let page: Rc<str> = font.page.as_str().into();
        for c in self.get(id).map(|n| n.children.clone()).unwrap_or_default() {
            if let Some(n) = self.get_mut(c) {
                n.visible = false;
            }
        }
        for l in letters {
            let child = match self.child(id, l.tag) {
                Some(c) => c,
                None => {
                    // Nouvelle lettre : couleur et opacité de l'étiquette.
                    let mut s = Node::new(Kind::Sprite(Sprite::empty()));
                    s.anchor = (0.5, 0.5);
                    let c = self.add(s);
                    self.attach(id, c, l.tag, l.tag);
                    let (color, opacity) = self.get(id).map_or(([255; 3], 255), |n| (n.displayed_color, n.displayed_opacity));
                    self.update_displayed_color(c, color);
                    self.update_displayed_opacity(c, opacity);
                    c
                }
            };
            if let Some(n) = self.get_mut(child) {
                let mut s = Sprite::file(page.clone(), l.rect[2], l.rect[3]);
                s.rect = l.rect;
                n.size = (l.rect[2] / CONTENT_SCALE, l.rect[3] / CONTENT_SCALE);
                s.place(n.size);
                n.kind = Kind::Sprite(s);
                n.pos = l.center;
                n.visible = true;
            }
        }
        if let Some(n) = self.get_mut(id) {
            n.size = size;
            if let Kind::Label(l) = &mut n.kind {
                l.text = text.to_string();
            }
        }
    }

    /// Une étiquette à cases (CCLabelAtlas::create).
    pub fn atlas_label(&mut self, assets: &Assets, text: &str, image: &str, item: (f32, f32), start: char) -> NodeId {
        let path: Rc<str> = assets.resolve(image).unwrap_or_default().into();
        let l = AtlasLabel { image: path, text: text.to_string(), item, start };
        let mut n = Node::new(Kind::Node);
        n.size = l.size();
        n.kind = Kind::Atlas(l);
        self.add(n)
    }

    /// Une étiquette en police système (CCLabelTTF), ancrée au centre.
    pub fn ttf_label(&mut self, text: &str, style: &TextStyle) -> NodeId {
        let mut n = Node::new(Kind::Sprite(Sprite::empty()));
        n.anchor = (0.5, 0.5);
        let id = self.add(n);
        self.set_ttf_text(id, text, style);
        id
    }

    /// Une image en neuf parties à la taille donnée ; sans marges, le centre
    /// est le tiers du milieu.
    pub fn scale9(&mut self, assets: &Assets, image: &str, insets: Option<[f32; 4]>, size: (f32, f32)) -> NodeId {
        let source = self.sprite_frame(assets, image);
        let source = if self.get(source).is_some_and(|n| matches!(&n.kind, Kind::Sprite(s) if s.image.is_some())) {
            source
        } else {
            self.remove(source);
            self.sprite_file(assets, image)
        };
        let (kind, original) = match self.get(source) {
            Some(Node { kind: Kind::Sprite(s), size, .. }) => (s.clone(), *size),
            _ => (Sprite::empty(), (0.0, 0.0)),
        };
        self.remove(source);
        let insets = insets.unwrap_or([original.0 / 3.0, original.1 / 3.0, original.0 / 3.0, original.1 / 3.0]);
        let mut n = Node::new(Kind::Scale9(Scale9 { sprite: kind, original, insets }));
        n.size = size;
        n.anchor = (0.5, 0.5);
        self.add(n)
    }

    /// Un nœud de polygones vide (CCDrawNode).
    pub fn draw_node(&mut self) -> NodeId {
        self.add(Node::new(Kind::Draw(Vec::new())))
    }

    /// Change le texte d'une étiquette en police système.
    pub fn set_ttf_text(&mut self, id: NodeId, text: &str, style: &TextStyle) {
        let Some(n) = self.get_mut(id) else { return };
        let Kind::Sprite(s) = &mut n.kind else { return };
        let image = Rc::new(crate::text::render(text, style));
        s.rect = [0.0, 0.0, image.width as f32, image.height as f32];
        n.size = image.size();
        s.image = Some(Image::Text(image));
        s.place(n.size);
    }

    /// Change le texte d'une étiquette (setString).
    pub fn set_text(&mut self, id: NodeId, text: &str) {
        let Some(n) = self.get_mut(id) else { return };
        match &mut n.kind {
            Kind::Label(_) => self.place_letters(id, text),
            Kind::Atlas(l) => {
                l.text = text.to_string();
                n.size = l.size();
            }
            _ => {}
        }
    }

    /// Change l'image d'un sprite (setDisplayFrame).
    pub fn set_frame(&mut self, assets: &Assets, id: NodeId, name: &str) {
        let Some(f) = assets.frame(name) else { return };
        if let Some(n) = self.get_mut(id)
            && let Kind::Sprite(s) = &mut n.kind
        {
            n.size = s.set_frame(&f);
        }
    }
}
