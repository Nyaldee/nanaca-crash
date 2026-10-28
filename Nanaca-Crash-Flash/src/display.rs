//! La liste d'affichage de Flash : des objets (clips, formes, textes,
//! boutons) rangés par profondeur sous leur parent, et les timelines des
//! clips qui les placent, déplacent et retirent image par image.
//!
//! Le lecteur ne connaît pas la logique du jeu : il signale chaque image
//! affichée (`frames`) et les sons à lancer (`sounds`), que la logique Rust
//! traite ensuite dans l'ordre de Flash.

use std::collections::BTreeMap;
use std::rc::Rc;

use crate::geom::{ColorTransform, Matrix, Rect};
use crate::library::{BUTTON_DOWN, BUTTON_HIT, BUTTON_OVER, BUTTON_UP, ButtonRecord, Character, Command, Frame, Id, Library, Place, StartSound, Timeline};

/// Un objet affiché ; les identifiants ne resservent jamais.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Obj(pub usize);

pub struct Object {
    pub parent: Option<Obj>,
    pub depth: i32,
    /// None : un clip vide (createEmptyMovieClip).
    pub character: Option<Id>,
    pub name: String,
    pub matrix: Matrix,
    pub color: ColorTransform,
    pub visible: bool,
    /// Un masque sur les profondeurs au-dessus, jusqu'à celle-ci.
    pub clip_depth: u16,
    /// Étape d'une forme qui se transforme.
    pub ratio: u16,
    /// Placé par une timeline (et non par le code).
    pub from_timeline: bool,
    /// Transformé par le code : la timeline ne le déplace plus.
    pub scripted: bool,
    /// Échelles (%) et rotation (degrés) données par le code, gardées
    /// telles quelles comme Flash (None : tirées de la matrice).
    pub transform: Option<(f64, f64, f64)>,
    pub kind: Kind,
}

pub enum Kind {
    Clip(Clip),
    Graphic,
    Text(TextField),
    Button(Button),
}

pub struct TextField {
    pub text: String,
    /// autoSize = "right" : le texte s'étend vers la gauche.
    pub autosize_right: bool,
}

pub struct Clip {
    /// Image affichée (à partir de 1 ; 0 avant la première).
    pub frame: u16,
    pub playing: bool,
    pub children: BTreeMap<i32, Obj>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonState {
    Up,
    Over,
    Down,
}

impl ButtonState {
    pub fn flag(self) -> u8 {
        match self {
            ButtonState::Up => BUTTON_UP,
            ButtonState::Over => BUTTON_OVER,
            ButtonState::Down => BUTTON_DOWN,
        }
    }
}

/// Un bouton : les objets de ses états (avec les états où ils
/// apparaissent), et son état courant.
pub struct Button {
    pub state: ButtonState,
    pub children: Vec<(u8, Obj)>,
}

pub struct Display {
    pub lib: Rc<Library>,
    objects: Vec<Option<Object>>,
    pub root: Obj,
    /// Images affichées depuis le dernier passage de la logique, dans
    /// l'ordre où leurs scripts tournaient dans Flash.
    pub frames: Vec<(Obj, u16)>,
    /// Sons lancés par les timelines (avec leur clip).
    pub sounds: Vec<(Obj, StartSound)>,
    /// Clips créés et retirés depuis le dernier passage de la logique.
    pub created: Vec<Obj>,
    pub removed: Vec<Obj>,
    /// Clips sans nom placés jusqu'ici (leurs noms automatiques).
    instances: u32,
}

impl Display {
    pub fn new(lib: Rc<Library>) -> Display {
        let mut d = Display { lib, objects: Vec::new(), root: Obj(0), frames: Vec::new(), sounds: Vec::new(), created: Vec::new(), removed: Vec::new(), instances: 0 };
        d.root = d.add(Object::new(None, 0, None, Kind::Clip(Clip { frame: 0, playing: true, children: BTreeMap::new() })));
        d
    }

    pub fn get(&self, o: Obj) -> Option<&Object> {
        self.objects.get(o.0).and_then(Option::as_ref)
    }

    pub fn get_mut(&mut self, o: Obj) -> Option<&mut Object> {
        self.objects.get_mut(o.0).and_then(Option::as_mut)
    }

