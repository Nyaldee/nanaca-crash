//! Le son, comme SimpleAudioEngine : une musique de fond et des effets, en
//! OGG Vorbis dans l'APK, décodés à leur premier usage puis mixés sur le fil
//! audio de cpal.

use std::collections::HashMap;
use std::io::Cursor;
use std::sync::{Arc, Mutex, MutexGuard};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use symphonia::core::codecs::CodecParameters;
use symphonia::core::codecs::audio::{AudioDecoder, AudioDecoderOptions};
use symphonia::core::formats::{FormatOptions, FormatReader, TrackType};
use symphonia::core::io::MediaSourceStream;
use symphonia::default::codecs::VorbisDecoder;
use symphonia::default::formats::OggReader;

use crate::assets::Assets;

/// Des échantillons (-1..1), entrelacés en stéréo.
pub struct Pcm {
    samples: Vec<f32>,
    rate: u32,
}

impl Pcm {
    fn frames(&self) -> usize {
        self.samples.len() / 2
    }

    fn frame(&self, i: usize) -> (f32, f32) {
        (self.samples[2 * i], self.samples[2 * i + 1])
    }
}

/// Décode un OGG Vorbis (mono ou stéréo) en stéréo.
fn decode(bytes: Vec<u8>) -> Option<Pcm> {
    let source = MediaSourceStream::new(Box::new(Cursor::new(bytes)), Default::default());
    let mut reader = OggReader::try_new(source, FormatOptions::default()).ok()?;
    let track = reader.default_track(TrackType::Audio)?;
    let id = track.id;
    let params = match &track.codec_params {
        Some(CodecParameters::Audio(p)) => p.clone(),
        _ => return None,
    };
    let rate = params.sample_rate?;
    let channels = params.channels.as_ref().map_or(2, |c| c.count());
    let mut decoder = VorbisDecoder::try_new(&params, &AudioDecoderOptions::default()).ok()?;
    let mut samples = Vec::new();
    let mut buffer: Vec<f32> = Vec::new();
    while let Ok(Some(packet)) = reader.next_packet() {
        if packet.track_id != id {
            continue;
        }
        let Ok(decoded) = decoder.decode(&packet) else { continue };
        decoded.copy_to_vec_interleaved(&mut buffer);
        if channels == 1 {
            samples.extend(buffer.iter().flat_map(|&s| [s, s]));
        } else {
            samples.extend(buffer.chunks_exact(channels).flat_map(|c| [c[0], c[1]]));
        }
    }
    Some(Pcm { samples, rate })
}

struct Voice {
    id: u32,
    music: bool,
    pcm: Arc<Pcm>,
    pos: f64,
    looped: bool,
    /// Où la boucle reprend (images), pour les musiques.
    loop_start: f64,
    gain: f32,
    paused: bool,
}

#[derive(Default)]
struct Mixer {
    voices: Vec<Voice>,
    rate: u32,
    channels: usize,
    music_volume: f32,
    effects_volume: f32,
}

impl Mixer {
    fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let channels = self.channels.max(1);
        let rate = f64::from(self.rate.max(1));
        let (music, effects) = (self.music_volume, self.effects_volume);
        self.voices.retain_mut(|v| {
            if v.paused {
                return true;
            }
            let step = f64::from(v.pcm.rate) / rate;
            let frames = v.pcm.frames();
            if frames == 0 {
                return false;
            }
            let gain = v.gain * if v.music { music } else { effects };
            for frame in out.chunks_exact_mut(channels) {
                if v.pos >= frames as f64 {
                    if !v.looped {
                        return false;
                    }
                    v.pos = v.pos - frames as f64 + v.loop_start.min(frames as f64 - 1.0);
                }
                let (l, r) = cubic(&v.pcm, v.pos, v.looped);
                if channels == 1 {
                    frame[0] += (l + r) / 2.0 * gain;
                } else {
                    frame[0] += l * gain;
                    frame[1] += r * gain;
                }
                v.pos += step;
            }
            true
        });
        for s in out {
            *s = s.clamp(-1.0, 1.0);
        }
    }
}

/// Un instant entre deux échantillons (Hermite cubique), pour passer à la
/// fréquence de la sortie sans grésillement ; en boucle, la fin rejoint le
/// début.
fn cubic(pcm: &Pcm, pos: f64, looped: bool) -> (f32, f32) {
    let n = pcm.frames() as isize;
    let i = pos as isize;
    let t = (pos - i as f64) as f32;
    let at = |k: isize| {
        let j = i + k;
        pcm.frame(if looped { j.rem_euclid(n) } else { j.clamp(0, n - 1) } as usize)
    };
    let (p0, p1, p2, p3) = (at(-1), at(0), at(1), at(2));
    let h = |a: f32, b: f32, c: f32, d: f32| {
        let (c1, c2, c3) = ((c - a) / 2.0, a - 2.5 * b + 2.0 * c - d / 2.0, (d - a) / 2.0 + 1.5 * (b - c));
        ((c3 * t + c2) * t + c1) * t + b
    };
    (h(p0.0, p1.0, p2.0, p3.0), h(p0.1, p1.1, p2.1, p3.1))
}

