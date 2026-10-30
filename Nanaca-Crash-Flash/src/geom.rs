//! Géométrie de Flash : matrices 2D, transformations de couleur,
//! rectangles. Les unités sont des pixels (le SWF stocke des vingtièmes de
//! pixel).

use std::ops::Mul;

/// x' = a·x + c·y + tx, y' = b·x + d·y + ty.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Matrix {
    pub const IDENTITY: Matrix = Matrix { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };

    pub fn translate(x: f32, y: f32) -> Matrix {
        Matrix { tx: x, ty: y, ..Matrix::IDENTITY }
    }

    pub fn scale(x: f32, y: f32) -> Matrix {
        Matrix { a: x, d: y, ..Matrix::IDENTITY }
    }

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.tx, self.b * x + self.d * y + self.ty)
    }

    pub fn invert(&self) -> Matrix {
        let det = self.a * self.d - self.b * self.c;
        if det.abs() < 1e-12 {
            return Matrix { a: 0.0, b: 0.0, c: 0.0, d: 0.0, tx: -self.tx, ty: -self.ty };
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Matrix { a, b, c, d, tx: -(a * self.tx + c * self.ty), ty: -(b * self.tx + d * self.ty) }
    }

    /// En 4×4 par colonnes, pour les shaders.
    pub fn to_mat4(&self) -> [f32; 16] {
        [self.a, self.b, 0.0, 0.0, self.c, self.d, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, self.tx, self.ty, 0.0, 1.0]
    }
}

/// `self * autre` : `autre` s'applique en premier.
impl Mul for Matrix {
    type Output = Matrix;
    fn mul(self, o: Matrix) -> Matrix {
        Matrix {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            tx: self.a * o.tx + self.c * o.ty + self.tx,
            ty: self.b * o.tx + self.d * o.ty + self.ty,
        }
    }
}

/// couleur' = couleur · mult + add (RGBA, 0..1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ColorTransform {
    pub mult: [f32; 4],
    pub add: [f32; 4],
}

impl ColorTransform {
    pub const IDENTITY: ColorTransform = ColorTransform { mult: [1.0; 4], add: [0.0; 4] };
}

/// `self * autre` : `autre` s'applique en premier.
impl Mul for ColorTransform {
    type Output = ColorTransform;
    fn mul(self, o: ColorTransform) -> ColorTransform {
        let mut r = ColorTransform::IDENTITY;
        for i in 0..4 {
            r.mult[i] = self.mult[i] * o.mult[i];
            r.add[i] = self.mult[i] * o.add[i] + self.add[i];
        }
        r
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x_min: f32,
    pub y_min: f32,
    pub x_max: f32,
    pub y_max: f32,
}

impl Rect {
    /// Le rectangle vide, neutre pour `union`.
    pub const EMPTY: Rect = Rect { x_min: f32::MAX, y_min: f32::MAX, x_max: f32::MIN, y_max: f32::MIN };

    pub fn is_empty(&self) -> bool {
        self.x_min > self.x_max || self.y_min > self.y_max
    }

    pub fn union(self, o: Rect) -> Rect {
        Rect { x_min: self.x_min.min(o.x_min), y_min: self.y_min.min(o.y_min), x_max: self.x_max.max(o.x_max), y_max: self.y_max.max(o.y_max) }
    }

    /// Agrandi pour contenir un point.
    pub fn with(self, x: f32, y: f32) -> Rect {
        Rect { x_min: self.x_min.min(x), y_min: self.y_min.min(y), x_max: self.x_max.max(x), y_max: self.y_max.max(y) }
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= self.x_min && x <= self.x_max && y >= self.y_min && y <= self.y_max
    }

    /// Le rectangle englobant ce rectangle transformé.
    pub fn transform(&self, m: Matrix) -> Rect {
        if self.is_empty() {
            return *self;
        }
        [(self.x_min, self.y_min), (self.x_max, self.y_min), (self.x_min, self.y_max), (self.x_max, self.y_max)]
            .iter()
            .fold(Rect::EMPTY, |r, &(x, y)| {
                let (x, y) = m.apply(x, y);
                r.with(x, y)
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inverse() {
        let m = Matrix { a: 2.0, b: 0.5, c: -1.0, d: 3.0, tx: 10.0, ty: -4.0 };
        let p = (m.invert() * m).apply(7.0, 9.0);
        assert!((p.0 - 7.0).abs() < 1e-4 && (p.1 - 9.0).abs() < 1e-4);
    }
}
