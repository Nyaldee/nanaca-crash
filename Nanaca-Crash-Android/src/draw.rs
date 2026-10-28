//! Le dessin du graphe de scène (CCNode::visit) : enfants d'ordre z négatif,
//! le nœud, puis les autres ; opacité et couleur transmises si le parent
//! les fait cascader.

use std::collections::HashMap;
use std::rc::Rc;

use crate::assets::{Assets, CONTENT_SCALE};
use crate::geom::Affine;
use crate::node::{Graph, Image, Kind, NodeId, Sprite};
use crate::render::{Blend, Renderer, Texture};

/// Les textures des fichiers, chargées au premier dessin, et si elles sont
/// passées en filtrage sans lissage (setAliasTexParameters).
#[derive(Default)]
pub struct Textures {
    files: HashMap<Rc<str>, (Option<Texture>, bool)>,
}

impl Textures {
    fn get(&mut self, renderer: &mut Renderer, assets: &Assets, image: &Image) -> Option<Texture> {
        match image {
            Image::File(path) => {
                let entry = self.files.entry(path.clone()).or_insert_with(|| {
                    let texture = crate::render::decode_image(&assets.bytes(path).unwrap_or_default()).map(|(w, h, rgba)| renderer.texture(w, h, &rgba));
                    (texture, false)
                });
                let alias = assets.is_aliased(path);
                if let Some(t) = entry.0
                    && entry.1 != alias
                {
                    renderer.set_smooth(t, !alias);
                    entry.1 = alias;
                }
                entry.0
            }
            Image::Text(t) => {
                if t.texture.get().is_none() {
                    t.texture.set(Some(renderer.texture(t.width, t.height, &t.rgba)));
                }
                t.texture.get()
            }
        }
    }

    /// Une texture de fichier (étiquettes à cases, polices).
    fn file(&mut self, renderer: &mut Renderer, assets: &Assets, path: &Rc<str>) -> Option<Texture> {
        self.get(renderer, assets, &Image::File(path.clone()))
    }
}

/// Ce qu'un parent transmet : sa transformation.
#[derive(Clone, Copy)]
struct Inherit {
    world: Affine,
}

pub struct Drawer<'a> {
    pub graph: &'a Graph,
    pub assets: &'a Assets,
    pub renderer: &'a mut Renderer,
    pub textures: &'a mut Textures,
}

