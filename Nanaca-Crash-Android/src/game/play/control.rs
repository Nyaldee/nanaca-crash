//! Le vol de Taichi (StateControlDelegate) : vitesses, pas de temps
//! limité, gravité, rebonds, relances, champs de force (C.F.F.), jauge
//! aérienne, ralentis, et ce qui suit Taichi (décors, personnages,
//! compteurs).

use std::collections::VecDeque;

use super::{Play, State, cast, hud, taichi};
use crate::game::Game;

/// Le vol en cours (champs de StateControlDelegate).
#[derive(Default)]
pub struct Control {
    pub vx: f32,
    pub vy: f32,
    pub px: f32,
    pub py: f32,
    /// Rotation de Taichi (degrés).
    pub r: f32,
    /// Puissance et angle de la jauge.
    pub power: f32,
    pub angle: f32,
    /// Ralenti : images restantes et pas de temps.
    pub slowcount: f32,
    pub slowinterval: f32,
    /// Position du prochain personnage.
    pub charapos: f32,
    /// Images depuis le dernier affichage du score, et des feux.
    pub score_frames: u32,
    pub firework_frames: u32,
    /// Pas sans limite (C.F.F. C), contacts possibles, sans gravité.
    pub nolimit: bool,
    pub hit_enabled: bool,
    pub no_gravity: bool,
    /// Spéciaux disponibles (setSpecial) : Youko bloque, Touko, Misato,
    /// Kiri, Miki, combo, Nanaka.
    pub youko_block: bool,
    pub touko: bool,
    pub misato: bool,
    pub kiri: bool,
    pub miki: bool,
    pub combo: bool,
    pub nanaca: bool,
    /// Record battu (le score passe en rouge une fois).
    pub record_beaten: bool,
    /// Chrono du spécial : -1 aucun, 0 raté, 1 réussi.
    pub special_state: i32,
    /// Attaque aérienne possible maintenant, et permise.
    pub aerial_available: bool,
    pub aerial_allowed: bool,
    /// Mode aérien affiché (0 haut, 1, 2 plongée, 3) ; -1 au départ.
    pub aerial_mode: i32,
    /// Jauge aérienne (0..100) et coups vers le haut restants.
    pub aerial_gauge: f32,
    pub ups: i32,
    /// Vitesse gardée pendant le C.F.F. C.
    pub saved: (f32, f32),
    /// Intervalle d'une image (1/60).
    pub interval: f32,
    /// Chrono du spécial (secondes).
    pub special_timer: f32,
    /// Compteur de vitesse masqué.
    pub speed_blind: bool,
    /// Champ de force en cours (0 A, 1 B, 2 C, 3 D, 99 aucun) et sa réserve.
    pub cff: i32,
    pub cff_count: f32,
    /// Démonstration (galerie) : pas de succès.
    pub demo: bool,
    pub stop: bool,
    /// Nanaka pas encore dépassée.
    pub nanaca_first: bool,
    /// Succès : spéciaux vus (bits), champs de force de Youko vus (bits),
    /// blocages de Youko, chutes de Tomoki et Sakuraba.
    pub special_bits: u32,
    pub block_bits: u32,
    pub block_count: i32,
    pub fallen: i32,
    /// Le champ de force que lancera la contre-attaque de Youko.
    pub cff_next: i32,
    /// Relances successives.
    pub boosts: i32,
    /// Le personnage courant et la pile des suivants.
    pub current: Option<usize>,
    pub stack: VecDeque<usize>,
    /// Le mode du compteur « %3.0f: » déjà affiché.
    pub shown_gauge: Option<f32>,
}

/// Couleurs des champs de force (RVB) : A, B, C, D.
const CFF_COLORS: [[u8; 3]; 4] = [[0x80, 0xFF, 0x80], [0xFF, 0x80, 0x80], [0x80, 0x80, 0xFF], [0xFF, 0xFF, 0x80]];

/// StatePlay::enter : initializeGameParameters, puis la musique du vol.
pub fn start(game: &mut Game) {
    initialize(game);
    let c = &mut super::play(game).c;
    c.demo = false;
    if c.power.round() as i32 == 100 && c.angle.round() as i32 == 45 {
        game.unlock(7);
    }
    game.play_music("bgmogg/bgm1.ogg", true, 24.049);
}

