//! La sortie du son. Les sons du SWF sont décodés à leur premier usage, puis
//! mixés sur le fil audio de cpal. Comme dans Flash, chaque lancement est
//! une nouvelle voix ; son volume, fixé par la logique du jeu, est suivi à
//! chaque image.

mod decode;
mod nellymoser;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

use crate::display::Obj;
use crate::library::{Character, Id, Library, StartSound};
use decode::Pcm;

/// Fréquence des positions des sons de timeline (début, fin, enveloppe).
const ENVELOPE_RATE: f64 = 44100.0;

struct Voice {
    /// Identifiant de la voix, rendu à la logique.
    handle: u64,
    id: Id,
    owner: Option<Obj>,
    pcm: Arc<Pcm>,
    /// Début de chaque boucle, fin, et position, en instants du son.
    start: f64,
    end: f64,
    pos: f64,
    /// Boucles restantes, celle-ci comprise.
    loops: u32,
    /// Volume gauche et droite selon la position (instants du son).
    envelope: Vec<(f64, f32, f32)>,
    gain: f32,
}

impl Voice {
    fn envelope_at(&self, pos: f64) -> (f32, f32) {
        let e = &self.envelope;
        match e.iter().position(|p| p.0 > pos) {
            None => e.last().map_or((1.0, 1.0), |p| (p.1, p.2)),
            Some(0) => (e[0].1, e[0].2),
            Some(i) => {
                let (a, b) = (e[i - 1], e[i]);
                let t = ((pos - a.0) / (b.0 - a.0)) as f32;
                (a.1 + (b.1 - a.1) * t, a.2 + (b.2 - a.2) * t)
            }
        }
    }
}

#[derive(Default)]
struct Mixer {
    voices: Vec<Voice>,
    /// Voix terminées d'elles-mêmes, pour la logique (onSoundComplete).
    finished: Vec<u64>,
    /// Fréquence et canaux de la sortie.
    rate: u32,
    channels: usize,
}

impl Mixer {
    /// Un bloc de sortie, entrelacé.
    fn render(&mut self, out: &mut [f32]) {
        out.fill(0.0);
        let (rate, channels) = (f64::from(self.rate), self.channels.max(1));
        let finished = &mut self.finished;
        self.voices.retain_mut(|v| {
            let step = f64::from(v.pcm.rate) / rate;
            let last = v.pcm.frames().saturating_sub(1);
            for frame in out.chunks_exact_mut(channels) {
                if v.pos >= v.end {
                    v.loops = v.loops.saturating_sub(1);
                    if v.loops == 0 || v.start >= v.end {
                        finished.push(v.handle);
                        return false;
                    }
                    v.pos = v.start + (v.pos - v.end);
                }
                let (l, r) = cubic(&v.pcm, v.pos, last);
                let (el, er) = if v.envelope.is_empty() { (1.0, 1.0) } else { v.envelope_at(v.pos) };
                let (l, r) = (l * el * v.gain, r * er * v.gain);
                if channels == 1 {
                    frame[0] += (l + r) / 2.0;
                } else {
                    frame[0] += l;
                    frame[1] += r;
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

/// Un instant entre deux échantillons, interpolé (Hermite cubique) pour
/// passer à la fréquence de la sortie sans grésillement.
fn cubic(pcm: &Pcm, pos: f64, last: usize) -> (f32, f32) {
    let i = pos as usize;
    let t = (pos - i as f64) as f32;
    let at = |k: isize| pcm.frame((i as isize + k).clamp(0, last as isize) as usize);
    let (p0, p1, p2, p3) = (at(-1), at(0), at(1), at(2));
    let h = |a: f32, b: f32, c: f32, d: f32| {
        let (c1, c2, c3) = ((c - a) / 2.0, a - 2.5 * b + 2.0 * c - d / 2.0, (d - a) / 2.0 + 1.5 * (b - c));
        ((c3 * t + c2) * t + c1) * t + b
    };
    (h(p0.0, p1.0, p2.0, p3.0), h(p0.1, p1.1, p2.1, p3.1))
}

pub struct Audio {
    mixer: Arc<Mutex<Mixer>>,
    /// None : pas de son (aucune sortie, ou tests).
    stream: Option<cpal::Stream>,
    pcm: HashMap<Id, Option<Arc<Pcm>>>,
    next_handle: u64,
}

impl Audio {
    /// La sortie par défaut (muet s'il n'y en a pas).
    pub fn new() -> Audio {
        let mut audio = Audio::silent();
        audio.stream = audio.open();
        audio
    }

    pub fn silent() -> Audio {
        Audio { mixer: Arc::default(), stream: None, pcm: HashMap::new(), next_handle: 0 }
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

    /// Lance un son : `offset` en secondes, joué `loops` fois ; `timeline`
    /// apporte le début, la fin et l'enveloppe d'un son de timeline.
    /// Rend l'identifiant de la voix (None si muet).
    pub fn start(&mut self, lib: &Library, id: Id, owner: Option<Obj>, offset: f64, loops: u32, timeline: Option<&StartSound>, gain: f32) -> Option<u64> {
        self.stream.as_ref()?;
        let pcm = self
            .pcm
            .entry(id)
            .or_insert_with(|| match lib.characters.get(&id) {
                Some(Character::Sound(s)) => decode::decode(s).map(Arc::new),
                _ => None,
            })
            .clone()?;
        let to_frames = f64::from(pcm.rate) / ENVELOPE_RATE;
        let mut start = offset.max(0.0) * f64::from(pcm.rate);
        let mut end = pcm.frames() as f64;
        let mut envelope = Vec::new();
        if let Some(t) = timeline {
            if let Some(i) = t.in_point {
                start = f64::from(i) * to_frames;
            }
            if let Some(o) = t.out_point {
                end = end.min(f64::from(o) * to_frames);
            }
            envelope = t.envelope.iter().map(|&(p, l, r)| (f64::from(p) * to_frames, l, r)).collect();
        }
        self.next_handle += 1;
        let handle = self.next_handle;
        lock(&self.mixer).voices.push(Voice { handle, id, owner, pcm, start, end, pos: start, loops: loops.max(1), envelope, gain });
        Some(handle)
    }

    /// Ce son joue-t-il ?
    pub fn playing(&self, id: Id) -> bool {
        lock(&self.mixer).voices.iter().any(|v| v.id == id)
    }

    /// Arrête les voix d'un son.
    pub fn stop_sound(&mut self, id: Id) {
        lock(&self.mixer).voices.retain(|v| v.id != id);
    }

    /// Arrête les voix d'un clip.
    pub fn stop_owner(&mut self, owner: Obj) {
        lock(&self.mixer).voices.retain(|v| v.owner != Some(owner));
    }

    pub fn stop_all(&mut self) {
        lock(&self.mixer).voices.clear();
    }

    /// Met à jour le volume de chaque voix selon son clip (None : clip
    /// disparu, le volume reste).
    pub fn set_gains(&mut self, gain: impl Fn(Option<Obj>) -> Option<f32>) {
        for v in &mut lock(&self.mixer).voices {
            if let Some(g) = gain(v.owner) {
                v.gain = g;
            }
        }
    }

    /// Les voix terminées d'elles-mêmes depuis le dernier appel.
    pub fn finished(&mut self) -> Vec<u64> {
        std::mem::take(&mut lock(&self.mixer).finished)
    }
}

/// Le mixeur, même si l'autre fil a paniqué en le tenant.
fn lock(mixer: &Mutex<Mixer>) -> MutexGuard<'_, Mixer> {
    mixer.lock().unwrap_or_else(|e| e.into_inner())
}
