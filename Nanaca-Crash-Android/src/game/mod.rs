//! Le jeu : le graphe de scène, ses actions et armatures (comme le
//! directeur et l'ordonnanceur de Cocos2d), les menus et le toucher, le
//! son, la sauvegarde, et la logique d'origine réécrite en Rust.

mod achievements;
mod backdrop;
mod focus;
mod gallery;
mod loader;
mod option;
mod play;
pub mod save;
mod title;
mod widgets;

use std::collections::HashMap;
use std::rc::Rc;

use crate::action::{Action, Callback, Event, Running};
use crate::armature::{Armature, ArmatureEvent};
use crate::assets::Assets;
use crate::audio::Audio;
use crate::data::xml;
use crate::node::{Kind, MenuItem, Node, NodeId};

pub use crate::node::Graph;
pub use focus::Dir;
use loader::Loader;
use save::Save;

/// Une image du jeu : 1/60 s (intervalle d'animation du directeur).
pub const FRAME: f32 = 1.0 / 60.0;
/// Hauteur de la scène en points ; la largeur suit les proportions.
pub const DESIGN_HEIGHT: f32 = 320.0;

/// Ce qu'une armature signale, transmis à la logique.
pub type ArmatureListener = Rc<dyn Fn(&mut Game, NodeId, &ArmatureEvent)>;

/// Les scènes du jeu.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SceneKind {
    /// Le titre et son entrée : 0 chargement, 1 sans animation, 2 glissé.
    Title(i32),
    Game,
    /// Le tutoriel : vrai s'il est ouvert depuis les options (il y ramène).
    Tutorial(bool),
    Option,
    Gallery,
    /// La page d'un élément de la galerie (changeGalleryScene).
    GalleryContent(i32),
    /// SceneAgain : « see you » avant une nouvelle partie.
    Again,
}

/// Les transitions entre scènes, faites sur une capture de la scène
/// sortante (les render textures de Cocos2d).
#[derive(Clone, Copy, Debug)]
pub enum Transition {
    /// CCTransitionCrossFade : l'ancienne image s'efface sur la première
    /// image de la nouvelle.
    CrossFade(f32),
    /// CCTransitionProgressVariant : l'ancienne image se referme en barre
    /// (centre et sens de la barre) sur la nouvelle scène.
    Progress { duration: f32, midpoint: (f32, f32), rate: (f32, f32) },
}

/// Une transition en cours.
#[derive(Clone, Copy, Debug)]
pub struct Transiting {
    pub kind: Transition,
    pub elapsed: f32,
}

/// La logique de la scène en cours.
pub enum Logic {
    None,
    Title(Box<title::Title>),
    Play(Box<play::Play>),
    Option(Box<option::Options>),
    Gallery(Box<gallery::Gallery>),
    Viewer(Box<gallery::Viewer>),
}

/// L'apparence d'un bouton pressé (selected) ou relâché (unselected).
pub type ItemStyle = Rc<dyn Fn(&mut Game, NodeId, bool)>;

/// Un générateur pseudo-aléatoire (xorshift), pour lrand48 et
/// math.random.
pub struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed | 1)
    }

    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// lrand48 : 0 à 2³¹ − 1.
    pub fn lrand48(&mut self) -> i32 {
        (self.next() >> 33) as i32
    }

    /// math.random() : 0 à 1 (exclu).
    pub fn random(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u64 << 24) as f32
    }
}