    pub fn exists(&self, o: Obj) -> bool {
        self.get(o).is_some()
    }

    fn add(&mut self, object: Object) -> Obj {
        self.objects.push(Some(object));
        Obj(self.objects.len() - 1)
    }

    pub fn clip(&self, o: Obj) -> Option<&Clip> {
        match &self.get(o)?.kind {
            Kind::Clip(c) => Some(c),
            _ => None,
        }
    }

    fn clip_mut(&mut self, o: Obj) -> Option<&mut Clip> {
        match &mut self.get_mut(o)?.kind {
            Kind::Clip(c) => Some(c),
            _ => None,
        }
    }

    /// La timeline d'un clip (la principale pour la racine).
    pub fn timeline(&self, o: Obj) -> Option<&Timeline> {
        timeline(&self.lib, self.get(o)?.character, o == self.root)
    }

    /// Les enfants d'un clip (par profondeur) ou d'un bouton.
    pub fn children(&self, o: Obj) -> Vec<Obj> {
        match self.get(o).map(|x| &x.kind) {
            Some(Kind::Clip(c)) => c.children.values().copied().collect(),
            Some(Kind::Button(b)) => b.children.iter().map(|c| c.1).collect(),
            _ => Vec::new(),
        }
    }

    /// Un enfant par son nom d'instance.
    pub fn child(&self, o: Obj, name: &str) -> Option<Obj> {
        self.children(o).into_iter().find(|&c| self.get(c).is_some_and(|x| x.name == name))
    }

    pub fn total_frames(&self, o: Obj) -> u16 {
        self.timeline(o).map_or(1, |t| t.frames.len().max(1) as u16)
    }

    pub fn current_frame(&self, o: Obj) -> u16 {
        self.clip(o).map_or(0, |c| c.frame)
    }

    pub fn label(&self, o: Obj, label: &str) -> Option<u16> {
        self.timeline(o)?.labels.get(label).copied()
    }

    // --- Création et retrait ------------------------------------------------

    /// Un nouvel objet d'un personnage sous `parent`, à `depth`.
    pub fn create(&mut self, parent: Obj, depth: i32, character: Id, name: &str) -> Obj {
        self.remove_depth(parent, depth);
        let o = self.instance(Some(parent), depth, Some(character), name);
        if let Some(c) = self.clip_mut(parent) {
            c.children.insert(depth, o);
        }
        self.start(o);
        o
    }

    /// L'objet d'un personnage, sans le ranger chez son parent.
    fn instance(&mut self, parent: Option<Obj>, depth: i32, character: Option<Id>, name: &str) -> Obj {
        let kind = match character.and_then(|id| self.lib.characters.get(&id)) {
            Some(Character::Sprite(_)) => Kind::Clip(Clip { frame: 0, playing: true, children: BTreeMap::new() }),
            Some(Character::EditText(t)) => Kind::Text(TextField { text: t.text.clone(), autosize_right: false }),
            Some(Character::Button(_)) => Kind::Button(Button { state: ButtonState::Up, children: Vec::new() }),
            _ => Kind::Graphic,
        };
        let mut object = Object::new(parent, depth, character, kind);
        object.name = name.to_string();
        // Flash nomme les clips sans nom « instance1 », « instance2 »…
        if object.name.is_empty() && matches!(object.kind, Kind::Clip(_)) {
            self.instances += 1;
            object.name = format!("instance{}", self.instances);
        }
        self.add(object)
    }

    /// Un nouveau clip affiche sa première image tout de suite ; un bouton
    /// crée les objets de ses états.
    fn start(&mut self, o: Obj) {
        if self.clip(o).is_some() {
            self.created.push(o);
            self.goto(o, 1);
        }
        let lib = self.lib.clone();
        if let Some(Character::Button(def)) = self.get(o).and_then(|x| x.character).and_then(|id| lib.characters.get(&id)) {
            let mut records: Vec<&ButtonRecord> = def.records.iter().filter(|r| r.states & !BUTTON_HIT != 0).collect();
            records.sort_by_key(|r| r.depth);
            for r in records {
                let child = self.instance(Some(o), timeline_depth(r.depth), Some(r.character), "");
                if let Some(x) = self.get_mut(child) {
                    x.matrix = r.matrix;
                    x.color = r.color;
                }
                if let Some(Object { kind: Kind::Button(b), .. }) = self.get_mut(o) {
                    b.children.push((r.states, child));
                }
                self.start(child);
            }
        }
    }

