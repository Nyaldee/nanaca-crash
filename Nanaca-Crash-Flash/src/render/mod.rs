//! Le rendu : les formes du SWF dessinées en vectoriel par la carte
//! graphique (voir `mesh`), avec leurs couleurs, dégradés et images, les
//! transformations de couleur et les masques.
//!
//! Le stencil sert à deux choses : son bit de poids fort marque l'intérieur
//! de la forme en cours de dessin, ses 7 autres bits comptent les masques
//! imbriqués au-dessus de chaque pixel. Un objet n'est visible que là où ce
//! compte vaut le niveau de masque en cours.

mod mesh;

pub use mesh::inside;

use std::collections::HashMap;

use miniquad::*;

use crate::geom::{ColorTransform, Matrix};
use crate::library::{Character, Fill, GradientKind, Id, Library, Spread};

/// Le bit qui marque l'intérieur d'une forme.
const INSIDE: i32 = 0x80;
/// Les bits du niveau de masque.
const LEVEL: i32 = 0x7F;

/// Un maillage préparé : une forme, une étape de forme qui se transforme,
/// ou un glyphe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MeshKey {
    Shape(Id),
    Morph(Id, u16),
    Glyph(Id, usize),
}

/// Les passes de dessin, chacune son réglage du stencil.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Pass {
    /// Marque l'intérieur d'un remplissage (inversion, pair-impair).
    FillStencil,
    /// Marque l'intérieur d'un trait (union).
    StrokeStencil,
    /// Peint là où l'intérieur est marqué, et l'efface.
    Cover,
    /// Masque : monte d'un niveau là où l'intérieur est marqué…
    MaskRaise,
    /// … puis efface la marque.
    MaskClear,
    /// Masque retiré : redescend d'un niveau.
    MaskLower,
    /// Avant d'effacer : toutes les écritures ouvertes.
    Clear,
}

enum Mode {
    Draw,
    /// Les objets dessinés forment un masque.
    MaskAdd,
    /// Les objets dessinés retirent un masque.
    MaskRemove,
}

struct GpuMesh {
    bindings: Bindings,
    items: Vec<GpuItem>,
}

struct GpuItem {
    paint: Paint,
    stroke: bool,
    stencil: (i32, i32),
    cover: (i32, i32),
}

#[derive(Clone, Copy)]
enum Paint {
    Solid([f32; 4]),
    /// Le dégradé (en texture de 256 couleurs) et la matrice de la forme
    /// vers le carré du dégradé.
    Gradient { ramp: TextureId, inverse: Matrix, kind: GradientKind, spread: Spread },
    /// L'image et la matrice de la forme vers ses coordonnées de texture.
    Bitmap { id: Id, inverse: Matrix, repeat: bool, smooth: bool },
}

#[repr(C)]
struct Uniforms {
    mvp: [f32; 16],
    paint: [f32; 16],
    color: [f32; 4],
    cx_mult: [f32; 4],
    cx_add: [f32; 4],
    /// x : genre de peinture, y : répétition du dégradé, z : foyer.
    params: [f32; 4],
}

