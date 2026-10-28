//! Transformations affines, à la manière de Cocos2d : x' = a x + c y + tx,
//! y' = b x + d y + ty, y vers le haut.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub tx: f32,
    pub ty: f32,
}

impl Affine {
    pub const IDENTITY: Affine = Affine { a: 1.0, b: 0.0, c: 0.0, d: 1.0, tx: 0.0, ty: 0.0 };

    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.tx, self.b * x + self.d * y + self.ty)
    }

    /// `self` d'abord, puis `then` (CCAffineTransformConcat).
    pub fn then(&self, then: &Affine) -> Affine {
        Affine {
            a: self.a * then.a + self.b * then.c,
            b: self.a * then.b + self.b * then.d,
            c: self.c * then.a + self.d * then.c,
            d: self.c * then.b + self.d * then.d,
            tx: self.tx * then.a + self.ty * then.c + then.tx,
            ty: self.tx * then.b + self.ty * then.d + then.ty,
        }
    }

    pub fn translate(&self, x: f32, y: f32) -> Affine {
        Affine { tx: self.tx + self.a * x + self.c * y, ty: self.ty + self.b * x + self.d * y, ..*self }
    }

    pub fn invert(&self) -> Affine {
        let det = self.a * self.d - self.b * self.c;
        if det == 0.0 {
            return Affine::IDENTITY;
        }
        let k = 1.0 / det;
        Affine {
            a: k * self.d,
            b: -k * self.b,
            c: -k * self.c,
            d: k * self.a,
            tx: k * (self.c * self.ty - self.d * self.tx),
            ty: k * (self.b * self.tx - self.a * self.ty),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Affine;

    #[test]
    fn invert() {
        let m = Affine { a: 2.0, b: 0.5, c: -1.0, d: 3.0, tx: 4.0, ty: -2.0 };
        let (x, y) = m.apply(1.5, -7.0);
        let (u, v) = m.invert().apply(x, y);
        assert!((u - 1.5).abs() < 1e-4 && (v + 7.0).abs() < 1e-4);
        let (p, q) = m.then(&m.invert()).apply(3.0, 2.0);
        assert!((p - 3.0).abs() < 1e-4 && (q - 2.0).abs() < 1e-4);
    }
}
