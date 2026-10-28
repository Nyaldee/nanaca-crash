//! Les formes en triangles pour la carte graphique, sans triangulation :
//! chaque remplissage est dessiné en deux temps (« stencil puis
//! couverture »). Ses bords forment avec un point fixe des triangles en
//! éventail ; dessinés en inversant un bit du stencil, ils marquent
//! exactement l'intérieur de la région (règle pair-impair), quel que soit
//! l'ordre des bords. Un rectangle qui couvre la région la peint ensuite là
//! où le bit est marqué.

use crate::geom::Rect;
use crate::library::{Fill, Layer, Line, Morph, Segment, Shape};

/// Écart maximal toléré entre une courbe et ses segments, en pixels de la
/// forme (le jeu agrandit jusqu'à environ ×4 à l'écran).
const TOLERANCE: f32 = 0.02;

pub struct MeshData {
    pub vertices: Vec<[f32; 2]>,
    pub items: Vec<Item>,
}

/// Un remplissage ou un trait : ses triangles de stencil, puis son
/// rectangle de couverture (6 sommets), dans `vertices`.
pub struct Item {
    pub fill: Fill,
    /// Les triangles s'ajoutent (trait) au lieu de s'inverser (remplissage).
    pub stroke: bool,
    pub stencil: (u32, u32),
    pub cover: (u32, u32),
}

impl MeshData {
    fn new() -> MeshData {
        MeshData { vertices: Vec::new(), items: Vec::new() }
    }

    fn fill(&mut self, fill: &Fill, segments: &[Segment]) {
        let start = self.vertices.len() as u32;
        let pivot = match segments.first() {
            Some(Segment::Line(p, _) | Segment::Curve(p, _, _)) => *p,
            None => return,
        };
        let mut bounds = Rect::EMPTY;
        let mut points = Vec::new();
        for s in segments {
            points.clear();
            flatten(s, &mut points);
            for w in points.windows(2) {
                self.vertices.extend_from_slice(&[pivot, w[0], w[1]]);
                bounds = bounds.with(w[0][0], w[0][1]).with(w[1][0], w[1][1]);
            }
        }
        self.finish(fill.clone(), false, start, bounds);
    }

    fn line(&mut self, line: &Line, segments: &[Segment]) {
        let start = self.vertices.len() as u32;
        // Les traits plus fins qu'un pixel en gardent un, comme dans Flash.
        let half = line.width.max(1.0) / 2.0;
        let mut bounds = Rect::EMPTY;
        let mut points = Vec::new();
        for s in segments {
            points.clear();
            flatten(s, &mut points);
            for w in points.windows(2) {
                let ([ax, ay], [bx, by]) = (w[0], w[1]);
                let len = (bx - ax).hypot(by - ay);
                if len > 0.0 {
                    let (nx, ny) = (-(by - ay) / len * half, (bx - ax) / len * half);
                    let q = [[ax + nx, ay + ny], [bx + nx, by + ny], [bx - nx, by - ny], [ax - nx, ay - ny]];
                    self.vertices.extend_from_slice(&[q[0], q[1], q[2], q[0], q[2], q[3]]);
                }
            }
            // Jointures et extrémités rondes.
            for &p in &points {
                self.disc(p, half);
                bounds = bounds.with(p[0] - half, p[1] - half).with(p[0] + half, p[1] + half);
            }
        }
        self.finish(Fill::Solid(line.color), true, start, bounds);
    }

    fn disc(&mut self, [x, y]: [f32; 2], r: f32) {
        const SIDES: usize = 12;
        for i in 0..SIDES {
            let (a0, a1) = (i as f32 / SIDES as f32 * std::f32::consts::TAU, (i + 1) as f32 / SIDES as f32 * std::f32::consts::TAU);
            self.vertices.extend_from_slice(&[[x, y], [x + r * a0.cos(), y + r * a0.sin()], [x + r * a1.cos(), y + r * a1.sin()]]);
        }
    }

    fn finish(&mut self, fill: Fill, stroke: bool, start: u32, b: Rect) {
        let end = self.vertices.len() as u32;
        if end == start || b.is_empty() {
            self.vertices.truncate(start as usize);
            return;
        }
        let cover = [[b.x_min, b.y_min], [b.x_max, b.y_min], [b.x_max, b.y_max], [b.x_min, b.y_min], [b.x_max, b.y_max], [b.x_min, b.y_max]];
        self.vertices.extend_from_slice(&cover);
        self.items.push(Item { fill, stroke, stencil: (start, end - start), cover: (end, 6) });
    }
}

