//! NANACA†CRASH!! Android 1.2.0, portage PC non officiel.
//!
//! L'APK original du joueur est lu au lancement (images, polices,
//! armatures, sons, textes) ; la logique du jeu, réécrite en Rust d'après le
//! code d'origine (C++ et Lua), pilote un graphe de scène qui se comporte
//! comme celui de Cocos2d-x 2.x.

#![windows_subsystem = "windows"]

mod action;
mod armature;
mod assets;
mod audio;
mod data;
mod draw;
mod game;
mod geom;
mod node;
mod original;
mod pad;
mod platform;
mod render;
mod text;

use std::rc::Rc;

use miniquad::{EventHandler, KeyCode, KeyMods, MouseButton, conf, window};

use assets::Assets;
use audio::Audio;
use draw::{Drawer, Textures};
use game::save::Save;
use game::{DESIGN_HEIGHT, Dir, FRAME, Game, Transition};
use pad::{Buttons, Pads};
use render::Renderer;

/// Proportions de la scène : 16:9, comme un téléphone de l'époque.
const ASPECT: f32 = 16.0 / 9.0;
/// Rattrapage maximal après un blocage, en secondes.
const MAX_CATCH_UP: f64 = 0.25;

/// Le mode d'affichage dans la sauvegarde (réglages) : clé et valeurs,
/// comme la version Flash.
const DISPLAY_SETTING: &str = "displaysetting";
const WINDOW: &str = "window";
const BORDERLESS: &str = "borderless";

struct App {
    renderer: Renderer,
    textures: Textures,
    game: Game,
    pads: Pads,
    pad: Buttons,
    last: f64,
    pending: f64,
    borderless: bool,
    mouse_down: bool,
    test: Option<Test>,
    /// Les captures des scènes d'une transition (sortante, entrante).
    out_snapshot: Option<render::Texture>,
    in_snapshot: Option<render::Texture>,
}

/// Tests (debug) : NANACA_SNAPSHOT=<images>:<fichier.png> joue ce nombre
/// d'images (dessinées une à une), enregistre la fenêtre en PNG et quitte ;
/// NANACA_INPUT=<image>:down:<x>,<y>;<image>:up:<x>,<y> simule le toucher
/// (points de la scène, y vers le haut), et <image>:nav_up|nav_down|
/// nav_left|nav_right|a_down|a_up|b|start:0,0 la manette ;
/// NANACA_EVERY=<n> enregistre aussi une image sur n
/// (<fichier>_<image>.png). Sans son ; la sauvegarde est lue, jamais écrite.
struct Test {
    frame: u32,
    frames: u32,
    every: u32,
    path: String,
    events: Vec<(u32, String, (f32, f32))>,
}

impl App {
    fn new(assets: Rc<Assets>, save: Save) -> App {
        let test = Test::from_env();
        let audio = if test.is_some() { Audio::silent() } else { Audio::new() };
        let game = Game::new(assets, audio, save, (DESIGN_HEIGHT * ASPECT, DESIGN_HEIGHT));
        // La taille d'ouverture, que retrouve la fenêtre en quittant le
        // plein écran ; puis le mode d'affichage enregistré.
        platform::remember_window();
        let borderless = test.is_none() && game.save.string(DISPLAY_SETTING, WINDOW) == BORDERLESS;
        if borderless {
            platform::set_borderless(true);
        }
        App { renderer: Renderer::new(), textures: Textures::default(), game, pads: Pads::new(), pad: Buttons::default(), last: miniquad::date::now(), pending: 0.0, borderless, mouse_down: false, test, out_snapshot: None, in_snapshot: None }
    }

    /// La scène dans la fenêtre : agrandie pour tenir, centrée (bandes
    /// noires). (x, y depuis le bas, largeur, hauteur) en pixels.
    fn viewport(&self) -> (i32, i32, i32, i32) {
        let (ww, wh) = window::screen_size();
        let (sw, sh) = self.game.design;
        let k = (ww / sw).min(wh / sh);
        let (w, h) = (sw * k, sh * k);
        (((ww - w) / 2.0) as i32, ((wh - h) / 2.0) as i32, w as i32, h as i32)
    }