pub struct Renderer {
    ctx: Box<dyn RenderingBackend>,
    shader: ShaderId,
    pipelines: HashMap<(Pass, i32), Pipeline>,
    meshes: HashMap<MeshKey, GpuMesh>,
    bitmaps: HashMap<Id, TextureId>,
    white: TextureId,
    /// Scène vers l'espace de l'écran (-1..1).
    projection: Matrix,
    level: i32,
    mode: Mode,
    /// _quality = "HIGH" : les images lissables sont lissées.
    pub smooth: bool,
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut ctx = window::new_rendering_backend();
        let shader = ctx
            .new_shader(
                ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT },
                ShaderMeta {
                    images: vec!["tex".to_string()],
                    uniforms: UniformBlockLayout {
                        uniforms: vec![
                            UniformDesc::new("mvp", UniformType::Mat4),
                            UniformDesc::new("paint", UniformType::Mat4),
                            UniformDesc::new("color", UniformType::Float4),
                            UniformDesc::new("cx_mult", UniformType::Float4),
                            UniformDesc::new("cx_add", UniformType::Float4),
                            UniformDesc::new("params", UniformType::Float4),
                        ],
                    },
                },
            )
            .expect("shader");
        let white = ctx.new_texture_from_rgba8(1, 1, &[255; 4]);
        Renderer {
            ctx,
            shader,
            pipelines: HashMap::new(),
            meshes: HashMap::new(),
            bitmaps: HashMap::new(),
            white,
            projection: Matrix::IDENTITY,
            level: 0,
            mode: Mode::Draw,
            smooth: false,
        }
    }

    /// Commence une image : la scène (`stage`, en pixels) occupe `viewport`
    /// (x, y, largeur, hauteur, depuis le bas) de la fenêtre ; le reste est
    /// noir.
    pub fn begin(&mut self, stage: (f32, f32), background: [f32; 4], viewport: (i32, i32, i32, i32)) {
        // L'effacement suit les masques d'écriture en cours : toutes les
        // couleurs et tout le stencil d'abord.
        let clear = self.pipeline(Pass::Clear);
        self.ctx.begin_default_pass(PassAction::Nothing);
        self.ctx.apply_pipeline(&clear);
        self.ctx.clear(Some((0.0, 0.0, 0.0, 1.0)), None, Some(0));
        let (x, y, w, h) = viewport;
        self.ctx.apply_viewport(x, y, w, h);
        self.ctx.apply_scissor_rect(x, y, w, h);
        self.ctx.clear(Some((background[0], background[1], background[2], 1.0)), None, Some(0));
        self.projection = Matrix { a: 2.0 / stage.0, b: 0.0, c: 0.0, d: -2.0 / stage.1, tx: -1.0, ty: 1.0 };
        self.level = 0;
        self.mode = Mode::Draw;
    }

    pub fn end(&mut self) {
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    /// Les pixels RGBA de la fenêtre (tests), ligne du haut d'abord.
    pub fn read_pixels(&mut self, width: i32, height: i32) -> Vec<u8> {
        let mut pixels = vec![0u8; (width * height * 4) as usize];
        // SAFETY : le tampon a la taille demandée, en RGBA 8 bits.
        unsafe {
            gl::glReadPixels(0, 0, width, height, gl::GL_RGBA, gl::GL_UNSIGNED_BYTE, pixels.as_mut_ptr().cast());
        }
        let row = (width * 4) as usize;
        let mut flipped = Vec::with_capacity(pixels.len());
        for line in pixels.chunks_exact(row).rev() {
            flipped.extend_from_slice(line);
        }
        flipped
    }

    // --- Masques ------------------------------------------------------------

    /// Les objets dessinés ensuite forment un masque.
    pub fn mask_begin(&mut self) {
        self.mode = Mode::MaskAdd;
    }

    /// Le masque est prêt : ce qui suit n'est visible qu'à l'intérieur.
    pub fn mask_apply(&mut self) {
        self.level += 1;
        self.mode = Mode::Draw;
    }

    /// Les objets du masque, dessinés à nouveau, le retirent.
    pub fn mask_remove_begin(&mut self) {
        self.mode = Mode::MaskRemove;
    }

    pub fn mask_end(&mut self) {
        self.level = (self.level - 1).max(0);
        self.mode = Mode::Draw;
    }

    // --- Dessin --------------------------------------------------------------

    /// Dessine un maillage (préparé au besoin) ; `tint` remplace la couleur
    /// des remplissages unis (glyphes).
    pub fn draw(&mut self, lib: &Library, key: MeshKey, world: Matrix, cx: ColorTransform, tint: Option<[f32; 4]>) {
        if !self.meshes.contains_key(&key) {
            let Some(mesh) = self.upload(lib, key) else { return };
            self.meshes.insert(key, mesh);
        }
        let mvp = (self.projection * world).to_mat4();
        let count = self.meshes[&key].items.len();
        for i in 0..count {
            let mesh = &self.meshes[&key];
            let item = &mesh.items[i];
            let (paint, stroke, stencil, cover, mut bindings) = (item.paint, item.stroke, item.stencil, item.cover, mesh.bindings.clone());
            let (uniforms, texture) = self.uniforms(&paint, mvp, cx, tint);
            bindings.images = vec![texture];
            let stencil_pass = if stroke { Pass::StrokeStencil } else { Pass::FillStencil };
            match self.mode {
                Mode::Draw => {
                    self.pass(stencil_pass, &bindings, &uniforms, stencil);
                    self.pass(Pass::Cover, &bindings, &uniforms, cover);
                }
                Mode::MaskAdd => {
                    self.pass(stencil_pass, &bindings, &uniforms, stencil);
                    self.pass(Pass::MaskRaise, &bindings, &uniforms, cover);
                    self.pass(Pass::MaskClear, &bindings, &uniforms, cover);
                }
                Mode::MaskRemove => self.pass(Pass::MaskLower, &bindings, &uniforms, cover),
            }
        }
    }

    fn pass(&mut self, pass: Pass, bindings: &Bindings, uniforms: &Uniforms, (start, count): (i32, i32)) {
        let pipeline = self.pipeline(pass);
        self.ctx.apply_pipeline(&pipeline);
        self.ctx.apply_bindings(bindings);
        self.ctx.apply_uniforms(UniformsSource::table(uniforms));
        self.ctx.draw(start, count, 1);
    }

    fn uniforms(&mut self, paint: &Paint, mvp: [f32; 16], cx: ColorTransform, tint: Option<[f32; 4]>) -> (Uniforms, TextureId) {
        let mut u = Uniforms { mvp, paint: Matrix::IDENTITY.to_mat4(), color: [1.0; 4], cx_mult: cx.mult, cx_add: cx.add, params: [0.0; 4] };
        let texture = match *paint {
            Paint::Solid(c) => {
                u.color = tint.unwrap_or(c);
                self.white
            }
            Paint::Gradient { ramp, inverse, kind, spread } => {
                u.paint = inverse.to_mat4();
                let (k, focal) = match kind {
                    GradientKind::Linear => (1.0, 0.0),
                    GradientKind::Radial => (2.0, 0.0),
                    GradientKind::Focal(f) => (3.0, f),
                };
                let s = match spread {
                    Spread::Pad => 0.0,
                    Spread::Reflect => 1.0,
                    Spread::Repeat => 2.0,
                };
                u.params = [k, s, focal, 0.0];
                ramp
            }
            Paint::Bitmap { id, inverse, repeat, smooth } => {
                u.paint = inverse.to_mat4();
                u.params = [4.0, 0.0, 0.0, 0.0];
                let t = self.bitmaps.get(&id).copied().unwrap_or(self.white);
                let wrap = if repeat { TextureWrap::Repeat } else { TextureWrap::Clamp };
                self.ctx.texture_set_wrap(t, wrap, wrap);
                let filter = if smooth && self.smooth { FilterMode::Linear } else { FilterMode::Nearest };
                self.ctx.texture_set_filter(t, filter, MipmapFilterMode::None);
                t
            }
        };
        (u, texture)
    }

    /// Le réglage d'une passe pour le niveau de masque en cours.
    fn pipeline(&mut self, pass: Pass) -> Pipeline {
        let level = self.level;
        if let Some(&p) = self.pipelines.get(&(pass, level)) {
            return p;
        }
        let face = |func, reference: i32, test_mask: i32, pass_op, write_mask: i32| StencilFaceState {
            fail_op: StencilOp::Keep,
            depth_fail_op: StencilOp::Keep,
            pass_op,
            test_func: func,
            test_ref: reference,
            test_mask: test_mask as u32,
            write_mask: write_mask as u32,
        };
        let (state, color) = match pass {
            Pass::FillStencil => (face(CompareFunc::Equal, level, LEVEL, StencilOp::Invert, INSIDE), false),
            Pass::StrokeStencil => (face(CompareFunc::Equal, INSIDE | level, LEVEL, StencilOp::Replace, INSIDE), false),
            Pass::Cover => (face(CompareFunc::Equal, INSIDE | level, 0xFF, StencilOp::Zero, INSIDE), true),
            Pass::MaskRaise => (face(CompareFunc::Equal, INSIDE | level, 0xFF, StencilOp::IncrementClamp, LEVEL), false),
            Pass::MaskClear => (face(CompareFunc::Equal, INSIDE, INSIDE, StencilOp::Zero, INSIDE), false),
            Pass::MaskLower => (face(CompareFunc::Equal, level, LEVEL, StencilOp::DecrementClamp, LEVEL), false),
            Pass::Clear => (face(CompareFunc::Always, 0, 0xFF, StencilOp::Keep, 0xFF), true),
        };
        // Le shader rend des couleurs prémultipliées.
        let params = PipelineParams {
            color_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
            alpha_blend: Some(BlendState::new(Equation::Add, BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha))),
            stencil_test: Some(StencilState { front: state, back: state }),
            color_write: (color, color, color, color),
            ..Default::default()
        };
        let p = self.ctx.new_pipeline(&[BufferLayout::default()], &[VertexAttribute::new("pos", VertexFormat::Float2)], self.shader, params);
        self.pipelines.insert((pass, level), p);
        p
    }

    /// Prépare un maillage et ce qu'il peint (dégradés, images).
    fn upload(&mut self, lib: &Library, key: MeshKey) -> Option<GpuMesh> {
        let data = match (key, lib.characters.get(&match key {
            MeshKey::Shape(id) | MeshKey::Morph(id, _) | MeshKey::Glyph(id, _) => id,
        })?) {
            (MeshKey::Shape(_), Character::Shape(s)) => mesh::shape(s),
            (MeshKey::Morph(_, ratio), Character::Morph(m)) => mesh::morph(m, ratio),
            (MeshKey::Glyph(_, index), Character::Font(f)) => mesh::glyph(&f.glyphs.get(index)?.segments),
            _ => return None,
        };
        if data.vertices.is_empty() {
            return None;
        }
        let vertices = self.ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Immutable, BufferSource::slice(&data.vertices));
        let indices: Vec<u32> = (0..data.vertices.len() as u32).collect();
        let index_buffer = self.ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        let mut items = Vec::with_capacity(data.items.len());
        for item in &data.items {
            let paint = match &item.fill {
                Fill::Solid(c) => Paint::Solid(*c),
                Fill::Gradient(g) => {
                    let ramp = self.ctx.new_texture_from_rgba8(256, 1, &ramp(&g.stops));
                    self.ctx.texture_set_filter(ramp, FilterMode::Linear, MipmapFilterMode::None);
                    Paint::Gradient { ramp, inverse: g.matrix.invert(), kind: g.kind, spread: g.spread }
                }
                Fill::Bitmap { id, matrix, repeat, smooth } => {
                    let Some(Character::Bitmap(b)) = lib.characters.get(id) else { continue };
                    if !self.bitmaps.contains_key(id) {
                        let t = self.ctx.new_texture_from_rgba8(b.width as u16, b.height as u16, &b.rgba);
                        self.bitmaps.insert(*id, t);
                    }
                    let to_uv = Matrix::scale(1.0 / b.width as f32, 1.0 / b.height as f32);
                    Paint::Bitmap { id: *id, inverse: to_uv * matrix.invert(), repeat: *repeat, smooth: *smooth }
                }
            };
            items.push(GpuItem { paint, stroke: item.stroke, stencil: (item.stencil.0 as i32, item.stencil.1 as i32), cover: (item.cover.0 as i32, item.cover.1 as i32) });
        }
        Some(GpuMesh { bindings: Bindings { vertex_buffers: vec![vertices], index_buffer, images: vec![self.white] }, items })
    }
}

