//! Lecture des données d'un SWF : entiers petit-boutistes, champs de bits,
//! chaînes, et les structures communes (rectangle, matrice, couleurs).
//! Toute lecture hors des données rend `None`.

use crate::geom::{ColorTransform, Matrix, Rect};

/// Vingtièmes de pixel par pixel.
pub const TWIPS: f32 = 20.0;

pub struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
    /// Octet en cours de lecture bit à bit, et ses bits restants.
    bits: u8,
    bits_left: u8,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Reader { data, pos: 0, bits: 0, bits_left: 0 }
    }

    pub fn is_empty(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Reprend aux octets entiers après une lecture de bits.
    pub fn align(&mut self) {
        self.bits_left = 0;
    }

    pub fn bytes(&mut self, n: usize) -> Option<&'a [u8]> {
        self.align();
        let b = self.data.get(self.pos..self.pos.checked_add(n)?)?;
        self.pos += n;
        Some(b)
    }

    /// Tout ce qui reste.
    pub fn rest(&mut self) -> &'a [u8] {
        self.align();
        let b = self.data.get(self.pos..).unwrap_or(&[]);
        self.pos = self.data.len();
        b
    }

    pub fn u8(&mut self) -> Option<u8> {
        Some(self.bytes(1)?[0])
    }

    pub fn u16(&mut self) -> Option<u16> {
        let b = self.bytes(2)?;
        Some(u16::from_le_bytes([b[0], b[1]]))
    }

    pub fn i16(&mut self) -> Option<i16> {
        Some(self.u16()? as i16)
    }

    pub fn u32(&mut self) -> Option<u32> {
        let b = self.bytes(4)?;
        Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// Nombre à virgule fixe 8.8.
    pub fn fixed8(&mut self) -> Option<f32> {
        Some(f32::from(self.i16()?) / 256.0)
    }

    /// Chaîne terminée par un zéro (UTF-8 depuis Flash 6).
    pub fn string(&mut self) -> Option<String> {
        self.align();
        let len = self.data.get(self.pos..)?.iter().position(|&b| b == 0)?;
        let s = String::from_utf8_lossy(&self.data[self.pos..self.pos + len]).into_owned();
        self.pos += len + 1;
        Some(s)
    }

    /// `n` bits non signés, poids fort d'abord.
    pub fn ubits(&mut self, n: u32) -> Option<u32> {
        let mut v = 0u32;
        for _ in 0..n {
            if self.bits_left == 0 {
                self.bits = *self.data.get(self.pos)?;
                self.pos += 1;
                self.bits_left = 8;
            }
            self.bits_left -= 1;
            v = (v << 1) | u32::from((self.bits >> self.bits_left) & 1);
        }
        Some(v)
    }

    /// `n` bits signés.
    pub fn sbits(&mut self, n: u32) -> Option<i32> {
        let v = self.ubits(n)?;
        Some(if n > 0 && n < 32 && v & (1 << (n - 1)) != 0 { (v | !((1u32 << n) - 1)) as i32 } else { v as i32 })
    }

    /// `n` bits signés en virgule fixe 16.16.
    pub fn fbits(&mut self, n: u32) -> Option<f32> {
        Some(self.sbits(n)? as f32 / 65536.0)
    }

    pub fn flag(&mut self) -> Option<bool> {
        Some(self.ubits(1)? == 1)
    }

    /// Un rectangle, en pixels.
    pub fn rect(&mut self) -> Option<Rect> {
        self.align();
        let n = self.ubits(5)?;
        let mut v = [0.0; 4];
        for x in &mut v {
            *x = self.sbits(n)? as f32 / TWIPS;
        }
        self.align();
        Some(Rect { x_min: v[0], x_max: v[1], y_min: v[2], y_max: v[3] })
    }

    /// Une matrice ; sa translation en pixels.
    pub fn matrix(&mut self) -> Option<Matrix> {
        self.align();
        let mut m = Matrix::IDENTITY;
        if self.flag()? {
            let n = self.ubits(5)?;
            m.a = self.fbits(n)?;
            m.d = self.fbits(n)?;
        }
        if self.flag()? {
            let n = self.ubits(5)?;
            m.b = self.fbits(n)?;
            m.c = self.fbits(n)?;
        }
        let n = self.ubits(5)?;
        m.tx = self.sbits(n)? as f32 / TWIPS;
        m.ty = self.sbits(n)? as f32 / TWIPS;
        self.align();
        Some(m)
    }

    /// Une transformation de couleur, avec ou sans alpha.
    pub fn color_transform(&mut self, alpha: bool) -> Option<ColorTransform> {
        self.align();
        let has_add = self.flag()?;
        let has_mult = self.flag()?;
        let n = self.ubits(4)?;
        let channels = if alpha { 4 } else { 3 };
        let mut c = ColorTransform::IDENTITY;
        if has_mult {
            for i in 0..channels {
                c.mult[i] = self.sbits(n)? as f32 / 256.0;
            }
        }
        if has_add {
            for i in 0..channels {
                c.add[i] = self.sbits(n)? as f32 / 255.0;
            }
        }
        self.align();
        Some(c)
    }

    pub fn rgb(&mut self) -> Option<[f32; 4]> {
        let b = self.bytes(3)?;
        Some([f32::from(b[0]) / 255.0, f32::from(b[1]) / 255.0, f32::from(b[2]) / 255.0, 1.0])
    }

    pub fn rgba(&mut self) -> Option<[f32; 4]> {
        let b = self.bytes(4)?;
        Some([f32::from(b[0]) / 255.0, f32::from(b[1]) / 255.0, f32::from(b[2]) / 255.0, f32::from(b[3]) / 255.0])
    }
}

#[cfg(test)]
mod tests {
    use super::Reader;

    #[test]
    fn bits() {
        let mut r = Reader::new(&[0b1110_0000, 0xFF]);
        assert_eq!(r.ubits(1), Some(1));
        assert_eq!(r.sbits(3), Some(-2));
        assert_eq!(r.u8(), Some(0xFF));
        assert_eq!(r.u8(), None);
    }
}