pub struct Game {
    pub g: Graph,
    pub assets: Rc<Assets>,
    pub audio: Audio,
    pub save: Save,
    /// Taille de la scène en points.
    pub design: (f32, f32),
    /// La scène affichée.
    pub scene: Option<NodeId>,
    pub logic: Logic,
    pub rng: Rng,
    /// Les textes de la langue du jeu (CCTextResource).
    texts: HashMap<String, String>,
    next: Option<(SceneKind, Option<Transition>)>,
    /// La scène qui attend la capture de l'ancienne (faite au dessin).
    pub awaiting_capture: Option<(SceneKind, Transition)>,
    /// La transition en cours (le toucher est coupé pendant).
    pub transition: Option<Transiting>,
    listeners: HashMap<NodeId, ArmatureListener>,
    /// Les menus, dans l'ordre où ils prennent le toucher, et l'action de
    /// chaque bouton.
    menus: Vec<NodeId>,
    item_actions: HashMap<NodeId, Callback>,
    item_styles: HashMap<NodeId, ItemStyle>,
    /// Une boîte de dialogue qui prend tout le toucher (son menu seul).
    pub modal: Option<NodeId>,
    /// Le menu qui suit le toucher en cours, et son bouton choisi.
    tracking: Option<(NodeId, Option<NodeId>)>,
    /// La scène a pris le toucher en cours (ccTouchBegan).
    scene_touch: bool,
    loader: Loader,
    /// CCDirector::pause : plus d'actions ni de mises à jour.
    pub director_paused: bool,
    /// Les éléments de la galerie (et leurs succès).
    pub gallery: Vec<achievements::Content>,
    /// Le choix de la manette, et son cadre.
    focus: Option<focus::Target>,
    focus_frame: Option<NodeId>,
}

impl Game {
    pub fn new(assets: Rc<Assets>, audio: Audio, save: Save, design: (f32, f32)) -> Game {
        let lang = crate::platform::language();
        let texts = load_texts(&assets, lang);
        let gallery = achievements::load(&assets, lang);
        crate::text::set_cjk(if lang == "zh-tw" { crate::text::Cjk::Traditional } else { crate::text::Cjk::Japanese });
        // Graine fixe pour les essais (NANACA_SEED), sinon l'heure.
        let seed = std::env::var("NANACA_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64));
        let mut game = Game {
            g: Graph::default(),
            assets,
            audio,
            save,
            design,
            scene: None,
            logic: Logic::None,
            rng: Rng::new(seed),
            texts,
            next: None,
            awaiting_capture: None,
            transition: None,
            listeners: HashMap::new(),
            menus: Vec::new(),
            item_actions: HashMap::new(),
            item_styles: HashMap::new(),
            modal: None,
            tracking: None,
            scene_touch: false,
            loader: Loader::default(),
            director_paused: false,
            gallery,
            focus: None,
            focus_frame: None,
        };
        // AppDelegate puis SceneSplash : les feuilles communes, puis le titre.
        game.assets.add_sprite_frames("gamesheet.plist");
        game.assets.add_sprite_frames("backgrounds.plist");
        game.apply_volumes();
        game.switch(SceneKind::Title(0));
        game
    }

    /// Un texte de la langue du jeu (sa clé s'il manque).
    pub fn text(&self, key: &str) -> String {
        self.texts.get(key).cloned().unwrap_or_else(|| key.to_string())
    }

    // --- Réglages (GameOption) -----------------------------------------------

    pub fn effect_volume(&self) -> f32 {
        self.save.float("effect_volume", 0.5)
    }

    pub fn sound_volume(&self) -> f32 {
        self.save.float("sound_volume", 0.5)
    }

    /// Les volumes enregistrés, donnés au son.
    pub fn apply_volumes(&mut self) {
        let (e, s) = (self.effect_volume(), self.sound_volume());
        self.audio.set_effects_volume(e);
        self.audio.set_music_volume(s);
    }

    /// GameOption::getSkinSelected : au premier lancement, la version 12
    /// est notée et le skin 2 (feu d'artifice) choisi.
    pub fn skin(&mut self) -> i32 {
        if self.save.int("game_version", -1) == -1 {
            self.save.set_int("game_version", 12);
            self.save.set_int("skin_selected", 2);
        }
        self.save.int("skin_selected", 2)
    }

    /// La clé du skin choisi (skin.xml).
    pub fn skin_key(&mut self) -> &'static str {
        match self.skin() {
            1 => "last_summer",
            2 => "firework_show",
            _ => "way_to_school",
        }
    }