    /// Retire un objet et ses descendants.
    pub fn remove(&mut self, o: Obj) {
        let Some(object) = self.get(o) else { return };
        let (parent, depth) = (object.parent, object.depth);
        if self.clip(o).is_some() {
            self.removed.push(o);
        }
        for child in self.children(o) {
            self.remove(child);
        }
        if let Some(p) = parent
            && let Some(c) = self.clip_mut(p)
            && c.children.get(&depth) == Some(&o)
        {
            c.children.remove(&depth);
        }
        self.objects[o.0] = None;
    }

    fn remove_depth(&mut self, parent: Obj, depth: i32) {
        if let Some(&old) = self.clip(parent).and_then(|c| c.children.get(&depth)) {
            self.remove(old);
        }
    }

    // --- Timelines ------------------------------------------------------------

    pub fn play(&mut self, o: Obj) {
        if let Some(c) = self.clip_mut(o) {
            c.playing = true;
        }
    }

    pub fn stop(&mut self, o: Obj) {
        if let Some(c) = self.clip_mut(o) {
            c.playing = false;
        }
    }

    pub fn goto_and_play(&mut self, o: Obj, frame: u16) {
        self.goto(o, frame);
        self.play(o);
    }

    pub fn goto_and_stop(&mut self, o: Obj, frame: u16) {
        self.goto(o, frame);
        self.stop(o);
    }

    /// Va à une image (bornée à la timeline) : ses commandes sont appliquées
    /// et elle est signalée à la logique.
    pub fn goto(&mut self, o: Obj, frame: u16) {
        let Some(current) = self.clip(o).map(|c| c.frame) else { return };
        let frame = frame.clamp(1, self.total_frames(o));
        if frame == current {
            return;
        }
        if frame > current {
            for f in current + 1..=frame {
                self.apply_frame(o, f, f == frame);
            }
        } else {
            self.rewind(o, frame);
        }
        if let Some(c) = self.clip_mut(o) {
            c.frame = frame;
        }
    }

    /// Avance d'une image tous les clips qui jouent : la racine d'abord,
    /// puis ses enfants par profondeur.
    pub fn advance(&mut self) {
        self.advance_clip(self.root);
    }

    fn advance_clip(&mut self, o: Obj) {
        // Les clips que cette image va placer n'avancent qu'à la suivante.
        let children = self.children(o);
        if let Some(clip) = self.clip(o) {
            let (frame, playing) = (clip.frame, clip.playing);
            let total = self.total_frames(o);
            if frame == 0 {
                self.goto(o, 1);
            } else if playing && total > 1 {
                self.goto(o, if frame >= total { 1 } else { frame + 1 });
            }
        }
        for child in children {
            if self.exists(child) {
                self.advance_clip(child);
            }
        }
    }

    /// Les commandes d'une image ; si elle est affichée, elle est signalée
    /// avant que les clips qu'elle place ne signalent la leur.
    fn apply_frame(&mut self, o: Obj, frame: u16, shown: bool) {
        let lib = self.lib.clone();
        let Some(def) = self.timeline_frame(&lib, o, frame) else { return };
        if shown {
            self.signal(o, frame, def);
        }
        for command in &def.commands {
            match command {
                Command::Place(p) => self.place(o, p),
                Command::Remove(depth) => self.remove_depth(o, timeline_depth(*depth)),
            }
        }
    }

