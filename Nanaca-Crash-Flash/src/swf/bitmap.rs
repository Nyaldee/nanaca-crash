//! Les images du SWF : JPEG avec transparence (DefineBitsJPEG3) et images
//! sans perte (DefineBitsLossless2), en pixels RGBA prémultipliés, comme le
//! SWF les stocke.

use super::reader::Reader;
use crate::library::Bitmap;

/// DefineBitsJPEG3, après son identifiant : un JPEG, puis sa couche alpha
/// compressée en zlib (un octet par pixel).
pub fn jpeg3(r: &mut Reader) -> Option<Bitmap> {
    let size = r.u32()? as usize;
    let mut bitmap = jpeg(r.bytes(size)?)?;
    let pixels = (bitmap.width * bitmap.height) as usize;
    if let Ok(alpha) = miniz_oxide::inflate::decompress_to_vec_zlib(r.rest())
        && alpha.len() >= pixels
    {
        // Les couleurs du JPEG sont déjà multipliées par cet alpha (au bruit
        // de compression près, borné ici).
        for (px, &a) in bitmap.rgba.chunks_exact_mut(4).zip(&alpha) {
            px[0] = px[0].min(a);
            px[1] = px[1].min(a);
            px[2] = px[2].min(a);
            px[3] = a;
        }
    }
    Some(bitmap)
}

fn jpeg(data: &[u8]) -> Option<Bitmap> {
    // Flash glisse parfois une fin d'image suivie d'un début (FF D9 FF D8)
    // entre deux parties du JPEG ; les décodeurs la refusent.
    let mut data = data.to_vec();
    if let Some(i) = data.windows(4).position(|w| w == [0xFF, 0xD9, 0xFF, 0xD8]) {
        data.drain(i..i + 4);
    }
    let image = image::load_from_memory_with_format(&data, image::ImageFormat::Jpeg).ok()?.into_rgba8();
    Some(Bitmap { width: image.width(), height: image.height(), rgba: image.into_raw() })
}

/// DefineBitsLossless2, après son identifiant : une image à palette
/// (format 3) ou en ARGB prémultiplié (format 5), compressée en zlib.
pub fn lossless2(r: &mut Reader) -> Option<Bitmap> {
    let format = r.u8()?;
    let width = u32::from(r.u16()?);
    let height = u32::from(r.u16()?);
    let palette_size = if format == 3 { usize::from(r.u8()?) + 1 } else { 0 };
    let data = miniz_oxide::inflate::decompress_to_vec_zlib(r.rest()).ok()?;
    let (w, h) = (width as usize, height as usize);
    let mut rgba = Vec::with_capacity(w * h * 4);
    match format {
        3 => {
            let palette = data.get(..palette_size * 4)?;
            // Les lignes d'indices sont alignées sur 4 octets.
            let stride = w.div_ceil(4) * 4;
            for y in 0..h {
                for x in 0..w {
                    let i = usize::from(*data.get(palette_size * 4 + y * stride + x)?);
                    rgba.extend_from_slice(palette.get(i * 4..i * 4 + 4)?);
                }
            }
        }
        5 => {
            for px in data.get(..w * h * 4)?.chunks_exact(4) {
                rgba.extend_from_slice(&[px[1], px[2], px[3], px[0]]);
            }
        }
        _ => return None,
    }
    Some(Bitmap { width, height, rgba })
}