/// Les points d'un segment, son départ compris.
fn flatten(s: &Segment, out: &mut Vec<[f32; 2]>) {
    match *s {
        Segment::Line(a, b) => out.extend_from_slice(&[a, b]),
        Segment::Curve(a, c, b) => {
            // L'écart d'une quadratique à sa corde vaut au plus |a − 2c + b| / 4.
            let dev = (a[0] - 2.0 * c[0] + b[0]).hypot(a[1] - 2.0 * c[1] + b[1]) / 4.0;
            let n = ((dev / TOLERANCE).sqrt().ceil() as usize).clamp(1, 64);
            out.push(a);
            for i in 1..=n {
                let t = i as f32 / n as f32;
                let u = 1.0 - t;
                out.push([u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0], u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1]]);
            }
        }
    }
}

/// Le point est-il dans la région bordée par ces bords (pair-impair) ?
pub fn inside(segments: &[Segment], x: f32, y: f32) -> bool {
    let mut points = Vec::new();
    let mut odd = false;
    for s in segments {
        points.clear();
        flatten(s, &mut points);
        for w in points.windows(2) {
            let ([ax, ay], [bx, by]) = (w[0], w[1]);
            if (ay > y) != (by > y) && x < ax + (y - ay) / (by - ay) * (bx - ax) {
                odd = !odd;
            }
        }
    }
    odd
}

/// Une forme : chaque calque, ses remplissages puis ses traits.
pub fn shape(shape: &Shape) -> MeshData {
    layers(&shape.layers)
}

pub fn layers(layers: &[Layer]) -> MeshData {
    let mut mesh = MeshData::new();
    for layer in layers {
        for (fill, segments) in &layer.fills {
            mesh.fill(fill, segments);
        }
        for (line, segments) in &layer.lines {
            mesh.line(line, segments);
        }
    }
    mesh
}

/// Un glyphe : un seul remplissage, teinté au dessin.
pub fn glyph(segments: &[Segment]) -> MeshData {
    let mut mesh = MeshData::new();
    mesh.fill(&Fill::Solid([1.0; 4]), segments);
    mesh
}

/// Une forme qui se transforme, à l'étape `ratio` (0..65535).
pub fn morph(m: &Morph, ratio: u16) -> MeshData {
    let t = f32::from(ratio) / 65535.0;
    let lerp = |a: f32, b: f32| a + (b - a) * t;
    let point = |a: [f32; 2], b: [f32; 2]| [lerp(a[0], b[0]), lerp(a[1], b[1])];
    let color = |a: [f32; 4], b: [f32; 4]| [lerp(a[0], b[0]), lerp(a[1], b[1]), lerp(a[2], b[2]), lerp(a[3], b[3])];
    let matrix = |a: crate::geom::Matrix, b: crate::geom::Matrix| crate::geom::Matrix {
        a: lerp(a.a, b.a),
        b: lerp(a.b, b.b),
        c: lerp(a.c, b.c),
        d: lerp(a.d, b.d),
        tx: lerp(a.tx, b.tx),
        ty: lerp(a.ty, b.ty),
    };
    let fills: Vec<Fill> = m
        .fills
        .iter()
        .map(|(a, b)| match (a, b) {
            (Fill::Solid(x), Fill::Solid(y)) => Fill::Solid(color(*x, *y)),
            (Fill::Gradient(x), Fill::Gradient(y)) => {
                let mut g = x.clone();
                g.matrix = matrix(x.matrix, y.matrix);
                for (s, e) in g.stops.iter_mut().zip(&y.stops) {
                    *s = (lerp(s.0, e.0), color(s.1, e.1));
                }
                Fill::Gradient(g)
            }
            (Fill::Bitmap { id, matrix: x, repeat, smooth }, Fill::Bitmap { matrix: y, .. }) => Fill::Bitmap { id: *id, matrix: matrix(*x, *y), repeat: *repeat, smooth: *smooth },
            (a, _) => a.clone(),
        })
        .collect();
    let lines: Vec<Line> = m.lines.iter().map(|(a, b)| Line { width: lerp(a.width, b.width), color: color(a.color, b.color) }).collect();
    // Les bords de fin, appariés un à un avec ceux du début ; une droite
    // face à une courbe devient une courbe de contrôle central.
    let as_curve = |s: Segment| match s {
        Segment::Line(a, b) => (a, [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0], b),
        Segment::Curve(a, c, b) => (a, c, b),
    };
    let edges: Vec<crate::library::Edge> = m
        .start
        .iter()
        .zip(&m.end)
        .map(|(s, e)| {
            let segment = match (s.segment, e.segment) {
                (Segment::Line(a0, b0), Segment::Line(a1, b1)) => Segment::Line(point(a0, a1), point(b0, b1)),
                (x, y) => {
                    let ((a0, c0, b0), (a1, c1, b1)) = (as_curve(x), as_curve(y));
                    Segment::Curve(point(a0, a1), point(c0, c1), point(b0, b1))
                }
            };
            crate::library::Edge { segment, ..*s }
        })
        .collect();
    layers(&[crate::swf::layer(&fills, &lines, &edges)])
}