    fn timeline_frame<'a>(&self, lib: &'a Library, o: Obj, frame: u16) -> Option<&'a Frame> {
        timeline(lib, self.get(o)?.character, o == self.root)?.frames.get(usize::from(frame).checked_sub(1)?)
    }

    fn signal(&mut self, o: Obj, frame: u16, def: &Frame) {
        self.frames.push((o, frame));
        self.sounds.extend(def.sounds.iter().map(|s| (o, s.clone())));
    }

    fn place(&mut self, parent: Obj, p: &Place) {
        let depth = timeline_depth(p.depth);
        let existing = self.clip(parent).and_then(|c| c.children.get(&depth).copied());
        let target = match (p.character, existing) {
            (Some(id), Some(old)) if p.is_move => {
                // Même objet, autre personnage.
                if let Some(x) = self.get_mut(old) {
                    x.character = Some(id);
                }
                old
            }
            (Some(id), _) => {
                let o = self.create(parent, depth, id, p.name.as_deref().unwrap_or(""));
                if let Some(x) = self.get_mut(o) {
                    x.from_timeline = true;
                }
                o
            }
            (None, Some(old)) => old,
            (None, None) => return,
        };
        if let Some(x) = self.get_mut(target) {
            apply_place(x, p);
        }
    }

    /// Retour à une image précédente : les objets de la timeline sont
    /// reconstruits depuis l'image 1 ; ceux qui restent à la même profondeur
    /// avec le même personnage sont gardés.
    fn rewind(&mut self, o: Obj, frame: u16) {
        let lib = self.lib.clone();
        let Some(timeline) = timeline(&lib, self.get(o).and_then(|x| x.character), o == self.root) else { return };
        // L'état de chaque profondeur à l'image visée.
        let mut wanted: BTreeMap<i32, (Id, Vec<&Place>)> = BTreeMap::new();
        for def in timeline.frames.iter().take(usize::from(frame)) {
            for command in &def.commands {
                match command {
                    Command::Place(p) => {
                        let depth = timeline_depth(p.depth);
                        match p.character {
                            Some(id) if p.is_move => {
                                if let Some(w) = wanted.get_mut(&depth) {
                                    w.0 = id;
                                    w.1.push(p);
                                }
                            }
                            Some(id) => {
                                wanted.insert(depth, (id, vec![p]));
                            }
                            None => {
                                if let Some(w) = wanted.get_mut(&depth) {
                                    w.1.push(p);
                                }
                            }
                        }
                    }
                    Command::Remove(depth) => {
                        wanted.remove(&timeline_depth(*depth));
                    }
                }
            }
        }
        if let Some(def) = timeline.frames.get(usize::from(frame) - 1) {
            self.signal(o, frame, def);
        }
        for (depth, child) in self.clip(o).map(|c| c.children.clone()).unwrap_or_default() {
            let Some(x) = self.get(child) else { continue };
            if x.from_timeline && wanted.get(&depth).is_none_or(|w| Some(w.0) != x.character) {
                self.remove(child);
            }
        }
        for (depth, (id, places)) in wanted {
            let existing = self.clip(o).and_then(|c| c.children.get(&depth).copied());
            let target = match existing {
                Some(x) if self.get(x).is_some_and(|x| x.from_timeline) => x,
                Some(_) => continue,
                None => {
                    let name = places.iter().rev().find_map(|p| p.name.clone()).unwrap_or_default();
                    let x = self.create(o, depth, id, &name);
                    if let Some(obj) = self.get_mut(x) {
                        obj.from_timeline = true;
                    }
                    x
                }
            };
            for p in places {
                if let Some(x) = self.get_mut(target) {
                    apply_place(x, p);
                }
            }
        }
    }

    // --- Propriétés -------------------------------------------------------------

    /// La matrice d'un objet vers la scène.
    pub fn world_matrix(&self, o: Obj) -> Matrix {
        let mut m = Matrix::IDENTITY;
        let mut cur = Some(o);
        while let Some(x) = cur.and_then(|c| self.get(c)) {
            m = x.matrix * m;
            cur = x.parent;
        }
        m
    }

    /// L'objet, marqué comme transformé par le code.
    pub fn scripted(&mut self, o: Obj) -> Option<&mut Object> {
        let x = self.get_mut(o)?;
        x.scripted = true;
        Some(x)
    }

    pub fn text(&self, o: Obj) -> Option<&str> {
        match &self.get(o)?.kind {
            Kind::Text(t) => Some(&t.text),
            _ => None,
        }
    }

    pub fn set_text(&mut self, o: Obj, text: &str) {
        if let Some(Object { kind: Kind::Text(t), .. }) = self.get_mut(o) {
            text.clone_into(&mut t.text);
        }
    }

    pub fn set_autosize_right(&mut self, o: Obj) {
        if let Some(Object { kind: Kind::Text(t), .. }) = self.get_mut(o) {
            t.autosize_right = true;
        }
    }

    // --- Propriétés de Flash (_x, _xscale, _rotation…) ------------------------

    /// _x et _y : Flash les garde en vingtièmes de pixel (tronqués).
    pub fn x(&self, o: Obj) -> f64 {
        self.get(o).map_or(0.0, |x| f64::from(x.matrix.tx))
    }

    pub fn y(&self, o: Obj) -> f64 {
        self.get(o).map_or(0.0, |x| f64::from(x.matrix.ty))
    }

    pub fn set_x(&mut self, o: Obj, v: f64) {
        if v.is_finite()
            && let Some(x) = self.scripted(o)
        {
            x.matrix.tx = twips(v);
        }
    }

    pub fn set_y(&mut self, o: Obj, v: f64) {
        if v.is_finite()
            && let Some(x) = self.scripted(o)
        {
            x.matrix.ty = twips(v);
        }
    }

    /// (_xscale, _yscale, _rotation).
    fn scale_rotation(&self, o: Obj) -> (f64, f64, f64) {
        let Some(x) = self.get(o) else { return (100.0, 100.0, 0.0) };
        if let Some(t) = x.transform {
            return t;
        }
        let m = x.matrix;
        let (a, b, c, d) = (f64::from(m.a), f64::from(m.b), f64::from(m.c), f64::from(m.d));
        let xscale = a.hypot(b) * 100.0;
        let mut yscale = c.hypot(d) * 100.0;
        if a * d - b * c < 0.0 {
            yscale = -yscale;
        }
        (xscale, yscale, b.atan2(a).to_degrees())
    }

    pub fn xscale(&self, o: Obj) -> f64 {
        self.scale_rotation(o).0
    }

    pub fn yscale(&self, o: Obj) -> f64 {
        self.scale_rotation(o).1
    }

    pub fn rotation(&self, o: Obj) -> f64 {
        self.scale_rotation(o).2
    }

    fn set_scale_rotation(&mut self, o: Obj, xscale: f64, yscale: f64, rotation: f64) {
        if let Some(x) = self.scripted(o) {
            x.transform = Some((xscale, yscale, rotation));
            let (sin, cos) = rotation.to_radians().sin_cos();
            let (sx, sy) = (xscale / 100.0, yscale / 100.0);
            x.matrix = Matrix { a: (sx * cos) as f32, b: (sx * sin) as f32, c: (-sy * sin) as f32, d: (sy * cos) as f32, tx: x.matrix.tx, ty: x.matrix.ty };
        }
    }

    pub fn set_xscale(&mut self, o: Obj, v: f64) {
        let (_, y, r) = self.scale_rotation(o);
        if v.is_finite() {
            self.set_scale_rotation(o, v, y, r);
        }
    }

    pub fn set_yscale(&mut self, o: Obj, v: f64) {
        let (x, _, r) = self.scale_rotation(o);
        if v.is_finite() {
            self.set_scale_rotation(o, x, v, r);
        }
    }

    /// _rotation, ramenée dans -180..180 comme Flash.
    pub fn set_rotation(&mut self, o: Obj, v: f64) {
        let (x, y, _) = self.scale_rotation(o);
        if v.is_finite() {
            let r = (v + 180.0).rem_euclid(360.0) - 180.0;
            self.set_scale_rotation(o, x, y, r);
        }
    }

    pub fn visible(&self, o: Obj) -> bool {
        self.get(o).is_some_and(|x| x.visible)
    }

    pub fn set_visible(&mut self, o: Obj, v: bool) {
        if let Some(x) = self.get_mut(o) {
            x.visible = v;
        }
    }

    /// _alpha, en pourcents.
    pub fn alpha(&self, o: Obj) -> f64 {
        self.get(o).map_or(100.0, |x| f64::from(x.color.mult[3]) * 100.0)
    }

    pub fn set_alpha(&mut self, o: Obj, v: f64) {
        if let Some(x) = self.scripted(o) {
            x.color.mult[3] = (v / 100.0) as f32;
        }
    }

    /// _width et _height : le rectangle dans le repère du parent.
    pub fn width(&self, o: Obj) -> f64 {
        let r = self.bounds(o, self.get(o).map_or(Matrix::IDENTITY, |x| x.matrix));
        if r.is_empty() { 0.0 } else { f64::from(r.x_max - r.x_min) }
    }

    pub fn height(&self, o: Obj) -> f64 {
        let r = self.bounds(o, self.get(o).map_or(Matrix::IDENTITY, |x| x.matrix));
        if r.is_empty() { 0.0 } else { f64::from(r.y_max - r.y_min) }
    }

    /// L'arbre des clips nommés (tests) : nom, personnage, image, lecture,
    /// visibilité.
    pub fn dump(&self) -> String {
        let mut out = String::new();
        let mut stack = vec![(self.root, 0)];
        while let Some((o, indent)) = stack.pop() {
            let Some(x) = self.get(o) else { continue };
            if let Kind::Clip(c) = &x.kind {
                let b = self.bounds(o, self.world_matrix(o));
                out.push_str(&format!(
                    "{}{} #{:?} image {} {}{} [{:.0},{:.0} → {:.0},{:.0}]\n",
                    "  ".repeat(indent),
                    if x.name.is_empty() { "(racine)" } else { &x.name },
                    x.character,
                    c.frame,
                    if c.playing { "joue" } else { "arrêté" },
                    if x.visible { "" } else { " caché" },
                    b.x_min,
                    b.y_min,
                    b.x_max,
                    b.y_max
                ));
            }
            for child in self.children(o).into_iter().rev() {
                if matches!(self.get(child).map(|c| &c.kind), Some(Kind::Clip(_))) && !self.get(child).is_some_and(|c| c.name.starts_with("instance")) {
                    stack.push((child, indent + 1));
                }
            }
        }
        out
    }

    /// Copie un clip à une profondeur de son parent (duplicateMovieClip) :
    /// même personnage, matrice et couleur, depuis sa première image.
    pub fn duplicate(&mut self, o: Obj, name: &str, depth: i32) -> Option<Obj> {
        let x = self.get(o)?;
        let (parent, character, matrix, color, transform) = (x.parent?, x.character?, x.matrix, x.color, x.transform);
        let copy = self.create(parent, depth, character, name);
        if let Some(c) = self.get_mut(copy) {
            c.matrix = matrix;
            c.color = color;
            c.transform = transform;
            c.scripted = true;
        }
        Some(copy)
    }

    /// Le rectangle d'un objet dans le repère `m`.
    pub fn bounds(&self, o: Obj, m: Matrix) -> Rect {
        let Some(x) = self.get(o) else { return Rect::EMPTY };
        match &x.kind {
            Kind::Clip(_) => self.children(o).into_iter().filter_map(|c| self.get(c).map(|cx| self.bounds(c, m * cx.matrix))).fold(Rect::EMPTY, Rect::union),
            Kind::Button(b) => b.children.iter().filter(|c| c.0 & BUTTON_UP != 0).filter_map(|&(_, c)| self.get(c).map(|cx| self.bounds(c, m * cx.matrix))).fold(Rect::EMPTY, Rect::union),
            _ => x.character.map_or(Rect::EMPTY, |id| character_bounds(&self.lib, id).transform(m)),
        }
    }

    /// Le point (x, y) de la scène touche-t-il les formes visibles de `o`
    /// (un bouton : sa zone de clic) ?
    pub fn hit(&self, o: Obj, x: f32, y: f32) -> bool {
        let Some(obj) = self.get(o) else { return false };
        if !obj.visible {
            return false;
        }
        match &obj.kind {
            Kind::Clip(_) => self.children(o).into_iter().any(|c| self.get(c).is_some_and(|cx| cx.clip_depth == 0) && self.hit(c, x, y)),
            Kind::Button(_) => {
                let Some(Character::Button(def)) = obj.character.and_then(|id| self.lib.characters.get(&id)) else { return false };
                let world = self.world_matrix(o);
                def.records.iter().filter(|r| r.states & BUTTON_HIT != 0).any(|r| character_hit(&self.lib, r.character, world * r.matrix, x, y))
            }
            _ => obj.character.is_some_and(|id| character_hit(&self.lib, id, self.world_matrix(o), x, y)),
        }
    }
}