    /// SkinResource::getArmatureImageName : la feuille des personnages.
    pub fn armature_sheet(&mut self) -> &'static str {
        match self.skin_key() {
            "firework_show" => "merge_firework",
            "last_summer" => "merge_summer",
            _ => "merge",
        }
    }

    /// L'image d'un skin : chemin de l'école, dernier été, feu d'artifice.
    pub fn skin_image(&mut self, school: &'static str, summer: &'static str, firework: &'static str) -> &'static str {
        match self.skin_key() {
            "firework_show" => firework,
            "last_summer" => summer,
            _ => school,
        }
    }

    /// SimpleAudioEngine::playEffect.
    pub fn play_effect(&mut self, path: &str) -> u32 {
        self.play_effect_gain(path, 1.0)
    }

    /// playEffect avec un volume propre (gain).
    pub fn play_effect_gain(&mut self, path: &str, gain: f32) -> u32 {
        let assets = self.assets.clone();
        self.audio.play_effect(&assets, path, false, gain)
    }

    /// playBackgroundMusic(chemin, boucle, reprise de la boucle en secondes).
    pub fn play_music(&mut self, path: &str, looped: bool, loop_start: f32) {
        let assets = self.assets.clone();
        self.audio.play_music(&assets, path, looped, loop_start);
    }

    // --- Raccourcis du graphe --------------------------------------------------

    /// createWithSpriteFrameName.
    pub fn sprite(&mut self, frame: &str) -> NodeId {
        let assets = self.assets.clone();
        self.g.sprite_frame(&assets, frame)
    }

    /// CCSprite::create (image entière).
    pub fn sprite_file(&mut self, file: &str) -> NodeId {
        let assets = self.assets.clone();
        self.g.sprite_file(&assets, file)
    }

    /// CCLabelBMFont::create.
    pub fn bm_label(&mut self, text: &str, font: &str, align: u8) -> NodeId {
        let assets = self.assets.clone();
        self.g.bm_label(&assets, text, font, align)
    }

    /// CCLabelAtlas::create (taille d'une case en points).
    pub fn atlas_label(&mut self, text: &str, image: &str, w: f32, h: f32, start: char) -> NodeId {
        let assets = self.assets.clone();
        self.g.atlas_label(&assets, text, image, (w, h), start)
    }

    /// addChild(enfant, z, étiquette).
    pub fn add(&mut self, parent: NodeId, child: NodeId, z: i32, tag: i32) {
        self.g.attach(parent, child, z, tag);
    }

    pub fn set_pos(&mut self, id: NodeId, x: f32, y: f32) {
        if let Some(n) = self.g.get_mut(id) {
            n.pos = (x, y);
        }
    }

    pub fn set_x(&mut self, id: NodeId, x: f32) {
        if let Some(n) = self.g.get_mut(id) {
            n.pos.0 = x;
        }
    }

    pub fn set_y(&mut self, id: NodeId, y: f32) {
        if let Some(n) = self.g.get_mut(id) {
            n.pos.1 = y;
        }
    }

    pub fn pos(&self, id: NodeId) -> (f32, f32) {
        self.g.get(id).map_or((0.0, 0.0), |n| n.pos)
    }

    pub fn size(&self, id: NodeId) -> (f32, f32) {
        self.g.get(id).map_or((0.0, 0.0), |n| n.size)
    }

    pub fn set_anchor(&mut self, id: NodeId, x: f32, y: f32) {
        if let Some(n) = self.g.get_mut(id) {
            n.anchor = (x, y);
        }
    }

    pub fn set_visible(&mut self, id: NodeId, on: bool) {
        if let Some(n) = self.g.get_mut(id) {
            n.visible = on;
        }
    }

    pub fn visible(&self, id: NodeId) -> bool {
        self.g.get(id).is_some_and(|n| n.visible)
    }

    pub fn set_scale(&mut self, id: NodeId, s: f32) {
        if let Some(n) = self.g.get_mut(id) {
            n.scale = (s, s);
        }
    }

    pub fn scale(&self, id: NodeId) -> f32 {
        self.g.get(id).map_or(1.0, |n| n.scale.0)
    }

    pub fn set_rotation(&mut self, id: NodeId, degrees: f32) {
        if let Some(n) = self.g.get_mut(id) {
            n.set_rotation(degrees);
        }
    }

    pub fn rotation(&self, id: NodeId) -> f32 {
        self.g.get(id).map_or(0.0, |n| n.rotation.0)
    }

    pub fn set_color(&mut self, id: NodeId, rgb: [u8; 3]) {
        self.g.set_color(id, rgb);
    }

    pub fn set_opacity(&mut self, id: NodeId, opacity: u8) {
        self.g.set_opacity(id, opacity);
    }

    pub fn child(&self, id: NodeId, tag: i32) -> Option<NodeId> {
        self.g.child(id, tag)
    }

    // --- Actions -------------------------------------------------------------

    /// runAction.
    pub fn run(&mut self, id: NodeId, action: Action) {
        self.run_tagged(id, action, -1);
    }

    pub fn run_tagged(&mut self, id: NodeId, mut action: Action, tag: i32) {
        let Some(n) = self.g.get_mut(id) else { return };
        action.start(id, n);
        n.actions.push(Running { action, tag });
    }

    /// stopAllActions.
    pub fn stop_actions(&mut self, id: NodeId) {
        let Some(n) = self.g.get_mut(id) else { return };
        let mut actions = std::mem::take(&mut n.actions);
        for r in &mut actions {
            r.action.stop(n);
        }
    }

    pub fn stop_action_tag(&mut self, id: NodeId, tag: i32) {
        let Some(n) = self.g.get_mut(id) else { return };
        if let Some(i) = n.actions.iter().position(|r| r.tag == tag) {
            let mut r = n.actions.remove(i);
            r.action.stop(n);
        }
    }

    // --- Armatures -----------------------------------------------------------

    /// Une armature (CCArmature::create), ses os cachés jusqu'au premier
    /// mouvement.
    pub fn armature(&mut self, name: &str) -> NodeId {
        let armature = self.assets.armature(name).map(|data| Box::new(Armature::new(data)));
        // updateOffsetPoint : la taille de la pose de départ ; l'ancre
        // compense son coin, l'origine reste à la position.
        let bounds = armature.as_ref().and_then(|a| a.initial_bounds(&self.assets));
        let mut n = Node::new(armature.map_or(Kind::Node, Kind::Armature));
        if let Some([x, y, w, h]) = bounds {
            n.size = (w, h);
            n.offset = (-x, -y);
            if w != 0.0 && h != 0.0 {
                n.anchor = (-x / w, -y / h);
            }
        }
        n.cascade_color = true;
        n.cascade_opacity = true;
        self.g.add(n)
    }

    pub fn armature_mut(&mut self, id: NodeId) -> Option<&mut Armature> {
        match &mut self.g.get_mut(id)?.kind {
            Kind::Armature(a) => Some(a),
            _ => None,
        }
    }

    /// Les événements de mouvement et d'images de cette armature.
    pub fn listen_armature(&mut self, id: NodeId, listener: ArmatureListener) {
        self.listeners.insert(id, listener);
    }

    // --- Menus ---------------------------------------------------------------

    /// Un menu plein écran (CCMenu), centré, qui prend le toucher.
    pub fn menu(&mut self) -> NodeId {
        let mut n = Node::new(Kind::Menu { enabled: true });
        n.size = self.design;
        n.anchor = (0.5, 0.5);
        n.ignore_anchor = true;
        n.pos = (self.design.0 / 2.0, self.design.1 / 2.0);
        n.cascade_color = true;
        n.cascade_opacity = true;
        let id = self.g.add(n);
        self.menus.push(id);
        id
    }

    pub fn set_menu_enabled(&mut self, menu: NodeId, on: bool) {
        if let Some(n) = self.g.get_mut(menu)
            && let Kind::Menu { enabled } = &mut n.kind
        {
            *enabled = on;
        }
    }

    /// Un bouton fait d'une étiquette (MyMenuItemLabel) : l'étiquette en
    /// enfant, ancrée en bas à gauche, le bouton à sa taille. Pressé, il ne
    /// change pas d'aspect : MyMenuItemLabel::selected saute le
    /// grossissement de CCMenuItemLabel.
    pub fn item_label(&mut self, label: NodeId, action: Option<Callback>) -> NodeId {
        let mut n = Node::new(Kind::Item(MenuItem { enabled: true, selected: false }));
        n.anchor = (0.5, 0.5);
        n.cascade_color = true;
        n.cascade_opacity = true;
        n.size = self.g.get(label).map_or((0.0, 0.0), |l| l.size);
        let id = self.g.add(n);
        if let Some(l) = self.g.get_mut(label) {
            l.anchor = (0.0, 0.0);
            l.pos = (0.0, 0.0);
        }
        self.g.attach(id, label, 0, -1);
        if let Some(a) = action {
            self.item_actions.insert(id, a);
        }
        id
    }

    /// L'action d'un bouton déjà construit.
    pub fn register_item(&mut self, item: NodeId, action: Callback) {
        self.item_actions.insert(item, action);
    }

    /// L'apparence d'un bouton pressé ou relâché.
    pub fn style_item(&mut self, item: NodeId, style: ItemStyle) {
        self.item_styles.insert(item, style);
    }

    /// Un bouton fait d'une image (CCMenuItemImage) : l'image en enfant.
    pub fn image_item(&mut self, image: NodeId, action: fn(&mut Game)) -> NodeId {
        let mut n = Node::new(Kind::Item(MenuItem { enabled: true, selected: false }));
        n.anchor = (0.5, 0.5);
        n.cascade_color = true;
        n.cascade_opacity = true;
        n.size = self.g.get(image).map_or((0.0, 0.0), |i| i.size);
        let item = self.g.add(n);
        if let Some(i) = self.g.get_mut(image) {
            i.anchor = (0.0, 0.0);
            i.pos = (0.0, 0.0);
        }
        self.g.attach(item, image, 0, -1);
        self.item_actions.insert(item, Rc::new(move |game: &mut Game, _| action(game)));
        item
    }

    /// CCMenuItemLabel::setString : le texte, puis la taille du bouton.
    pub fn set_item_text(&mut self, item: NodeId, text: &str) {
        let Some(label) = self.g.get(item).and_then(|n| n.children.first().copied()) else { return };
        self.g.set_text(label, text);
        let size = self.g.get(label).map_or((0.0, 0.0), |l| l.size);
        if let Some(n) = self.g.get_mut(item) {
            n.size = size;
        }
    }

    /// Le bouton d'un menu sous le toucher (CCMenu::itemForTouch).
    fn item_at(&self, menu: NodeId, point: (f32, f32)) -> Option<NodeId> {
        self.g.get(menu)?.children.iter().copied().find(|&c| {
            self.g.get(c).is_some_and(|n| n.visible && matches!(n.kind, Kind::Item(MenuItem { enabled: true, .. }))) && self.g.contains(c, point)
        })
    }

    /// Un menu peut-il prendre le toucher : actif, visible, parents visibles.
    fn menu_active(&self, menu: NodeId) -> bool {
        self.g.get(menu).is_some_and(|n| matches!(n.kind, Kind::Menu { enabled: true })) && self.g.shown(menu)
    }

    fn set_selected(&mut self, item: NodeId, on: bool) {
        if let Some(n) = self.g.get_mut(item)
            && let Kind::Item(i) = &mut n.kind
        {
            i.selected = on;
        }
        if let Some(style) = self.item_styles.get(&item).cloned() {
            style(self, item, on);
        }
    }

    // --- Toucher -------------------------------------------------------------

    /// Début du toucher (clic) : les menus d'abord, qui le gardent s'ils
    /// ont un bouton dessous.
    pub fn touch_began(&mut self, point: (f32, f32)) {
        if self.tracking.is_some() || self.input_blocked() {
            return;
        }
        // Une boîte de dialogue avale tout : seul son menu répond.
        let menus = match self.modal {
            Some(m) if self.g.alive(m) => vec![m],
            Some(_) => {
                self.modal = None;
                self.menus.clone()
            }
            None => self.menus.clone(),
        };
        for menu in menus {
            if !self.menu_active(menu) {
                continue;
            }
            if let Some(item) = self.item_at(menu, point) {
                self.set_selected(item, true);
                self.tracking = Some((menu, Some(item)));
                return;
            }
        }
        if self.modal.is_some() {
            return;
        }
        // Puis la scène (couches tactiles, curseurs, listes).
        match &self.logic {
            // Une animation de la galerie : ses gestes seulement.
            Logic::Play(p) if p.viewer.is_some() => self.scene_touch = gallery::touch_began_content(self, point),
            Logic::Play(_) => {
                self.scene_touch = true;
                play::touch_began(self, point);
            }
            Logic::Option(_) => self.scene_touch = option::touch_began(self, point),
            Logic::Gallery(_) => self.scene_touch = gallery::touch_began(self, point),
            Logic::Viewer(_) => self.scene_touch = gallery::touch_began_content(self, point),
            _ => {}
        }
    }

    pub fn touch_moved(&mut self, point: (f32, f32)) {
        if self.scene_touch {
            match &self.logic {
                Logic::Option(_) => option::touch_moved(self, point),
                Logic::Gallery(_) => gallery::touch_moved(self, point),
                Logic::Viewer(_) => gallery::touch_moved_content(self, point),
                Logic::Play(p) if p.viewer.is_some() => gallery::touch_moved_content(self, point),
                Logic::Play(_) => play::touch_moved(self, point),
                _ => {}
            }
            return;
        }
        let Some((menu, current)) = self.tracking else { return };
        let item = self.item_at(menu, point);
        if item != current {
            if let Some(c) = current {
                self.set_selected(c, false);
            }
            if let Some(i) = item {
                self.set_selected(i, true);
            }
            self.tracking = Some((menu, item));
        }
    }

    /// Fin du toucher : le bouton choisi est activé.
    pub fn touch_ended(&mut self, point: (f32, f32)) {
        if std::mem::take(&mut self.scene_touch) {
            match &self.logic {
                Logic::Play(p) if p.viewer.is_some() => gallery::touch_ended_content(self, point),
                Logic::Play(_) => play::touch_ended(self),
                Logic::Option(_) => option::touch_ended(self, point),
                Logic::Gallery(_) => gallery::touch_ended(self, point),
                Logic::Viewer(_) => gallery::touch_ended_content(self, point),
                _ => {}
            }
            return;
        }
        self.touch_moved(point);
        let Some((_, item)) = self.tracking.take() else { return };
        if let Some(item) = item {
            self.set_selected(item, false);
            if let Some(a) = self.item_actions.get(&item).cloned() {
                a(self, item);
            }
        }
    }

    // --- Scènes --------------------------------------------------------------

    /// replaceScene : la scène change à la fin de l'image.
    pub fn replace_scene(&mut self, kind: SceneKind) {
        self.next = Some((kind, None));
    }

    /// replaceScene avec une transition.
    pub fn replace_scene_with(&mut self, kind: SceneKind, transition: Transition) {
        self.next = Some((kind, Some(transition)));
    }

    /// La capture de l'ancienne scène est faite : la nouvelle arrive.
    pub fn start_transition(&mut self) {
        let Some((kind, transition)) = self.awaiting_capture.take() else { return };
        self.switch(kind);
        self.transition = Some(Transiting { kind: transition, elapsed: 0.0 });
    }

    /// Le toucher est coupé pendant une transition.
    fn input_blocked(&self) -> bool {
        self.transition.is_some() || self.awaiting_capture.is_some()
    }

    fn switch(&mut self, kind: SceneKind) {
        if let Some(old) = self.scene.take() {
            self.g.remove(old);
        }
        self.menus.clear();
        self.item_actions.clear();
        self.item_styles.clear();
        self.modal = None;
        self.listeners.clear();
        self.tracking = None;
        self.scene_touch = false;
        self.director_paused = false;
        self.focus = None;
        self.focus_frame = None;
        self.logic = Logic::None;
        let root = self.g.layer(self.design);
        self.scene = Some(root);
        match kind {
            SceneKind::Title(pattern) => title::enter(self, root, pattern),
            SceneKind::Game => play::enter(self, root),
            SceneKind::Tutorial(from_options) => play::enter_tutorial(self, root, from_options),
            SceneKind::Again => again(self, root),
            SceneKind::Option => option::enter(self, root),
            SceneKind::Gallery => gallery::enter(self, root),
            SceneKind::GalleryContent(id) => gallery::enter_content(self, root, id),
        }
    }

    // --- Images --------------------------------------------------------------

    /// Une image du jeu : les actions (ordonnanceur, priorité système),
    /// les armatures (mises à jour des nœuds), les sélecteurs (chargement,
    /// logique de la scène), puis le changement de scène.
    pub fn step(&mut self, dt: f32) {
        // Rien ne bouge avant la capture de l'ancienne scène.
        if self.awaiting_capture.is_some() {
            return;
        }
        if let Some(t) = &mut self.transition {
            t.elapsed += dt;
            let duration = match t.kind {
                Transition::CrossFade(d) | Transition::Progress { duration: d, .. } => d,
            };
            if t.elapsed >= duration {
                self.transition = None;
            }
        }
        if !self.director_paused {
            self.step_actions(dt);
            self.step_armatures(dt);
            // Mises à jour des nœuds (scheduleUpdate), puis sélecteurs.
            match self.logic {
                Logic::Play(_) => play::update(self, dt),
                Logic::Option(_) => option::update(self, dt),
                Logic::Gallery(_) => gallery::update(self, dt),
                Logic::Viewer(_) => gallery::update_content(self, dt),
                _ => {}
            }
            self.loader_step();
            if let Logic::Title(_) = self.logic {
                title::update(self);
            }
            self.follow_focus();
        }
        match self.next.take() {
            Some((kind, None)) => self.switch(kind),
            Some((kind, Some(t))) => self.awaiting_capture = Some((kind, t)),
            None => {}
        }
    }

    fn step_actions(&mut self, dt: f32) {
        let mut events = Vec::new();
        let mut tinted = Vec::new();
        for id in self.g.ids() {
            let Some(n) = self.g.get_mut(id) else { continue };
            if n.paused || n.actions.is_empty() {
                continue;
            }
            let before = (n.opacity, n.color);
            let mut actions = std::mem::take(&mut n.actions);
            actions.retain_mut(|r| {
                r.action.step(dt, n, &mut events);
                if r.action.done() {
                    r.action.stop(n);
                    return false;
                }
                true
            });
            // Les actions lancées entre-temps s'ajoutent après.
            actions.append(&mut n.actions);
            n.actions = actions;
            if (n.opacity, n.color) != before {
                tinted.push((id, n.opacity, n.color, before));
            }
        }
        // Fondus et teintes : setOpacity et setColor, transmis aux enfants.
        for (id, opacity, color, before) in tinted {
            if opacity != before.0 {
                self.g.set_opacity(id, opacity);
            }
            if color != before.1 {
                self.g.set_color(id, color);
            }
        }
        for e in events {
            match e {
                Event::Call(f, id) => {
                    if self.g.alive(id) {
                        f(self, id);
                    }
                }
                Event::Remove(id) => self.g.remove(id),
            }
        }
    }

    fn step_armatures(&mut self, dt: f32) {
        let mut events = Vec::new();
        for id in self.g.ids() {
            let Some(n) = self.g.get_mut(id) else { continue };
            if n.paused {
                continue;
            }
            if let Kind::Armature(a) = &mut n.kind {
                for e in a.update(dt) {
                    events.push((id, e));
                }
            }
        }
        for (id, e) in events {
            if let Some(l) = self.listeners.get(&id).cloned()
                && self.g.alive(id)
            {
                l(self, id, &e);
            }
        }
        self.listeners.retain(|id, _| self.g.alive(*id));
    }
}

/// SceneAgain : « see you » en bas à droite, puis une nouvelle partie
/// 1.2 s plus tard (sans transition).
fn again(game: &mut Game, root: NodeId) {
    game.assets.add_sprite_frames("gamesheet.plist");
    let see = game.sprite("seeyou.png");
    game.set_anchor(see, 1.0, 0.0);
    game.set_pos(see, game.design.0 - 5.0, 0.0);
    game.add(root, see, 0, -1);
    game.run(root, Action::sequence(vec![Action::delay(1.2), Action::call(|game: &mut Game, _| game.replace_scene(SceneKind::Game))]));
}

/// Les textes du jeu (string.xml, ou sa traduction japonaise ou chinoise).
fn load_texts(assets: &Assets, lang: &str) -> HashMap<String, String> {
    let file = match lang {
        "ja" => "string_ja.xml",
        "zh-tw" => "string_zh-tw.xml",
        _ => "string.xml",
    };
    let Some(root) = assets.apk.text(file).and_then(|t| xml::parse(&t)) else { return HashMap::new() };
    root.children_named("string").filter_map(|s| Some((s.attr("name")?.to_string(), s.text.clone()))).collect()
}
