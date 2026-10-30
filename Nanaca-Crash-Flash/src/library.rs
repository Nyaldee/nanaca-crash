//! Le contenu du SWF une fois lu : ses personnages (formes, images,
//! polices, textes, boutons, sons, clips) et leurs timelines, en données à
//! nous, prêtes à afficher et à jouer.

use std::collections::HashMap;

use crate::geom::{ColorTransform, Matrix, Rect};

/// Identifiant d'un personnage du SWF.
pub type Id = u16;

pub struct Library {
    pub characters: HashMap<Id, Character>,
    /// La timeline principale.
    pub root: Timeline,
    /// Noms de liaison (attachMovie, attachSound) vers les personnages.
    pub exports: HashMap<String, Id>,
    /// Taille de la scène, en pixels.
    pub stage: (f32, f32),
    pub frame_rate: f32,
    pub background: [f32; 4],
}

pub enum Character {
    Shape(Shape),
    Morph(Morph),
    Bitmap(Bitmap),
    Font(Font),
    Text(Text),
    EditText(EditText),
    Button(Button),
    Sound(Sound),
    Sprite(Timeline),
}

// --- Formes -----------------------------------------------------------------

/// Une forme : ses calques, dessinés dans l'ordre. Chaque calque dessine
/// ses remplissages, puis ses traits.
pub struct Shape {
    pub bounds: Rect,
    pub layers: Vec<Layer>,
}