    /// Manettes : un bouton qui vient d'être appuyé devient une commande.
    fn poll_pads(&mut self) {
        let pad = self.pads.poll();
        let old = self.pad;
        self.pad = pad;
        if pad.start && pad.select {
            if !(old.start && old.select) {
                window::order_quit();
            }
            return;
        }
        let pressed = |now: bool, before: bool| now && !before;
        for (now, before, dir) in [(pad.up, old.up, Dir::Up), (pad.down, old.down, Dir::Down), (pad.left, old.left, Dir::Left), (pad.right, old.right, Dir::Right)] {
            if pressed(now, before) {
                self.game.pad_nav(dir);
            }
        }
        if pressed(pad.a, old.a) {
            self.game.pad_confirm(true);
        }
        if !pad.a && old.a {
            self.game.pad_confirm(false);
        }
        if pressed(pad.b, old.b) {
            self.game.pad_back();
        }
        if !pad.start && old.start && !old.select {
            // Start seul, au relâché (Start + Select quitte).
            self.game.pad_pause();
        }
    }
}

impl Test {
    fn from_env() -> Option<Test> {
        if !cfg!(debug_assertions) {
            return None;
        }
        let spec = std::env::var("NANACA_SNAPSHOT").ok()?;
        let (frames, path) = spec.split_once(':')?;
        let input = std::env::var("NANACA_INPUT").unwrap_or_default();
        let events = input
            .split(';')
            .filter_map(|e| {
                let mut p = e.splitn(3, ':');
                let frame = p.next()?.parse().ok()?;
                let kind = p.next()?.to_string();
                let (x, y) = p.next()?.split_once(',')?;
                Some((frame, kind, (x.parse().ok()?, y.parse().ok()?)))
            })
            .collect();
        let every = std::env::var("NANACA_EVERY").ok().and_then(|n| n.parse().ok()).unwrap_or(0);
        Some(Test { frame: 0, frames: frames.parse().unwrap_or(1), every, path: path.to_string(), events })
    }
}

impl App {
    fn draw_scene(&mut self) {
        if let Some(root) = self.game.scene {
            let assets = self.game.assets.clone();
            Drawer { graph: &self.game.g, assets: &assets, renderer: &mut self.renderer, textures: &mut self.textures }.scene(root);
        }
    }

    fn drop_snapshots(&mut self) {
        for t in [self.out_snapshot.take(), self.in_snapshot.take()].into_iter().flatten() {
            self.renderer.delete(t);
        }
    }

    /// Une image de la scène capturée, sur la partie (fractions x0, y0,
    /// x1, y1) de l'écran.
    fn draw_snapshot(&mut self, t: render::Texture, part: [f32; 4], alpha: f32) {
        let (w, h) = self.game.design;
        let [x0, y0, x1, y1] = part;
        let corners = [(x0 * w, y0 * h), (x1 * w, y0 * h), (x0 * w, y1 * h), (x1 * w, y1 * h)];
        let uv = [(x0, y0), (x1, y0), (x0, y1), (x1, y1)];
        self.renderer.quad(Some(t.id), corners, uv, [alpha; 4], render::Blend::Normal);
    }

