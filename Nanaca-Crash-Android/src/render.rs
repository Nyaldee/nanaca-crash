//! Le rendu : des quadrilatères texturés, regroupés tant que la texture et
//! le mélange ne changent pas. Comme Cocos2d, les textures sont en alpha
//! prémultiplié et les couleurs des sommets aussi.

use std::collections::HashMap;

use miniquad::*;

/// Le mélange d'un dessin (facteurs source et destination de Cocos2d).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Blend {
    /// GL_ONE, GL_ONE_MINUS_SRC_ALPHA : le mélange normal (le seul employé).
    Normal,
}

/// Une texture chargée, avec sa taille en pixels.
#[derive(Clone, Copy, Debug)]
pub struct Texture {
    pub id: TextureId,
    pub width: f32,
    pub height: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct Vertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
}

/// Sommets par lot (au plus).
const BATCH: usize = 4096;

pub struct Renderer {
    ctx: Box<dyn RenderingBackend>,
    shader: ShaderId,
    /// Le même, qui écarte les pixels presque transparents : la forme
    /// d'une découpe (seuil d'alpha 0.05).
    clip_shader: ShaderId,
    pipelines: HashMap<(Blend, Stencil), Pipeline>,
    bindings: Bindings,
    white: TextureId,
    vertices: Vec<Vertex>,
    texture: Option<TextureId>,
    blend: Blend,
    stencil: Stencil,
    /// Points de la scène vers l'espace de l'écran (-1..1).
    mvp: [f32; 16],
    /// Taille de la scène en points et du viewport en pixels.
    design: (f32, f32),
    viewport: (i32, i32, i32, i32),
    /// Niveau de découpe (CCClippingNode) en cours.
    level: i32,
}

/// Le rôle du stencil pour un dessin.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Stencil {
    /// Dessin visible là où le niveau de découpe est atteint.
    Draw(i32),
    /// Forme d'une découpe : monte le niveau, sans couleur.
    Raise(i32),
    /// Découpe retirée : redescend le niveau, sans couleur.
    Lower(i32),
}

