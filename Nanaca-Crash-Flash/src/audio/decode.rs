//! Les sons du SWF en échantillons : MP3 (symphonia), Nellymoser, ADPCM de
//! Flash et PCM.

use std::io::Cursor;

use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::default::codecs::MpaDecoder;
use symphonia::default::formats::MpaReader;

use crate::library::{Codec, Sound};

/// Des échantillons (-1..1), entrelacés en stéréo.
pub struct Pcm {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub stereo: bool,
}

impl Pcm {
    /// Nombre d'instants (un échantillon par canal).
    pub fn frames(&self) -> usize {
        if self.stereo { self.samples.len() / 2 } else { self.samples.len() }
    }

    /// Gauche et droite d'un instant.
    pub fn frame(&self, i: usize) -> (f32, f32) {
        if self.stereo { (self.samples[2 * i], self.samples[2 * i + 1]) } else { (self.samples[i], self.samples[i]) }
    }
}

pub fn decode(sound: &Sound) -> Option<Pcm> {
    let mut samples = match sound.codec {
        // Le MP3 commence par le nombre d'échantillons de latence (2 octets).
        Codec::Mp3 => mp3(sound.data.get(2..)?)?,
        Codec::Nellymoser => super::nellymoser::decode(&sound.data),
        Codec::Adpcm => adpcm(&sound.data, sound.stereo)?,
        Codec::Pcm => pcm(&sound.data, sound.sixteen_bits),
        Codec::Other(_) => return None,
    };
    samples.truncate(sound.samples as usize * if sound.stereo { 2 } else { 1 });
    Some(Pcm { samples, rate: sound.rate, stereo: sound.stereo })
}

fn mp3(data: &[u8]) -> Option<Vec<f32>> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(data.to_vec())), Default::default());
    let mut reader = MpaReader::try_new(source, FormatOptions::default()).ok()?;
    let params = match &reader.default_track(TrackType::Audio)?.codec_params {
        Some(CodecParameters::Audio(p)) => p.clone(),
        _ => return None,
    };
    let mut decoder = MpaDecoder::try_new(&params, &AudioDecoderOptions::default()).ok()?;
    let mut out = Vec::new();
    let mut buffer = Vec::new();
    while let Ok(Some(packet)) = reader.next_packet() {
        let Ok(decoded) = decoder.decode(&packet) else { continue };
        decoded.copy_to_vec_interleaved(&mut buffer);
        out.extend_from_slice(&buffer);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use crate::library::{Character, Codec};

    /// Tous les sons du SWF original se décodent à leur durée exacte.
    #[test]
    fn original() {
        let Ok((data, _)) = crate::original::load("", "") else {
            eprintln!("SWF original absent : test sauté");
            return;
        };
        let lib = crate::swf::load(&data).unwrap();
        let mut ids: Vec<_> = lib.characters.iter().filter_map(|(&id, c)| if let Character::Sound(s) = c { Some((id, s)) } else { None }).collect();
        ids.sort_by_key(|s| s.0);
        for (id, sound) in ids {
            let pcm = super::decode(sound).unwrap_or_else(|| panic!("son {id} non décodé"));
            assert_eq!(pcm.frames(), sound.samples as usize, "son {id}");
            let peak = pcm.samples.iter().fold(0f32, |m, s| m.max(s.abs()));
            let rms = (pcm.samples.iter().map(|s| s * s).sum::<f32>() / pcm.samples.len() as f32).sqrt();
            eprintln!("son {id} {:?} {:.2} s, crête {peak:.3}, moyenne {rms:.4}", sound.codec, pcm.frames() as f32 / pcm.rate as f32);
            if id == 302 {
                assert_eq!(sound.codec, Codec::Nellymoser);
            }
        }
    }
}

fn pcm(data: &[u8], sixteen_bits: bool) -> Vec<f32> {
    if sixteen_bits {
        data.chunks_exact(2).map(|b| f32::from(i16::from_le_bytes([b[0], b[1]])) / 32768.0).collect()
    } else {
        data.iter().map(|&b| (f32::from(b) - 128.0) / 128.0).collect()
    }
}

// --- ADPCM de Flash ----------------------------------------------------------------

/// Pas de quantification (ceux de l'ADPCM IMA).
const STEPS: [i32; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66, 73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230,
    253, 279, 307, 337, 371, 408, 449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066, 2272, 2499, 2749, 3024, 3327,
    3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630, 9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

/// Évolution de l'indice du pas selon la taille des codes (2 à 5 bits) et
/// leur valeur absolue.
const INDEX_CHANGES: [&[i32]; 4] = [&[-1, 2], &[-1, -1, 2, 4], &[-1, -1, -1, -1, 2, 4, 6, 8], &[-1, -1, -1, -1, -1, -1, -1, -1, 1, 2, 4, 6, 8, 10, 13, 16]];

/// Échantillons par canal d'un paquet (chaque paquet repart d'une valeur
/// et d'un pas donnés en clair).
const PACKET: usize = 4096;

/// Lecture de bits, poids fort d'abord.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn read(&mut self, n: u32) -> Option<u32> {
        let mut v = 0;
        for _ in 0..n {
            let byte = *self.data.get(self.pos / 8)?;
            v = (v << 1) | u32::from((byte >> (7 - self.pos % 8)) & 1);
            self.pos += 1;
        }
        Some(v)
    }
}

fn adpcm(data: &[u8], stereo: bool) -> Option<Vec<f32>> {
    let mut bits = Bits { data, pos: 0 };
    let size = bits.read(2)? + 2;
    let changes = INDEX_CHANGES[size as usize - 2];
    let sign = 1 << (size - 1);
    let channels = if stereo { 2 } else { 1 };
    let mut out = Vec::new();
    'packets: loop {
        let mut state = [(0i32, 0usize); 2];
        for s in state.iter_mut().take(channels) {
            let (Some(sample), Some(index)) = (bits.read(16), bits.read(6)) else { break 'packets };
            *s = (i32::from(sample as u16 as i16), index as usize);
            out.push(s.0 as f32 / 32768.0);
        }
        for _ in 1..PACKET {
            for (sample, index) in state.iter_mut().take(channels) {
                let Some(code) = bits.read(size) else { break 'packets };
                let mut step = STEPS[*index];
                let mut delta = step >> (size - 1);
                let mut bit = sign >> 1;
                while bit != 0 {
                    if code & bit != 0 {
                        delta += step;
                    }
                    step >>= 1;
                    bit >>= 1;
                }
                *sample = if code & sign != 0 { *sample - delta } else { *sample + delta }.clamp(-32768, 32767);
                *index = (*index as i32 + changes[(code & (sign - 1)) as usize]).clamp(0, 88) as usize;
                out.push(*sample as f32 / 32768.0);
            }
        }
    }
    Some(out)
}