    /// La transition en cours, par-dessus la nouvelle scène.
    fn draw_transition(&mut self) {
        let Some(t) = self.game.transition else {
            self.drop_snapshots();
            return;
        };
        let Some(out) = self.out_snapshot else { return };
        match t.kind {
            Transition::CrossFade(duration) => {
                // CCTransitionCrossFade : la nouvelle scène figée à sa première
                // image, l'ancienne qui s'efface dessus.
                if self.in_snapshot.is_none() {
                    self.in_snapshot = Some(self.renderer.snapshot());
                }
                self.renderer.clear();
                if let Some(t_in) = self.in_snapshot {
                    self.draw_snapshot(t_in, [0.0, 0.0, 1.0, 1.0], 1.0);
                }
                self.draw_snapshot(out, [0.0, 0.0, 1.0, 1.0], (1.0 - t.elapsed / duration).clamp(0.0, 1.0));
            }
            Transition::Progress { duration, midpoint, rate } => {
                // CCProgressTimer en barre, de 100 % à 0 % (updateBar).
                let alpha = (1.0 - t.elapsed / duration).clamp(0.0, 1.0);
                let offset = |r: f32| (1.0 - r + alpha * r) * 0.5;
                let (mut min, mut max) = ((midpoint.0 - offset(rate.0), midpoint.1 - offset(rate.1)), (midpoint.0 + offset(rate.0), midpoint.1 + offset(rate.1)));
                if min.0 < 0.0 {
                    max.0 -= min.0;
                    min.0 = 0.0;
                }
                if max.0 > 1.0 {
                    min.0 -= max.0 - 1.0;
                    max.0 = 1.0;
                }
                if min.1 < 0.0 {
                    max.1 -= min.1;
                    min.1 = 0.0;
                }
                if max.1 > 1.0 {
                    min.1 -= max.1 - 1.0;
                    max.1 = 1.0;
                }
                self.draw_snapshot(out, [min.0, min.1, max.0, max.1], 1.0);
            }
        }
    }

    /// Un point de la fenêtre dans la scène (points, y vers le haut).
    fn to_scene(&self, x: f32, y: f32) -> (f32, f32) {
        let (vx, vy, vw, _) = self.viewport();
        let (_, wh) = window::screen_size();
        let k = vw as f32 / self.game.design.0;
        ((x - vx as f32) / k, (wh - y - vy as f32) / k)
    }
}

impl EventHandler for App {
    fn update(&mut self) {
        if let Some(test) = &mut self.test {
            let frame = test.frame;
            test.frame += 1;
            let events: Vec<_> = test.events.iter().filter(|e| e.0 == frame).map(|e| (e.1.clone(), e.2)).collect();
            for (kind, p) in events {
                match kind.as_str() {
                    "down" => self.game.touch_began(p),
                    "move" => self.game.touch_moved(p),
                    "nav_up" => self.game.pad_nav(Dir::Up),
                    "nav_down" => self.game.pad_nav(Dir::Down),
                    "nav_left" => self.game.pad_nav(Dir::Left),
                    "nav_right" => self.game.pad_nav(Dir::Right),
                    "a_down" => self.game.pad_confirm(true),
                    "a_up" => self.game.pad_confirm(false),
                    "b" => self.game.pad_back(),
                    "start" => self.game.pad_pause(),
                    _ => self.game.touch_ended(p),
                }
            }
            self.game.step(FRAME);
            return;
        }
        let now = miniquad::date::now();
        self.pending = (self.pending + now - self.last).min(MAX_CATCH_UP);
        self.last = now;
        self.poll_pads();
        while self.pending >= f64::from(FRAME) {
            self.pending -= f64::from(FRAME);
            self.game.step(FRAME);
        }
    }

    fn draw(&mut self) {
        let viewport = self.viewport();
        self.renderer.begin(self.game.design, viewport);
        // Une transition commence : l'ancienne scène est capturée, la
        // nouvelle la remplace.
        if self.game.awaiting_capture.is_some() {
            self.draw_scene();
            self.drop_snapshots();
            self.out_snapshot = Some(self.renderer.snapshot());
            self.renderer.clear();
            self.game.start_transition();
        }
        self.draw_scene();
        self.draw_transition();
        if let Some(test) = &self.test {
            let (w, h) = window::screen_size();
            if test.every > 0 && test.frame % test.every == 0 && test.frame < test.frames {
                let pixels = self.renderer.read_pixels(w as i32, h as i32);
                let path = test.path.strip_suffix(".png").unwrap_or(&test.path);
                let _ = std::fs::write(format!("{path}_{}.png", test.frame), png(w as u32, h as u32, &pixels));
            }
            if test.frame >= test.frames {
                let pixels = self.renderer.read_pixels(w as i32, h as i32);
                let _ = std::fs::write(&test.path, png(w as u32, h as u32, &pixels));
                std::process::exit(0);
            }
        }
        self.renderer.end();
    }