impl Renderer {
    pub fn new() -> Renderer {
        let mut ctx = window::new_rendering_backend();
        let meta = || ShaderMeta { images: vec!["tex".to_string()], uniforms: UniformBlockLayout { uniforms: vec![UniformDesc::new("mvp", UniformType::Mat4)] } };
        let shader = ctx.new_shader(ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT }, meta()).expect("shader");
        let clip_shader = ctx.new_shader(ShaderSource::Glsl { vertex: VERTEX, fragment: FRAGMENT_CLIP }, meta()).expect("shader");
        let white = ctx.new_texture_from_rgba8(1, 1, &[255; 4]);
        let vertex_buffer = ctx.new_buffer(BufferType::VertexBuffer, BufferUsage::Stream, BufferSource::empty::<Vertex>(BATCH));
        // Deux triangles par quadrilatère.
        let indices: Vec<u16> = (0..(BATCH / 4) as u16).flat_map(|q| [0, 1, 2, 2, 1, 3].map(|i| q * 4 + i)).collect();
        let index_buffer = ctx.new_buffer(BufferType::IndexBuffer, BufferUsage::Immutable, BufferSource::slice(&indices));
        let bindings = Bindings { vertex_buffers: vec![vertex_buffer], index_buffer, images: vec![white] };
        Renderer {
            ctx,
            shader,
            clip_shader,
            pipelines: HashMap::new(),
            bindings,
            white,
            vertices: Vec::with_capacity(BATCH),
            texture: None,
            blend: Blend::Normal,
            stencil: Stencil::Draw(0),
            mvp: [0.0; 16],
            design: (1.0, 1.0),
            viewport: (0, 0, 1, 1),
            level: 0,
        }
    }

    /// Une texture RGBA prémultipliée.
    pub fn texture(&mut self, width: u32, height: u32, rgba: &[u8]) -> Texture {
        let id = self.ctx.new_texture_from_rgba8(width as u16, height as u16, rgba);
        self.ctx.texture_set_filter(id, FilterMode::Linear, MipmapFilterMode::None);
        Texture { id, width: width as f32, height: height as f32 }
    }

    /// Lissée (par défaut) ou au plus proche (setAliasTexParameters).
    pub fn set_smooth(&mut self, texture: Texture, smooth: bool) {
        self.flush();
        let filter = if smooth { FilterMode::Linear } else { FilterMode::Nearest };
        self.ctx.texture_set_filter(texture.id, filter, MipmapFilterMode::None);
    }

    /// Commence une image : la scène (`design`, en points) occupe
    /// `viewport` (x, y depuis le bas, largeur, hauteur) de la fenêtre.
    pub fn begin(&mut self, design: (f32, f32), viewport: (i32, i32, i32, i32)) {
        self.ctx.begin_default_pass(PassAction::Nothing);
        // L'effacement suit les masques d'écriture du dernier réglage.
        let clear = self.pipeline(Blend::Normal, Stencil::Draw(0));
        self.ctx.apply_pipeline(&clear);
        self.ctx.clear(Some((0.0, 0.0, 0.0, 1.0)), None, Some(0));
        let (x, y, w, h) = viewport;
        self.ctx.apply_viewport(x, y, w, h);
        self.ctx.apply_scissor_rect(x, y, w, h);
        self.design = design;
        self.viewport = viewport;
        let (sx, sy) = (2.0 / design.0, 2.0 / design.1);
        self.mvp = [sx, 0.0, 0.0, 0.0, 0.0, sy, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, -1.0, -1.0, 0.0, 1.0];
        self.level = 0;
        self.stencil = Stencil::Draw(0);
        self.texture = None;
    }

    pub fn end(&mut self) {
        self.flush();
        self.ctx.end_render_pass();
        self.ctx.commit_frame();
    }

    /// Un quadrilatère : coins (bas gauche, bas droite, haut gauche, haut
    /// droite) en points de la scène, leurs coordonnées de texture, et la
    /// couleur prémultipliée (0..1). Sans texture : couleur unie.
    pub fn quad(&mut self, texture: Option<TextureId>, corners: [(f32, f32); 4], uv: [(f32, f32); 4], color: [f32; 4], blend: Blend) {
        self.quad_colors(texture, corners, uv, [color; 4], blend);
    }

    /// Un quadrilatère à couleur par coin (dégradés).
    pub fn quad_colors(&mut self, texture: Option<TextureId>, corners: [(f32, f32); 4], uv: [(f32, f32); 4], colors: [[f32; 4]; 4], blend: Blend) {
        let texture = texture.unwrap_or(self.white);
        if self.texture != Some(texture) || self.blend != blend || self.vertices.len() + 4 > BATCH {
            self.flush();
            self.texture = Some(texture);
            self.blend = blend;
        }
        for i in 0..4 {
            self.vertices.push(Vertex { pos: [corners[i].0, corners[i].1], uv: [uv[i].0, uv[i].1], color: colors[i] });
        }
    }

    /// Dessine le lot en cours.
    pub fn flush(&mut self) {
        let Some(texture) = self.texture else { return };
        if self.vertices.is_empty() {
            return;
        }
        let pipeline = self.pipeline(self.blend, self.stencil);
        self.ctx.buffer_update(self.bindings.vertex_buffers[0], BufferSource::slice(&self.vertices));
        self.bindings.images[0] = texture;
        self.ctx.apply_pipeline(&pipeline);
        self.ctx.apply_bindings(&self.bindings);
        self.ctx.apply_uniforms(UniformsSource::table(&self.mvp));
        self.ctx.draw(0, (self.vertices.len() / 4 * 6) as i32, 1);
        self.vertices.clear();
    }

    // --- Découpes (CCClippingNode) -------------------------------------------

    /// Ce qui est dessiné ensuite forme la découpe.
    pub fn clip_begin(&mut self) {
        self.flush();
        self.stencil = Stencil::Raise(self.level);
    }

    /// La découpe est prête : la suite n'est visible qu'à l'intérieur.
    pub fn clip_apply(&mut self) {
        self.flush();
        self.level += 1;
        self.stencil = Stencil::Draw(self.level);
    }

    /// La découpe, dessinée à nouveau, est retirée.
    pub fn clip_remove_begin(&mut self) {
        self.flush();
        self.stencil = Stencil::Lower(self.level);
    }

    pub fn clip_end(&mut self) {
        self.flush();
        self.level = (self.level - 1).max(0);
        self.stencil = Stencil::Draw(self.level);
    }

    fn pipeline(&mut self, blend: Blend, stencil: Stencil) -> Pipeline {
        if let Some(&p) = self.pipelines.get(&(blend, stencil)) {
            return p;
        }
        let (src, dst) = match blend {
            Blend::Normal => (BlendFactor::One, BlendFactor::OneMinusValue(BlendValue::SourceAlpha)),
        };
        let face = |func, reference: i32, pass_op, write_mask: u32| StencilFaceState { fail_op: StencilOp::Keep, depth_fail_op: StencilOp::Keep, pass_op, test_func: func, test_ref: reference, test_mask: 0xFF, write_mask };
        let (state, color) = match stencil {
            Stencil::Draw(level) => (face(CompareFunc::Equal, level, StencilOp::Keep, 0xFF), true),
            Stencil::Raise(level) => (face(CompareFunc::Equal, level, StencilOp::IncrementClamp, 0xFF), false),
            Stencil::Lower(level) => (face(CompareFunc::Equal, level, StencilOp::DecrementClamp, 0xFF), false),
        };
        let params = PipelineParams {
            color_blend: Some(BlendState::new(Equation::Add, src, dst)),
            alpha_blend: Some(BlendState::new(Equation::Add, src, dst)),
            stencil_test: Some(StencilState { front: state, back: state }),
            color_write: (color, color, color, color),
            ..Default::default()
        };
        let shader = if color { self.shader } else { self.clip_shader };
        let p = self.ctx.new_pipeline(
            &[BufferLayout::default()],
            &[VertexAttribute::new("pos", VertexFormat::Float2), VertexAttribute::new("uv", VertexFormat::Float2), VertexAttribute::new("color", VertexFormat::Float4)],
            shader,
            params,
        );
        self.pipelines.insert((blend, stencil), p);
        p
    }

    /// L'image dessinée jusqu'ici (la scène, dans son viewport) en texture
    /// opaque, ligne du bas d'abord : v = 0 en bas.
    pub fn snapshot(&mut self) -> Texture {
        self.flush();
        let (x, y, w, h) = self.viewport;
        let mut pixels = vec![0u8; (w * h * 4) as usize];
        // SAFETY : le tampon a la taille demandée, en RGBA 8 bits.
        unsafe {
            gl::glReadPixels(x, y, w, h, gl::GL_RGBA, gl::GL_UNSIGNED_BYTE, pixels.as_mut_ptr().cast());
        }
        for p in pixels.chunks_exact_mut(4) {
            p[3] = 255;
        }
        self.texture(w as u32, h as u32, &pixels)
    }

    pub fn delete(&mut self, texture: Texture) {
        self.flush();
        self.ctx.delete_texture(texture.id);
    }

    /// Efface l'image en cours (noir), pour redessiner par-dessus.
    pub fn clear(&mut self) {
        self.flush();
        let clear = self.pipeline(Blend::Normal, Stencil::Draw(0));
        self.ctx.apply_pipeline(&clear);
        self.ctx.clear(Some((0.0, 0.0, 0.0, 1.0)), None, Some(0));
    }

    /// Les pixels RGBA de la fenêtre (tests), ligne du haut d'abord.
    pub fn read_pixels(&mut self, width: i32, height: i32) -> Vec<u8> {
        self.flush();
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
}