#[derive(Default)]
pub struct Layer {
    /// Chaque remplissage et les bords de ses régions (non ordonnés : ils
    /// forment ensemble des contours fermés).
    pub fills: Vec<(Fill, Vec<Segment>)>,
    pub lines: Vec<(Line, Vec<Segment>)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Segment {
    Line([f32; 2], [f32; 2]),
    /// Courbe de Bézier quadratique : départ, contrôle, arrivée.
    Curve([f32; 2], [f32; 2], [f32; 2]),
}

#[derive(Clone, Debug)]
pub enum Fill {
    Solid([f32; 4]),
    Gradient(Gradient),
    /// `matrix` place l'image (en pixels d'image) dans la forme.
    Bitmap { id: Id, matrix: Matrix, repeat: bool, smooth: bool },
}

#[derive(Clone, Debug)]
pub struct Gradient {
    pub kind: GradientKind,
    /// Place le carré du dégradé (-1..1) dans la forme.
    pub matrix: Matrix,
    pub spread: Spread,
    /// Position (0..1) et couleur de chaque étape.
    pub stops: Vec<(f32, [f32; 4])>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GradientKind {
    Linear,
    Radial,
    /// Radial avec un foyer décalé (-1..1 sur l'axe x).
    Focal(f32),
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Spread {
    Pad,
    Reflect,
    Repeat,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    /// Épaisseur en pixels (0 : trait le plus fin possible).
    pub width: f32,
    pub color: [f32; 4],
}

/// Un bord d'une forme, avec ses styles : remplissage à gauche (`fill0`),
/// à droite (`fill1`), trait (`line`) ; 0 pour aucun, sinon un indice à
/// partir de 1.
#[derive(Clone, Copy, Debug)]
pub struct Edge {
    pub fill0: u16,
    pub fill1: u16,
    pub line: u16,
    pub segment: Segment,
}

/// Une forme qui se transforme : ses styles et ses bords au début et à la
/// fin, appariés un à un.
pub struct Morph {
    pub start_bounds: Rect,
    pub end_bounds: Rect,
    pub fills: Vec<(Fill, Fill)>,
    pub lines: Vec<(Line, Line)>,
    pub start: Vec<Edge>,
    pub end: Vec<Edge>,
}

// --- Images -------------------------------------------------------------------

/// Pixels RGBA (couleurs prémultipliées par l'alpha), ligne du haut
/// d'abord.
pub struct Bitmap {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

// --- Textes -------------------------------------------------------------------

pub struct Font {
    /// Taille du carré « em » dans les unités des glyphes.
    pub em: f32,
    pub ascent: f32,
    pub descent: f32,
    pub glyphs: Vec<Glyph>,
    /// Caractère (UTF-16) vers glyphe.
    pub codes: HashMap<u16, usize>,
}

pub struct Glyph {
    /// Les bords du glyphe (un seul remplissage).
    pub segments: Vec<Segment>,
    pub advance: f32,
}

/// Un texte fixe : ses glyphes déjà placés.
pub struct Text {
    pub bounds: Rect,
    pub matrix: Matrix,
    pub glyphs: Vec<PlacedGlyph>,
}

#[derive(Clone, Copy)]
pub struct PlacedGlyph {
    pub font: Id,
    pub index: usize,
    /// Position de la ligne de base et taille du texte, en pixels.
    pub x: f32,
    pub y: f32,
    pub size: f32,
    pub color: [f32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Align {
    Left,
    Right,
    Center,
    Justify,
}

/// Un champ de texte (modifiable par le code).
pub struct EditText {
    pub bounds: Rect,
    pub font: Option<Id>,
    /// Taille du texte, en pixels.
    pub size: f32,
    pub color: [f32; 4],
    pub align: Align,
    pub left_margin: f32,
    pub right_margin: f32,
    pub leading: f32,
    /// Variable liée au champ ("" : aucune).
    pub variable: String,
    pub text: String,
    pub html: bool,
    pub word_wrap: bool,
}

// --- Boutons --------------------------------------------------------------------

pub struct Button {
    pub records: Vec<ButtonRecord>,
}

pub struct ButtonRecord {
    /// États où l'objet apparaît (BUTTON_UP…).
    pub states: u8,
    pub character: Id,
    pub depth: u16,
    pub matrix: Matrix,
    pub color: ColorTransform,
}

pub const BUTTON_UP: u8 = 1;
pub const BUTTON_OVER: u8 = 2;
pub const BUTTON_DOWN: u8 = 4;
pub const BUTTON_HIT: u8 = 8;

// --- Sons --------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Codec {
    /// PCM petit-boutiste (ou d'ordre inconnu, traité pareil).
    Pcm,
    Adpcm,
    Mp3,
    Nellymoser,
    /// Un format que le jeu n'utilise pas.
    Other(u8),
}

pub struct Sound {
    pub codec: Codec,
    pub rate: u32,
    pub sixteen_bits: bool,
    pub stereo: bool,
    /// Nombre d'échantillons (par canal).
    pub samples: u32,
    pub data: Vec<u8>,
}

/// Un son lancé (ou arrêté) par une image de timeline.
#[derive(Clone, Debug)]
pub struct StartSound {
    pub id: Id,
    pub stop: bool,
    /// Ne pas relancer le son s'il joue déjà.
    pub no_multiple: bool,
    pub loops: u16,
    /// Début et fin, en échantillons à 44 100 Hz.
    pub in_point: Option<u32>,
    pub out_point: Option<u32>,
    /// Volume gauche et droite (0..1) à des positions à 44 100 Hz.
    pub envelope: Vec<(u32, f32, f32)>,
}

// --- Timelines --------------------------------------------------------------------

#[derive(Default)]
pub struct Timeline {
    pub frames: Vec<Frame>,
    /// Étiquette vers numéro d'image (à partir de 1).
    pub labels: HashMap<String, u16>,
}

#[derive(Default)]
pub struct Frame {
    pub commands: Vec<Command>,
    pub sounds: Vec<StartSound>,
}

pub enum Command {
    Place(Place),
    Remove(u16),
}

/// Placement ou modification de l'objet d'une profondeur.
pub struct Place {
    pub depth: u16,
    /// Un nouveau personnage à cette profondeur.
    pub character: Option<Id>,
    /// Modifie l'objet déjà présent (avec `character` : le remplace).
    pub is_move: bool,
    pub matrix: Option<Matrix>,
    pub color: Option<ColorTransform>,
    pub ratio: Option<u16>,
    pub name: Option<String>,
    /// Ce placement est un masque jusqu'à cette profondeur.
    pub clip_depth: Option<u16>,
}