    /// La souris tient lieu de doigt : bouton gauche appuyé, déplacé,
    /// relâché.
    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.mouse_down = true;
            self.game.clear_focus();
            let p = self.to_scene(x, y);
            self.game.touch_began(p);
        }
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        if self.mouse_down {
            let p = self.to_scene(x, y);
            self.game.touch_moved(p);
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left && self.mouse_down {
            self.mouse_down = false;
            let p = self.to_scene(x, y);
            self.game.touch_ended(p);
        }
    }

    /// Alt+Entrée ou F11 : fenêtre ou plein écran sans bordure. Le clavier
    /// fait aussi la manette : flèches, Entrée ou Espace (A), Échap (B),
    /// P (Start).
    fn key_down_event(&mut self, key: KeyCode, mods: KeyMods, repeat: bool) {
        if repeat {
            return;
        }
        if key == KeyCode::Enter && mods.alt || key == KeyCode::F11 {
            self.borderless = !self.borderless;
            platform::set_borderless(self.borderless);
            self.game.save.set_string(DISPLAY_SETTING, if self.borderless { BORDERLESS } else { WINDOW });
            return;
        }
        match key {
            KeyCode::Up => self.game.pad_nav(Dir::Up),
            KeyCode::Down => self.game.pad_nav(Dir::Down),
            KeyCode::Left => self.game.pad_nav(Dir::Left),
            KeyCode::Right => self.game.pad_nav(Dir::Right),
            KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space => self.game.pad_confirm(true),
            KeyCode::Escape => self.game.pad_back(),
            KeyCode::P => self.game.pad_pause(),
            _ => {}
        }
    }

    fn key_up_event(&mut self, key: KeyCode, _mods: KeyMods) {
        if matches!(key, KeyCode::Enter | KeyCode::KpEnter | KeyCode::Space) {
            self.game.pad_confirm(false);
        }
    }
}

/// Un PNG RGBA (captures de test).
fn png(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    let mut raw = Vec::with_capacity(rgba.len() + height as usize);
    for line in rgba.chunks_exact(width as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(line);
    }
    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut chunk = |kind: &[u8], data: &[u8]| {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let start = out.len();
        out.extend_from_slice(kind);
        out.extend_from_slice(data);
        let crc = original::crc32(&out[start..]);
        out.extend_from_slice(&crc.to_be_bytes());
    };
    let mut header = Vec::new();
    header.extend_from_slice(&width.to_be_bytes());
    header.extend_from_slice(&height.to_be_bytes());
    header.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(b"IHDR", &header);
    chunk(b"IDAT", &miniz_oxide::deflate::compress_to_vec_zlib(&raw, 6));
    chunk(b"IEND", &[]);
    out
}

fn main() {
    let mut save = if Test::from_env().is_some() { Save::memory() } else { Save::load() };
    let (path, checked) = save.assets();
    let assets = match original::Apk::load(path, checked) {
        Ok((apk, stamp)) => {
            save.set_assets_checked(&stamp);
            Rc::new(Assets::new(apk))
        }
        Err(message) => {
            message_box(&format!("{message}\n\nIts path can also be set in \"assets\" > \"path\" of {}.", save.file()));
            return;
        }
    };
    let conf = conf::Conf {
        window_title: "NANACA†CRASH!!".to_string(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    };
    miniquad::start(conf, move || Box::new(App::new(assets, save)));
}

/// Une boîte de message Windows (le jeu ne peut pas démarrer).
fn message_box(text: &str) {
    #[cfg(windows)]
    {
        #[link(name = "user32")]
        unsafe extern "system" {
            fn MessageBoxW(wnd: *mut core::ffi::c_void, text: *const u16, caption: *const u16, kind: u32) -> i32;
        }
        let wide = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
        // SAFETY : les deux chaînes finissent par 0 et vivent pendant l'appel.
        unsafe { MessageBoxW(std::ptr::null_mut(), wide(text).as_ptr(), wide("NANACA†CRASH!!").as_ptr(), 0x10) };
    }
    #[cfg(not(windows))]
    eprintln!("{text}");
}