/// initializeGameParameters : Taichi visible, l'élan de la jauge, tout
/// remis à zéro, la pile des personnages mélangée.
pub fn initialize(game: &mut Game) {
    let play = super::play(game);
    let (jm, taichi_id, game_layer, bg) = (play.jump_meter, play.taichi.node, play.game_layer, play.bg);
    let (angle, power) = (play.meter_angle, play.meter_power);
    let casts: Vec<usize> = (0..play.casts.len()).collect();
    let _ = jm;
    game.set_visible(taichi_id, true);
    let c = &mut super::play(game).c;
    // Le drapeau de démonstration (StatePlayDemo::enter) reste.
    *c = Control { power, angle, interval: 0.016_666_668, hit_enabled: true, demo: c.demo, ..Control::default() };
    let a = c.angle * 0.017_453_28;
    let v = f64::from(c.power) * 0.3;
    c.vx = (f64::from(a.cos()) * v) as f32;
    c.vy = (f64::from(a.sin()) * v) as f32;
    c.py = 0.75;
    c.cff = 99;
    c.aerial_allowed = true;
    c.nanaca_first = true;
    c.aerial_mode = -1;
    c.aerial_gauge = 100.0;
    c.ups = 3;
    c.special_state = -1;
    update_aerial_state(game, false, false, 0.0);
    for id in [game_layer, bg] {
        game.set_anchor(id, 0.3, 0.0);
    }
    // La pile des personnages : chacun inséré à une place au hasard.
    for k in casts {
        let n = super::play(game).c.stack.len();
        let pos = if n == 0 { 0 } else { (n as f32 * 4.656_613e-10 * game.rng.lrand48() as f32).round() as usize };
        let stack = &mut super::play(game).c.stack;
        stack.insert(pos.min(stack.len()), k);
    }
}

/// boost : une relance (personnage, Nanaka).
pub fn boost(game: &mut Game, power: f32, angle: f32) {
    let p = boost_apply_cff(game, power);
    let c = &mut super::play(game).c;
    let a = angle * 0.017_453_28;
    c.vx += a.cos() * p;
    c.boosts += 1;
    c.vy = c.vy.abs() + a.sin() * p;
    check_boost_combo(game);
}

/// aerial_boost : le coup de l'attaque aérienne.
pub fn aerial_boost(game: &mut Game) {
    let c = &mut super::play(game).c;
    let (x, y) = (c.vx, c.vy);
    let a = (f64::from(y.atan2(x)) * 0.9) as f32;
    c.vx = x + a.cos() * 10.0;
    c.boosts += 1;
    c.vy = -(y + a.sin() * 10.0);
    check_boost_combo(game);
}

fn check_boost_combo(game: &mut Game) {
    let c = &super::play(game).c;
    if c.demo {
        return;
    }
    match c.boosts {
        4 => game.unlock(0x1a),
        6 => game.unlock(0x1b),
        10 => game.unlock(0x1c),
        _ => {}
    }
}

/// boost_apply_cff : les champs A (puissance doublée) et B (charge).
fn boost_apply_cff(game: &mut Game, power: f32) -> f32 {
    let c = &mut super::play(game).c;
    let mut p = power;
    if c.cff == 0 {
        c.cff_count -= 10.0;
        if c.cff_count <= 0.0 {
            c.cff_count = 0.0;
            cff_clear(game);
        }
        p += p;
        slow_with_interval(game, 30.0, 1.0);
        taichi::cff_a(game);
    } else if c.cff == 1 && c.cff_count < 60.0 {
        c.cff_count = (f64::from(c.cff_count) + f64::from(p) * 0.2).min(60.0) as f32;
        taichi::cff_b_charge(game);
        p *= 0.9;
    }
    p
}

pub fn slow_with_interval(game: &mut Game, count: f32, interval: f32) {
    let c = &mut super::play(game).c;
    c.slowcount = count;
    c.slowinterval = interval;
}

pub fn slow_with_distance(game: &mut Game, count: f32, distance: f32) {
    let c = &mut super::play(game).c;
    c.slowcount = count;
    let s = distance / ((c.vx * c.vx + c.vy * c.vy).sqrt() * count);
    c.slowinterval = s.min(c.interval);
}

