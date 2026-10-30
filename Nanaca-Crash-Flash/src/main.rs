//! NANACA†CRASH!! Flash 1.10, portage PC non officiel.
//!
//! Le SWF original du joueur est lu au lancement (formes, images, polices,
//! textes, sons, timelines) ; la logique du jeu, réécrite en Rust d'après
//! le code d'origine, pilote ses animations comme le faisait Flash.

#![windows_subsystem = "windows"]

mod audio;
mod display;
mod draw;
mod game;
mod geom;
mod library;
mod original;
mod pad;
mod platform;
mod render;
mod swf;

use std::rc::Rc;

use miniquad::{CursorIcon, EventHandler, KeyCode, KeyMods, MouseButton, conf, window};

use audio::Audio;
use game::save::Save;
use game::{Dir, Game};
use library::Library;
use pad::{Buttons, Pads};
use render::Renderer;

/// Rattrapage maximal après un blocage, en secondes.
const MAX_CATCH_UP: f64 = 0.25;

struct App {
    renderer: Renderer,
    game: Game,
    pads: Pads,
    /// Les boutons des manettes à l'image précédente.
    pad: Buttons,
    lib: Rc<Library>,
    start: f64,
    last: f64,
    pending: f64,
    /// Tests : la partie simulée, une image par affichage.
    test: Option<Test>,
}

/// Tests (debug) : NANACA_SNAPSHOT=<images>:<fichier.png> joue ce nombre
/// d'images (dessinées une à une), enregistre la fenêtre en PNG et quitte ;
/// NANACA_INPUT=<image>:move:<x>,<y>;<image>:down;<image>:up… simule la
/// souris (coordonnées de la scène), <image>:nav:<direction>;<image>:a:down…
/// la manette ; NANACA_DUMP affiche l'arbre des clips.
struct Test {
    frame: u32,
    frames: u32,
    path: String,
    events: Vec<(u32, String, String)>,
}

impl App {
    fn new(lib: Rc<Library>, save: Save) -> App {
        // Les tests jouent sans son.
        let testing = cfg!(debug_assertions) && std::env::var_os("NANACA_SNAPSHOT").is_some();
        let audio = if testing { Audio::silent() } else { Audio::new() };
        let now = miniquad::date::now();
        let mut app = App { renderer: Renderer::new(), game: Game::new(lib.clone(), audio, save), pads: Pads::new(), pad: Buttons::default(), lib, start: now, last: now, pending: 0.0, test: None };
        app.game.run_scripts();
        app.test = Test::from_env();
        // La taille d'ouverture, que retrouve la fenêtre en quittant le
        // plein écran ; puis le mode d'affichage enregistré.
        platform::remember_window();
        if app.test.is_none() && app.game.saved_borderless() {
            app.game.set_borderless(true);
        }
        app
    }

    /// La scène dans la fenêtre : agrandie pour tenir, centrée (bandes
    /// noires). (x, y depuis le bas, largeur, hauteur) en pixels.
    fn viewport(&self) -> (i32, i32, i32, i32) {
        let (ww, wh) = window::screen_size();
        let (sw, sh) = self.lib.stage;
        let k = (ww / sw).min(wh / sh);
        let (w, h) = (sw * k, sh * k);
        (((ww - w) / 2.0) as i32, ((wh - h) / 2.0) as i32, w as i32, h as i32)
    }