impl Object {
    fn new(parent: Option<Obj>, depth: i32, character: Option<Id>, kind: Kind) -> Object {
        Object {
            parent,
            depth,
            character,
            name: String::new(),
            matrix: Matrix::IDENTITY,
            color: ColorTransform::IDENTITY,
            visible: true,
            clip_depth: 0,
            ratio: 0,
            from_timeline: false,
            scripted: false,
            transform: None,
            kind,
        }
    }
}

/// Une coordonnée gardée en vingtièmes de pixel (tronqués, comme Flash).
fn twips(v: f64) -> f32 {
    ((v * 20.0).trunc() / 20.0) as f32
}

/// Les profondeurs de timeline sont décalées sous celles du code
/// (duplicateMovieClip…), comme dans Flash.
pub const TIMELINE_DEPTH: i32 = -16384;

fn timeline_depth(depth: u16) -> i32 {
    i32::from(depth) + TIMELINE_DEPTH
}

fn timeline(lib: &Library, character: Option<Id>, root: bool) -> Option<&Timeline> {
    match character {
        None if root => Some(&lib.root),
        None => None,
        Some(id) => match lib.characters.get(&id)? {
            Character::Sprite(t) => Some(t),
            _ => None,
        },
    }
}

/// Un placement de timeline sur un objet ; un objet transformé par le code
/// garde sa matrice et sa couleur.
fn apply_place(x: &mut Object, p: &Place) {
    if !x.scripted {
        if let Some(m) = p.matrix {
            x.matrix = m;
            x.transform = None;
        }
        if let Some(c) = p.color {
            x.color = c;
        }
    }
    if let Some(r) = p.ratio {
        x.ratio = r;
    }
    if let Some(n) = &p.name {
        n.clone_into(&mut x.name);
    }
    if let Some(c) = p.clip_depth {
        x.clip_depth = c;
    }
}