pub struct Audio {
    mixer: Arc<Mutex<Mixer>>,
    stream: Option<cpal::Stream>,
    cache: HashMap<String, Option<Arc<Pcm>>>,
    /// Sons en cours de décodage (préchargement), sur leur fil.
    pending: HashMap<String, std::thread::JoinHandle<Option<Pcm>>>,
    next_id: u32,
}

impl Audio {
    pub fn new() -> Audio {
        let mut audio = Audio::silent();
        audio.stream = audio.open();
        audio
    }

    pub fn silent() -> Audio {
        let mixer = Mixer { music_volume: 1.0, effects_volume: 1.0, ..Mixer::default() };
        Audio { mixer: Arc::new(Mutex::new(mixer)), stream: None, cache: HashMap::new(), pending: HashMap::new(), next_id: 0 }
    }

    fn open(&self) -> Option<cpal::Stream> {
        let device = cpal::default_host().default_output_device()?;
        let config = device.default_output_config().ok()?.config();
        {
            let mut m = lock(&self.mixer);
            m.rate = config.sample_rate;
            m.channels = usize::from(config.channels);
        }
        let mixer = self.mixer.clone();
        let stream = device.build_output_stream::<f32, _, _>(config, move |out, _| lock(&mixer).render(out), |_| {}, None).ok()?;
        stream.play().ok()?;
        Some(stream)
    }

    fn pcm(&mut self, assets: &Assets, path: &str) -> Option<Arc<Pcm>> {
        if self.stream.is_none() {
            return None;
        }
        if let Some(job) = self.pending.remove(path) {
            let pcm = job.join().ok().flatten().map(Arc::new);
            self.cache.insert(path.to_string(), pcm);
        }
        self.cache.entry(path.to_string()).or_insert_with(|| decode(assets.apk.asset(path)?).map(Arc::new)).clone()
    }

    /// preloadEffect : décodé sur un autre fil, prêt au premier usage.
    pub fn preload(&mut self, assets: &Assets, path: &str) {
        if self.stream.is_none() || self.cache.contains_key(path) || self.pending.contains_key(path) {
            return;
        }
        let Some(bytes) = assets.apk.asset(path) else { return };
        self.pending.insert(path.to_string(), std::thread::spawn(move || decode(bytes)));
    }

    /// playEffect : rend l'identifiant de la voix (0 : rien).
    pub fn play_effect(&mut self, assets: &Assets, path: &str, looped: bool, gain: f32) -> u32 {
        let Some(pcm) = self.pcm(assets, path) else { return 0 };
        self.next_id += 1;
        let id = self.next_id;
        lock(&self.mixer).voices.push(Voice { id, music: false, pcm, pos: 0.0, looped, loop_start: 0.0, gain, paused: false });
        id
    }

    pub fn stop_effect(&mut self, id: u32) {
        lock(&self.mixer).voices.retain(|v| v.id != id || v.music);
    }

    pub fn stop_all_effects(&mut self) {
        lock(&self.mixer).voices.retain(|v| v.music);
    }

    /// playBackgroundMusic : remplace la musique en cours ; en boucle, elle
    /// reprend à `loop_start` secondes.
    pub fn play_music(&mut self, assets: &Assets, path: &str, looped: bool, loop_start: f32) {
        self.stop_music();
        let Some(pcm) = self.pcm(assets, path) else { return };
        self.next_id += 1;
        let id = self.next_id;
        let loop_start = f64::from(loop_start.max(0.0)) * f64::from(pcm.rate);
        lock(&self.mixer).voices.push(Voice { id, music: true, pcm, pos: 0.0, looped, loop_start, gain: 1.0, paused: false });
    }

    pub fn stop_music(&mut self) {
        lock(&self.mixer).voices.retain(|v| !v.music);
    }

    pub fn pause_music(&mut self, paused: bool) {
        for v in lock(&self.mixer).voices.iter_mut().filter(|v| v.music) {
            v.paused = paused;
        }
    }

    pub fn music_playing(&self) -> bool {
        lock(&self.mixer).voices.iter().any(|v| v.music && !v.paused)
    }

    pub fn music_volume(&self) -> f32 {
        lock(&self.mixer).music_volume
    }

    pub fn effects_volume(&self) -> f32 {
        lock(&self.mixer).effects_volume
    }

    pub fn set_music_volume(&mut self, volume: f32) {
        lock(&self.mixer).music_volume = volume.clamp(0.0, 1.0);
    }

    pub fn set_effects_volume(&mut self, volume: f32) {
        lock(&self.mixer).effects_volume = volume.clamp(0.0, 1.0);
    }
}

/// Le mixeur, même si l'autre fil a paniqué en le tenant.
fn lock(mixer: &Mutex<Mixer>) -> MutexGuard<'_, Mixer> {
    mixer.lock().unwrap_or_else(|e| e.into_inner())
}

#[cfg(test)]
mod tests {
    /// Les musiques et sons de l'APK se décodent.
    #[test]
    fn decode() {
        let Ok((apk, _)) = crate::original::Apk::load("", "") else { return };
        for name in ["bgmogg/bgm0.ogg", "seogg/se01.ogg"] {
            let pcm = super::decode(apk.asset(name).unwrap()).unwrap_or_else(|| panic!("{name}"));
            assert!(pcm.frames() > 1000, "{name}");
            assert!(pcm.samples.iter().any(|s| s.abs() > 0.01), "{name}");
        }
    }
}