/// cff_clear : plus de champ de force, Taichi reprend sa couleur.
pub fn cff_clear(game: &mut Game) {
    let t = super::play(game).taichi.node;
    super::play(game).c.cff = 99;
    game.run(t, crate::action::Action::tint_to(1.0, [0xFF; 3]));
}

/// setComicForceField : un champ de force (type 0..3).
pub fn set_comic_force_field(game: &mut Game, kind: i32) {
    let c = &mut super::play(game).c;
    let count = c.cff_count;
    let (active, changed) = if count <= 0.0 { (false, false) } else { (true, c.cff != kind) };
    c.cff = kind;
    if kind == 2 {
        c.saved = (c.vx, c.vy);
        c.no_gravity = true;
        c.vx = 10.0;
        c.vy = 20.0;
        c.cff_count = count + 20.0;
        set_speed_blind(game, true);
    } else {
        c.cff_count = count + 30.0;
    }
    let color = CFF_COLORS[kind.clamp(0, 3) as usize];
    taichi::cff_start(game, color, active, changed);
}

pub fn set_speed_blind(game: &mut Game, on: bool) {
    super::play(game).c.speed_blind = on;
    if on {
        let speed = super::play(game).hud.speed_label;
        game.g.set_text(speed, "---.--:/;");
    }
}

/// C.F.F. B : relâché quand Taichi va retomber près du sol.
fn cff_process_b(game: &mut Game) {
    let c = &super::play(game).c;
    if c.vx >= 2.0 || c.vy > 0.0 || c.py >= 5.0 || -(c.vy * c.vy * -0.102_040_81) < c.py || !c.hit_enabled || c.slowcount as i32 != 0 || c.no_gravity {
        return;
    }
    let count = c.cff_count;
    boost(game, count, 45.0);
    let c = &mut super::play(game).c;
    c.cff_count = 0.0;
    taichi::cff_b_release(game);
    cff_clear(game);
    slow_with_distance(game, 60.0, 1.0);
    let c = &mut super::play(game).c;
    c.hit_enabled = true;
    c.py += 0.2;
}

/// C.F.F. C : vol rasant jusqu'à épuisement de la réserve.
fn cff_process_c(game: &mut Game, dt: f32) {
    let c = &mut super::play(game).c;
    if f64::from(c.py) > 2.3 {
        c.slowcount = 0.0;
        c.nolimit = true;
        c.vy = 0.0;
        c.py = 2.3;
        c.vx = 55.0;
        taichi::cff_c(game);
        super::play(game).c.shown_gauge = Some(0.0);
        return;
    }
    c.cff_count -= c.vx * 0.060_000_002 * dt;
    let shown = c.cff_count.round();
    if c.shown_gauge != Some(shown) {
        c.shown_gauge = Some(shown);
        let label = super::play(game).taichi.counter;
        game.g.set_text(label, &format!("{:03}", shown as i32));
    }
    let c = &mut super::play(game).c;
    if c.cff_count <= 0.0 {
        c.vx = c.saved.0;
        c.cff_count = 0.0;
        c.vy = c.saved.1;
        c.no_gravity = false;
        c.nolimit = false;
        cff_clear(game);
        taichi::cff_c_stop(game);
        set_speed_blind(game, false);
    }
}

/// C.F.F. D : au sol, Taichi rebondit plus haut.
fn cff_process_d(game: &mut Game) {
    let c = &mut super::play(game).c;
    c.cff_count -= 6.0;
    if c.cff_count <= 0.0 {
        c.cff_count = 0.0;
        cff_clear(game);
    }
    game.play_effect("seogg/se19.ogg");
    taichi::cff_d(game);
    slow_with_interval(game, 45.0, 0.0);
    let c = &mut super::play(game).c;
    let v = (c.vy * c.vy - c.py * -19.6).max(0.0);
    c.vy = v.sqrt() * 1.1;
    c.vx *= 1.1;
}