/// Les 256 couleurs d'un dégradé.
fn ramp(stops: &[(f32, [f32; 4])]) -> Vec<u8> {
    let mut out = Vec::with_capacity(256 * 4);
    for i in 0..256 {
        let t = i as f32 / 255.0;
        let c = match stops.iter().position(|s| s.0 >= t) {
            None => stops.last().map_or([0.0; 4], |s| s.1),
            Some(0) => stops[0].1,
            Some(k) => {
                let (a, b) = (stops[k - 1], stops[k]);
                let f = if b.0 > a.0 { (t - a.0) / (b.0 - a.0) } else { 0.0 };
                [0, 1, 2, 3].map(|j| a.1[j] + (b.1[j] - a.1[j]) * f)
            }
        };
        out.extend(c.map(|v| (v.clamp(0.0, 1.0) * 255.0).round() as u8));
    }
    out
}

const VERTEX: &str = r"#version 100
attribute vec2 pos;
uniform mat4 mvp;
varying vec2 local;
void main() {
    local = pos;
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
}
";

const FRAGMENT: &str = r"#version 100
precision highp float;
varying vec2 local;
uniform mat4 paint;
uniform vec4 color;
uniform vec4 cx_mult;
uniform vec4 cx_add;
uniform vec4 params;
uniform sampler2D tex;