    /// Un point de la fenêtre sur la scène.
    fn to_stage(&self, x: f32, y: f32) -> (f32, f32) {
        let (vx, vy, vw, vh) = self.viewport();
        let (_, wh) = window::screen_size();
        let k = vw as f32 / self.lib.stage.0;
        // y du viewport est compté depuis le bas.
        let top = wh - (vy + vh) as f32;
        ((x - vx as f32) / k, (y - top) / k)
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

    /// Tests : une image simulée, avec ses événements de souris.
    fn test_step(&mut self) {
        let Some(test) = &mut self.test else { return };
        let frame = test.frame;
        test.frame += 1;
        let events: Vec<(String, String)> = test.events.iter().filter(|e| e.0 == frame).map(|e| (e.1.clone(), e.2.clone())).collect();
        self.game.tick(f64::from(frame) * 1000.0 / f64::from(self.lib.frame_rate));
        for (kind, arg) in events {
            match kind.as_str() {
                "move" => {
                    let (x, y) = arg.split_once(',').unwrap_or(("0", "0"));
                    self.game.mouse_move(x.parse().unwrap_or(0.0), y.parse().unwrap_or(0.0));
                }
                "down" => self.game.mouse_down(),
                "up" => self.game.mouse_up(),
                // La manette : nav:up|down|left|right, a:down|up, b, start.
                "nav" => {
                    let dir = match arg.as_str() {
                        "up" => Dir::Up,
                        "down" => Dir::Down,
                        "left" => Dir::Left,
                        _ => Dir::Right,
                    };
                    self.game.pad_nav(dir);
                }
                "a" => self.game.pad_confirm(arg == "down"),
                "b" => self.game.pad_back(),
                "start" => self.game.pad_pause(),
                _ => {}
            }
        }
        self.game.step();
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
                Some((p.next()?.parse().ok()?, p.next()?.to_string(), p.next().unwrap_or("").to_string()))
            })
            .collect();
        Some(Test { frame: 0, frames: frames.parse().unwrap_or(1), path: path.to_string(), events })
    }
}

impl EventHandler for App {
    fn update(&mut self) {
        if self.test.is_some() {
            self.test_step();
            return;
        }
        let now = miniquad::date::now();
        self.pending = (self.pending + now - self.last).min(MAX_CATCH_UP);
        self.last = now;
        self.poll_pads();
        self.game.tick((now - self.start) * 1000.0);
        let step = 1.0 / f64::from(self.lib.frame_rate);
        while self.pending >= step {
            self.pending -= step;
            self.game.step();
        }
    }

    fn draw(&mut self) {
        let viewport = self.viewport();
        self.renderer.smooth = self.game.quality == "HIGH";
        self.renderer.begin(self.lib.stage, self.lib.background, viewport);
        draw::draw(&mut self.renderer, &self.game.d);
        if let Some(test) = &self.test
            && test.frame >= test.frames
        {
            if std::env::var_os("NANACA_DUMP").is_some() {
                eprint!("{}", self.game.d.dump());
            }
            let (w, h) = window::screen_size();
            let pixels = self.renderer.read_pixels(w as i32, h as i32);
            let _ = std::fs::write(&test.path, png(w as u32, h as u32, &pixels));
            std::process::exit(0);
        }
        self.renderer.end();
    }

    fn mouse_motion_event(&mut self, x: f32, y: f32) {
        let (sx, sy) = self.to_stage(x, y);
        self.game.mouse_move(sx, sy);
        window::set_mouse_cursor(if self.game.hand_cursor() { CursorIcon::Pointer } else { CursorIcon::Default });
    }

    /// Alt+Entrée ou F11 : fenêtre ou plein écran sans bordure.
    fn key_down_event(&mut self, key: KeyCode, mods: KeyMods, repeat: bool) {
        if !repeat && (key == KeyCode::Enter && mods.alt || key == KeyCode::F11) {
            let on = !self.game.borderless();
            self.game.set_borderless(on);
        }
    }

    fn mouse_button_down_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.mouse_motion_event(x, y);
            self.game.mouse_down();
        }
    }

    fn mouse_button_up_event(&mut self, button: MouseButton, x: f32, y: f32) {
        if button == MouseButton::Left {
            self.mouse_motion_event(x, y);
            self.game.mouse_up();
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
    let mut save = Save::load();
    let (path, checked) = save.assets();
    let lib = match original::load(path, checked) {
        Ok((data, stamp)) => {
            save.set_assets_checked(&stamp);
            swf::load(&data)
        }
        Err(message) => Err(format!("{message}\n\nIts path can also be set in \"assets\" > \"path\" of {}.", save.file())),
    };
    let lib = match lib {
        Ok(lib) => Rc::new(lib),
        Err(message) => {
            message_box(&message);
            return;
        }
    };
    let (w, h) = lib.stage;
    let conf = conf::Conf {
        window_title: "NANACA†CRASH!!".to_string(),
        window_width: w as i32,
        window_height: h as i32,
        window_resizable: true,
        high_dpi: true,
        sample_count: 4,
        ..Default::default()
    };
    miniquad::start(conf, move || Box::new(App::new(lib, save)));
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