/// update_aerial_state : la jauge aérienne et ce qui est possible ;
/// rend le mode.
pub fn update_aerial_state(game: &mut Game, charging: bool, falling: bool, dt: f32) -> i32 {
    let gauge_label = super::play(game).hud.aerial_label;
    let daggers = super::play(game).hud.daggers;
    let font_color = super::play(game).font_color;
    let c = &mut super::play(game).c;
    let before = c.aerial_gauge.trunc();
    let mut mode = if falling { 1 } else { 3 };
    let mut available = false;
    if charging {
        c.aerial_gauge = (c.aerial_gauge + dt * 33.333_336).min(100.0);
        let shown = c.aerial_gauge.trunc();
        let (allowed, gauge, ups) = (c.aerial_allowed, c.aerial_gauge, c.ups);
        if before != shown {
            game.g.set_text(gauge_label, &format!("{shown:3.0}:"));
            if shown >= 100.0 {
                game.set_color(gauge_label, font_color);
                if let Some(d) = game.child(daggers, 3) {
                    game.set_color(d, [0x66, 0x66, 0xFF]);
                }
            } else {
                game.set_color(gauge_label, [0x66; 3]);
            }
        }
        if allowed {
            if !falling {
                if gauge == 100.0 {
                    mode = 2;
                    available = true;
                }
            } else if ups > 0 {
                mode = 0;
                available = true;
            }
        }
    }
    super::play(game).c.aerial_available = available;
    mode
}