impl Drawer<'_> {
    /// Dessine une scène entière.
    pub fn scene(&mut self, root: NodeId) {
        self.visit(root, Inherit { world: Affine::IDENTITY });
    }

    fn visit(&mut self, id: NodeId, parent: Inherit) {
        let Some(n) = self.graph.get(id) else { return };
        if !n.visible {
            return;
        }
        let world = n.transform().then(&parent.world);
        // Couleur et opacité affichées : déjà transmises par les parents.
        let opacity = f32::from(n.displayed_opacity) / 255.0;
        let color = n.displayed_color.map(|c| f32::from(c) / 255.0);
        let inherit = Inherit { world };
        let children = self.graph.sorted_children(id);
        let split = children.iter().position(|&c| self.graph.get(c).is_some_and(|c| c.z >= 0)).unwrap_or(children.len());
        if let Kind::Clip { stencil, inverted } = n.kind {
            // La forme de la découpe, dessinée dans le stencil seulement.
            if let Some(s) = stencil.filter(|_| !inverted) {
                self.renderer.clip_begin();
                self.visit(s, inherit);
                self.renderer.clip_apply();
                for &c in &children {
                    self.visit(c, inherit);
                }
                self.renderer.clip_remove_begin();
                self.visit(s, inherit);
                self.renderer.clip_end();
            } else {
                for &c in &children {
                    self.visit(c, inherit);
                }
            }
            return;
        }
        for &c in &children[..split] {
            self.visit(c, inherit);
        }
        let rgba = [color[0] * opacity, color[1] * opacity, color[2] * opacity, opacity];
        match &n.kind {
            Kind::Sprite(s) => self.sprite(s, &world, n.size, rgba),
            Kind::Color(layer) => {
                let (w, h) = n.size;
                let corners = [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)].map(|(x, y)| world.apply(x, y));
                let colors = layer.corners.map(|c| {
                    let a = f32::from(c[3]) / 255.0 * opacity;
                    [f32::from(c[0]) / 255.0 * color[0] * a, f32::from(c[1]) / 255.0 * color[1] * a, f32::from(c[2]) / 255.0 * color[2] * a, a]
                });
                self.renderer.quad_colors(None, corners, [(0.0, 0.0); 4], colors, layer.blend);
            }
            Kind::Scale9(s) => self.scale9(s, &world, n.size, rgba),
            Kind::Draw(triangles) => {
                for (p, c) in triangles {
                    let a = c[3] * opacity;
                    let col = [c[0] * color[0] * a, c[1] * color[1] * a, c[2] * color[2] * a, a];
                    let corners = [p[0], p[1], p[2], p[2]].map(|(x, y)| world.apply(x, y));
                    self.renderer.quad(None, corners, [(0.0, 0.0); 4], col, Blend::Normal);
                }
            }
            Kind::Atlas(a) => {
                if let Some(tex) = self.textures.file(self.renderer, self.assets, &a.image) {
                    let (iw, ih) = a.item;
                    let (pw, ph) = (iw * CONTENT_SCALE, ih * CONTENT_SCALE);
                    let per_row = (tex.width / pw).floor().max(1.0) as u32;
                    for (i, c) in a.text.chars().enumerate() {
                        let k = (c as u32).wrapping_sub(a.start as u32);
                        let (col, row) = (k % per_row, k / per_row);
                        let (l0, r0) = (col as f32 * pw / tex.width, (col as f32 * pw + pw) / tex.width);
                        let (t0, b0) = (row as f32 * ph / tex.height, (row as f32 * ph + ph) / tex.height);
                        let x = i as f32 * iw;
                        let corners = [(x, 0.0), (x + iw, 0.0), (x, ih), (x + iw, ih)].map(|(x, y)| world.apply(x, y));
                        self.renderer.quad(Some(tex.id), corners, [(l0, b0), (r0, b0), (l0, t0), (r0, t0)], rgba, Blend::Normal);
                    }
                }
            }
            Kind::Progress(p) => {
                // Barre (CCProgressTimer::updateBar) : la part visible de
                // l'image autour du point milieu, ramenée dans l'image.
                let f = (p.percent / 100.0).clamp(0.0, 1.0);
                let half = |rate: f32| ((1.0 - rate) + f * rate) * 0.5;
                let (mut x0, mut x1) = (p.midpoint.0 - half(p.bar_rate.0), p.midpoint.0 + half(p.bar_rate.0));
                let (mut y0, mut y1) = (p.midpoint.1 - half(p.bar_rate.1), p.midpoint.1 + half(p.bar_rate.1));
                if x0 < 0.0 {
                    x1 -= x0;
                    x0 = 0.0;
                }
                if x1 > 1.0 {
                    x0 -= x1 - 1.0;
                    x1 = 1.0;
                }
                if y0 < 0.0 {
                    y1 -= y0;
                    y0 = 0.0;
                }
                if y1 > 1.0 {
                    y0 -= y1 - 1.0;
                    y1 = 1.0;
                }
                self.sprite_part(&p.sprite, &world, n.size, rgba, [x0, y0, x1, y1]);
            }
            Kind::Armature(a) => a.draw(self, &world, rgba),
            Kind::Node | Kind::Label(_) | Kind::Menu { .. } | Kind::Item(_) | Kind::Clip { .. } => {}
        }
        self.rest(&children[split..], inherit);
    }

    fn rest(&mut self, children: &[NodeId], inherit: Inherit) {
        for &c in children {
            self.visit(c, inherit);
        }
    }

    pub fn sprite(&mut self, s: &Sprite, world: &Affine, size: (f32, f32), rgba: [f32; 4]) {
        self.sprite_part(s, world, size, rgba, [0.0, 0.0, 1.0, 1.0]);
    }

    /// Une partie (fractions x0, y0, x1, y1 de l'image) d'un sprite ; sans
    /// image, un rectangle de couleur (CCSprite sans texture).
    fn sprite_part(&mut self, s: &Sprite, world: &Affine, _size: (f32, f32), rgba: [f32; 4], part: [f32; 4]) {
        let (w, h) = (s.rect[2] / CONTENT_SCALE, s.rect[3] / CONTENT_SCALE);
        let (x, y) = s.offset;
        let local = [x + w * part[0], y + h * part[1], x + w * part[2], y + h * part[3]];
        self.image_quad(s, world, rgba, local, part);
    }

    /// Une image étirée en neuf parties à la taille du nœud : coins fixes,
    /// bords et centre étirés (CCScale9Sprite).
    fn scale9(&mut self, s: &crate::node::Scale9, world: &Affine, size: (f32, f32), rgba: [f32; 4]) {
        let (w, h) = s.original;
        if w <= 0.0 || h <= 0.0 {
            return;
        }
        // Marges : gauche, droite ; haut et bas (le y des marges descend).
        let [ix, iy, iw, ih] = s.insets;
        let (left, right) = (ix, w - ix - iw);
        let (top, bottom) = (iy, h - iy - ih);
        let xs = [0.0, left, (size.0 - right).max(left), size.0.max(left + right)];
        let ys = [0.0, bottom, (size.1 - top).max(bottom), size.1.max(bottom + top)];
        let fx = [0.0, left / w, (w - right) / w, 1.0];
        let fy = [0.0, bottom / h, (h - top) / h, 1.0];
        let mut sprite = s.sprite.clone();
        sprite.offset = (0.0, 0.0);
        for i in 0..3 {
            for j in 0..3 {
                self.image_quad(&sprite, world, rgba, [xs[i], ys[j], xs[i + 1], ys[j + 1]], [fx[i], fy[j], fx[i + 1], fy[j + 1]]);
            }
        }
    }

    /// Un rectangle du nœud (`local` : x0, y0, x1, y1 en points) peint
    /// d'une partie de l'image (fractions).
    fn image_quad(&mut self, s: &Sprite, world: &Affine, rgba: [f32; 4], local: [f32; 4], part: [f32; 4]) {
        let corners = [(local[0], local[1]), (local[2], local[1]), (local[0], local[3]), (local[2], local[3])].map(|(x, y)| world.apply(x, y));
        let Some(image) = &s.image else {
            self.renderer.quad(None, corners, [(0.0, 0.0); 4], rgba, s.blend);
            return;
        };
        let Some(tex) = self.textures.get(self.renderer, self.assets, image) else { return };
        let [rx, ry, rw, rh] = s.rect;
        // Les coordonnées de texture de chaque coin (bas gauche, bas
        // droite, haut gauche, haut droite), retournements compris.
        let uv = |fx: f32, fy: f32| {
            let fx = if s.flip_x { 1.0 - fx } else { fx };
            let fy = if s.flip_y { 1.0 - fy } else { fy };
            if s.rotated {
                // Tournée de 90° dans la feuille : x de l'image le long du
                // y de la texture, y de l'image le long du x.
                ((rx + rh * fy) / tex.width, (ry + rw * fx) / tex.height)
            } else {
                ((rx + rw * fx) / tex.width, (ry + rh * (1.0 - fy)) / tex.height)
            }
        };
        let uvs = [uv(part[0], part[1]), uv(part[2], part[1]), uv(part[0], part[3]), uv(part[2], part[3])];
        self.renderer.quad(Some(tex.id), corners, uvs, rgba, s.blend);
    }
}