/// Le rectangle d'un personnage dans son repère.
pub fn character_bounds(lib: &Library, id: Id) -> Rect {
    match lib.characters.get(&id) {
        Some(Character::Shape(s)) => s.bounds,
        Some(Character::Text(t)) => t.bounds.transform(t.matrix),
        Some(Character::EditText(t)) => t.bounds,
        Some(Character::Morph(m)) => m.start_bounds.union(m.end_bounds),
        _ => Rect::EMPTY,
    }
}

/// Le point de la scène touche-t-il un personnage dessiné avec `world` ?
fn character_hit(lib: &Library, id: Id, world: Matrix, x: f32, y: f32) -> bool {
    let (lx, ly) = world.invert().apply(x, y);
    match lib.characters.get(&id) {
        Some(Character::Shape(s)) => s.bounds.contains(lx, ly) && s.layers.iter().any(|l| l.fills.iter().any(|(_, segs)| crate::render::inside(segs, lx, ly))),
        // Un texte fixe : sur le contour d'un de ses glyphes, comme Flash.
        Some(Character::Text(t)) => t.bounds.transform(t.matrix).contains(lx, ly)
            && t.glyphs.iter().any(|g| {
                let Some(Character::Font(f)) = lib.characters.get(&g.font) else { return false };
                let Some(glyph) = f.glyphs.get(g.index) else { return false };
                let k = g.size / f.em;
                let (gx, gy) = (t.matrix * Matrix::translate(g.x, g.y) * Matrix::scale(k, k)).invert().apply(lx, ly);
                crate::render::inside(&glyph.segments, gx, gy)
            }),
        Some(Character::Sprite(t)) => t.frames.first().is_some_and(|f| {
            f.commands.iter().any(|c| match c {
                Command::Place(p) => p.character.is_some_and(|child| character_hit(lib, child, world * p.matrix.unwrap_or(Matrix::IDENTITY), x, y)),
                Command::Remove(_) => false,
            })
        }),
        _ => character_bounds(lib, id).contains(lx, ly),
    }
}