/// execute_impl : une image de vol.
pub fn execute(game: &mut Game, dt: f32) {
    super::play(game).elapsed += dt;
    match super::play(game).c.cff {
        1 => cff_process_b(game),
        2 => cff_process_c(game, dt),
        _ => {}
    }
    let c = &mut super::play(game).c;
    let mut py = c.py;
    let mut vy = c.vy;
    if py == 0.0 && f64::from(c.vx) < 1e-4 && f64::from(vy) < 1e-4 {
        c.vx = 0.0;
        c.vy = 0.0;
        if c.demo {
            return;
        }
        let (px, angle, power) = (c.px, c.angle, c.power);
        super::change_state(game, State::GameOver { distance: px, angle, power });
        return;
    }
    let vx = c.vx.min(128.0);
    c.vx = vx;
    let step;
    if c.slowcount <= 0.0 {
        if c.nolimit {
            step = dt;
        } else {
            let denom = vx.max(vy.abs() - dt * -9.8);
            let limit = if -8.2 / py <= -1.8 { 1.28 } else { (-8.2 / py + 2.8) * 1.28 };
            step = if limit / denom < dt { limit / denom } else { dt };
        }
    } else {
        let si = c.slowinterval;
        let denom = vx.max(vy.abs() - si * -9.8);
        let s = (0.08 / denom).min(si);
        c.slowcount -= dt / c.interval;
        step = s * (dt / c.interval);
    }
    if c.no_gravity {
        py += step * vy;
    }
    c.px += step * vx;
    let mut zoom_from_scale = false;
    if c.no_gravity {
        c.py = py;
    } else {
        py += (vy + step * -9.8 * 0.5) * step;
        vy += step * -9.8;
        c.py = py;
        c.vy = vy;
        if py < 0.0 {
            c.boosts = 0;
            if c.cff == 3 {
                cff_process_d(game);
            } else {
                let c = &mut super::play(game).c;
                let v = (vy * vy - py * -19.6).max(0.0);
                c.vx = vx * 0.8;
                c.vy = v.sqrt() * 0.8;
                let (bx, by) = (c.vx, c.vy);
                taichi::bounce(game, bx, by);
            }
            let c = &mut super::play(game).c;
            c.py = 0.0;
            c.misato = false;
            zoom_from_scale = true;
        }
    }
    // Le zoom : de loin en hauteur, sinon retour progressif.
    let (game_layer, bg) = (super::play(game).game_layer, super::play(game).bg);
    let py = super::play(game).c.py;
    let zoom = if !zoom_from_scale && py >= 2.5 {
        (1.0 / (f64::from(py) * 0.4) as f32).max(0.25)
    } else {
        let s = game.scale(game_layer);
        if s > 1.0 { s - 0.01 } else { 1.0 }
    };
    game.set_scale(game_layer, zoom);
    game.set_scale(bg, zoom);
    let c = &mut super::play(game).c;
    let turn = (c.vx * 30.0 * step).min(30.0);
    c.r = clamp_angle(c.r + turn);
    // Le chrono du spécial, et l'arrivée des personnages.
    let mut next_cast = c.px + 20.0 >= c.charapos;
    if c.special_timer > 0.0 && c.special_state == -1 {
        c.special_timer -= dt;
        if c.special_timer <= 0.0 {
            c.special_timer = 0.0;
            c.special_state = 0;
        }
        next_cast = c.px + 20.0 >= c.charapos;
    }
    if next_cast && let Some(k) = super::play(game).c.stack.pop_front() {
        let c = &mut super::play(game).c;
        c.charapos += 10.0;
        let pos = c.charapos;
        cast::reset(game, k);
        if let Some(cur) = super::play(game).c.current {
            let tag = super::play(game).casts[k].tag;
            super::play(game).casts[cur].next_tag = tag;
            cast::on_next_char(game, cur);
        }
        super::play(game).c.current = Some(k);
        cast::set_stage_position(game, k, pos);
    }
    let c = &super::play(game).c;
    let (px, py, r) = (c.px, c.py, c.r);
    super::backgrounds::move_cast(game, px, py, r);
    taichi::move_cast(game, px, py, r);
    cast::move_nanaca(game, px);
    cast::move_youko_block(game, px, py, r, dt);
    hud::contact_ready(game);
    for k in 0..super::play(game).casts.len() {
        cast::move_cast(game, k, px, py, r, dt);
        let cast = &super::play(game).casts[k];
        if !cast.in_stage {
            continue;
        }
        let (node, active, tag) = (cast.node, cast.active, cast.tag);
        let x = game.pos(node).0;
        if x <= -1350.0 || !active {
            let c = &super::play(game).c;
            if (tag != 0x6d || !c.youko_block) && active {
                let n = c.stack.len();
                let pos = (n as f32 * 4.656_613e-10 * game.rng.lrand48() as f32).round() as usize;
                let stack = &mut super::play(game).c.stack;
                stack.insert(pos.min(stack.len()), k);
                super::play(game).casts[k].in_stage = false;
            }
        } else {
            game.set_visible(node, true);
        }
    }
    // Le mode aérien (jauge sous 10 m, chute) et son étiquette.
    let c = &super::play(game).c;
    let (charging, falling) = (c.py <= 10.0, c.vy < 0.0);
    let mode = update_aerial_state(game, charging, falling, dt);
    let indicator = super::play(game).hud.aerial_indicator;
    if mode != super::play(game).c.aerial_mode {
        super::play(game).c.aerial_mode = mode;
        game.set_color(indicator, [0xCC; 3]);
        game.set_rotation(indicator, 0.0);
        match mode {
            0 => {
                game.set_color(indicator, [0xFF, 0x66, 0x66]);
                game.set_rotation(indicator, 10.0);
            }
            2 => {
                game.set_color(indicator, [0x66, 0x66, 0xFF]);
                game.set_rotation(indicator, -10.0);
            }
            _ => {}
        }
    }
    // Toutes les 5 images : score, record, compteurs.
    let c = &mut super::play(game).c;
    c.score_frames += 1;
    if c.score_frames > 4 {
        c.score_frames = 0;
        let (px, py, r, demo, vx, vy, blind) = (c.px, c.py, c.r, c.demo, c.vx, c.vy, c.speed_blind);
        let record = super::play(game).hud.record;
        hud::set_score(game, record, &format!("{:9.2}:", px.min(1_000_000.0)));
        let best = super::play(game).best;
        if px >= best && !super::play(game).c.record_beaten {
            super::play(game).c.record_beaten = true;
            hud::set_score_color(game, record, [0xFF, 0, 0]);
        }
        hud::out_marker(game, py, r, demo);
        hud::special_indicator(game);
        if !blind {
            hud::speed_meter(game, vx, vy, demo);
        }
    }
    // Les feux d'artifice : toutes les 30 images, 20 % de chances.
    if super::play(game).fireworks.is_some() {
        let c = &mut super::play(game).c;
        c.firework_frames += 1;
        if c.firework_frames > 0x1d {
            c.firework_frames = 0;
            if game.rng.lrand48() as f32 * 4.656_613e-10 >= 0.8 {
                super::fireworks::ignite(game);
            }
        }
    }
}

/// clamp_angle : la rotation ramenée dans un tour.
fn clamp_angle(a: f32) -> f32 {
    let mut a = a % 360.0;
    if a < 0.0 {
        a += 360.0;
    }
    a
}

impl Play {
    /// Le vol est en cours (StatePlay).
    pub fn flying(&self) -> bool {
        matches!(self.state, State::Play)
    }
}