void main() {
    vec4 c = color;
    if (params.x > 0.5) {
        vec2 p = (paint * vec4(local, 0.0, 1.0)).xy;
        if (params.x > 3.5) {
            // Les images sont prémultipliées : la vraie couleur d'abord.
            c = texture2D(tex, p);
            if (c.a > 0.0) { c.rgb /= c.a; }
        } else {
            float t;
            if (params.x < 1.5) {
                t = (p.x + 1.0) * 0.5;
            } else if (params.x < 2.5) {
                t = length(p);
            } else {
                // Foyer décalé : position le long du rayon foyer → bord.
                vec2 f = vec2(params.z, 0.0);
                vec2 d = p - f;
                float a = dot(d, d);
                float b = dot(f, d);
                float k = dot(f, f) - 1.0;
                t = a > 0.0 ? a / (-b + sqrt(max(b * b - a * k, 0.0))) : 0.0;
            }
            if (params.y < 0.5) {
                t = clamp(t, 0.0, 1.0);
            } else if (params.y < 1.5) {
                t = mod(t, 2.0);
                if (t > 1.0) { t = 2.0 - t; }
            } else {
                t = fract(t);
            }
            c = texture2D(tex, vec2(t * (255.0 / 256.0) + 0.5 / 256.0, 0.5));
        }
    }
    c = clamp(c * cx_mult + cx_add, 0.0, 1.0);
    gl_FragColor = vec4(c.rgb * c.a, c.a);
}
";