/// Décode un PNG ou un JPEG en RGBA prémultiplié.
pub fn decode_image(bytes: &[u8]) -> Option<(u32, u32, Vec<u8>)> {
    let img = image::load_from_memory(bytes).ok()?.into_rgba8();
    let (w, h) = img.dimensions();
    let mut rgba = img.into_raw();
    for p in rgba.chunks_exact_mut(4) {
        let a = u16::from(p[3]);
        if a < 255 {
            for c in &mut p[..3] {
                *c = ((u16::from(*c) * a + 127) / 255) as u8;
            }
        }
    }
    Some((w, h, rgba))
}

const VERTEX: &str = r"#version 100
attribute vec2 pos;
attribute vec2 uv;
attribute vec4 color;
uniform mat4 mvp;
varying vec2 v_uv;
varying vec4 v_color;
void main() {
    v_uv = uv;
    v_color = color;
    gl_Position = mvp * vec4(pos, 0.0, 1.0);
}
";

const FRAGMENT: &str = r"#version 100
precision mediump float;
varying vec2 v_uv;
varying vec4 v_color;
uniform sampler2D tex;
void main() {
    gl_FragColor = texture2D(tex, v_uv) * v_color;
}
";

const FRAGMENT_CLIP: &str = r"#version 100
precision mediump float;
varying vec2 v_uv;
varying vec4 v_color;
uniform sampler2D tex;
void main() {
    vec4 c = texture2D(tex, v_uv) * v_color;
    if (c.a <= 0.05) { discard; }
    gl_FragColor = c;
}
";
