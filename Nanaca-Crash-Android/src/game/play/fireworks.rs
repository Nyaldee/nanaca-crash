//! Les feux d'artifice du skin « Firework show » (FireworksLayer et
//! Firework) : une fusée monte, éclate en anneaux de particules qui
//! laissent une traînée, et les grosses gerbes font un flash blanc
//! (MSG_RUN_SPARK, SceneGame::runSpark).
//!
//! L'original réduit le nombre et la durée des feux si l'appareil rame
//! (moyenne des 10 derniers pas ≥ 0.032 s) : avec le pas fixe de 1/60 s,
//! cela n'arrive jamais, ce n'est donc pas repris.

use std::collections::VecDeque;

use crate::action::Action;
use crate::game::Game;
use crate::node::{Kind, NodeId};

use super::play;

/// Un feu d'artifice (Firework).
struct Firework {
    /// Particules par anneau (multipliées par le numéro de l'anneau).
    per_ring: i32,
    /// Longueur de la traînée (images).
    trail: usize,
    /// Durée restante et totale de la gerbe.
    life: f32,
    life_total: f32,
    /// Ampleur de la gerbe (x, y).
    scale: (f32, f32),
    /// Hauteur d'éclatement.
    target: f32,
    exploded: bool,
    /// La fusée : x, y, vx, vy.
    rocket: [f32; 4],
    /// Les particules : x, y, vx, vy.
    particles: Vec<[f32; 4]>,
    /// Positions passées (fusée ou particules), la plus ancienne devant.
    history: VecDeque<Vec<(f32, f32)>>,
    /// Couleur de chaque anneau (r, v, b, a).
    colors: Vec<[f32; 4]>,
}

impl Firework {
    /// Firework::init : `rings` anneaux de `per_ring` × n particules.
    fn new(game: &mut Game, pos: (f32, f32), drop: f32, scale: f32, life: f32, rings: i32, per_ring: i32) -> Firework {
        let total: i32 = (1..=rings).map(|i| per_ring * i).sum();
        let mut particles = Vec::with_capacity(total.max(0) as usize);
        let (mut ring, mut left, mut in_ring) = (1, per_ring, per_ring);
        let (mut angle, mut radius) = (0.0f32, 1.0 / rings as f32);
        for i in 0..total {
            let jitter_x = (-0.5 + game.rng.lrand48() as f32 * 4.656_613e-10) * 0.05;
            let vx = (angle.sin() * radius + jitter_x) * scale;
            let jitter_y = (-0.5 + game.rng.lrand48() as f32 * 4.656_613e-10) * 0.05;
            let vy = (angle.cos() * radius + jitter_y) * scale;
            particles.push([pos.0, pos.1, vx, vy]);
            left -= 1;
            if left < 1 {
                ring += 1;
                left = per_ring * ring;
                in_ring = left;
            }
            // L'angle suit le numéro de la particule dans toute la gerbe
            // (pas dans l'anneau), comme dans l'original.
            angle = (6.28318 / in_ring as f32) * (i + 1) as f32;
            radius = ring as f32 / rings as f32;
        }
        Firework {
            per_ring,
            trail: 5,
            life,
            life_total: life,
            scale: (scale, scale),
            target: pos.1,
            exploded: false,
            rocket: [pos.0, drop + pos.1, 0.0, -8.0],
            particles,
            history: VecDeque::new(),
            colors: Vec::new(),
        }
    }
}

/// La couche des feux (FireworksLayer).
pub struct Fireworks {
    pub layer: NodeId,
    draw: NodeId,
    list: Vec<Firework>,
    /// Durée d'une gerbe et feux simultanés au plus (aussi les particules
    /// du premier anneau).
    life: f32,
    max: i32,
}

/// FireworksLayer::init : la couche et son dessin (caché sans feu).
pub fn create(game: &mut Game) -> Fireworks {
    let layer = game.g.color_layer([0, 0, 0, 0], game.design);
    let draw = game.g.draw_node();
    game.set_pos(draw, 0.0, 0.0);
    game.add(layer, draw, 100, 100);
    game.set_visible(draw, false);
    Fireworks { layer, draw, list: Vec::new(), life: 2.0, max: 8 }
}

fn state(game: &mut Game) -> Option<&mut Fireworks> {
    play(game).fireworks.as_mut()
}

/// Les couleurs des anneaux.
const GOLD: [[f32; 4]; 4] = [[1.0, 1.0, 0.2, 1.0], [1.0, 0.8, 0.0, 1.0], [1.0, 0.4, 0.0, 1.0], [1.0, 0.1, 0.0, 1.0]];
const GREEN: [[f32; 4]; 4] = [[1.0, 1.0, 0.2, 1.0], [0.8, 1.0, 0.0, 1.0], [0.0, 0.8, 0.0, 1.0], [0.0, 1.0, 0.0, 1.0]];
const YELLOW: [[f32; 4]; 3] = [[1.0, 1.0, 0.4, 1.0], [1.0, 1.0, 0.2, 1.0], [1.0, 1.0, 0.0, 1.0]];
const PALE: [[f32; 4]; 3] = [[1.0, 1.0, 0.8, 1.0], [0.8, 1.0, 0.6, 1.0], [0.5, 0.8, 0.8, 1.0]];

/// FireworksLayer::ignite : un feu au hasard au-dessus du centre.
pub fn ignite(game: &mut Game) {
    let Some(fw) = state(game) else { return };
    if fw.list.len() as i32 >= fw.max {
        return;
    }
    let (life, max) = (fw.life, fw.max);
    let (w, h) = game.design;
    let r = [game.rng.lrand48(), game.rng.lrand48(), game.rng.lrand48(), game.rng.lrand48()].map(|v| v as f32);
    let pos = (-100.0 + w * 0.5 + r[0] * 9.313_226e-8, 100.0 + h * 0.5);
    let mut f = Firework::new(game, pos, -200.0, 1.0 + r[1] * 4.656_613e-10, life, 4, max);
    let (kind, trail) = (r[2] * 4.656_613e-10, r[3] * 4.656_613e-10);
    f.colors = if kind >= 0.75 {
        GOLD.to_vec()
    } else if kind >= 0.5 {
        GREEN.to_vec()
    } else if kind < 0.25 {
        YELLOW.to_vec()
    } else {
        PALE.to_vec()
    };
    f.trail = if trail >= 0.2 { 1 } else { 8 };
    add(game, f);
}

/// FireworksLayer::ignite_special : la grande gerbe du lancement parfait.
pub fn ignite_special(game: &mut Game) {
    let Some(fw) = state(game) else { return };
    let (life, max) = (fw.life, fw.max);
    let (w, h) = game.design;
    game.rng.lrand48();
    let mut f = Firework::new(game, (w * 0.5, 100.0 + h * 0.5), -200.0, 3.0, life + 1.0, 8, max);
    f.trail = 10;
    let [gold, orange, red, deep] = [GOLD[0], GOLD[1], GOLD[2], GOLD[3]];
    f.colors = vec![gold, orange, orange, orange, red, red, deep];
    add(game, f);
}

fn add(game: &mut Game, f: Firework) {
    let Some(fw) = state(game) else { return };
    fw.list.push(f);
    let draw = fw.draw;
    game.set_visible(draw, true);
}

/// SceneGame::runSpark : le flash blanc.
fn spark(game: &mut Game) {
    let Some(flash) = play(game).flash else { return };
    game.stop_actions(flash);
    game.set_opacity(flash, 0xFF);
    game.run(flash, Action::sequence(vec![Action::fade_out(0.25), Action::hide()]));
    game.set_visible(flash, true);
}

/// Un trait épais à bouts carrés (CCDrawNode::drawSegment), en deux
/// triangles.
fn segment(out: &mut Vec<([(f32, f32); 3], [f32; 4])>, a: (f32, f32), b: (f32, f32), radius: f32, color: [f32; 4]) {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let len = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if len > 0.0 { (dx / len, dy / len) } else { (1.0, 0.0) };
    let (nx, ny) = (-uy * radius, ux * radius);
    let (a, b) = ((a.0 - ux * radius, a.1 - uy * radius), (b.0 + ux * radius, b.1 + uy * radius));
    let p = [(a.0 + nx, a.1 + ny), (a.0 - nx, a.1 - ny), (b.0 + nx, b.1 + ny), (b.0 - nx, b.1 - ny)];
    out.push(([p[0], p[1], p[2]], color));
    out.push(([p[1], p[3], p[2]], color));
}

/// FireworksLayer::update : chaque feu avance et se redessine.
pub fn update(game: &mut Game, dt: f32) {
    let Some(fw) = state(game) else { return };
    let draw = fw.draw;
    if !game.visible(draw) {
        return;
    }
    if state(game).is_some_and(|fw| fw.list.is_empty()) {
        game.set_visible(draw, false);
        return;
    }
    let mut list = std::mem::take(&mut state(game).expect("feux").list);
    let mut triangles = Vec::new();
    let mut sparks = 0;
    list.retain_mut(|f| {
        if !f.exploded {
            let r = &mut f.rocket;
            let prev = f.history.front().and_then(|h| h.first().copied()).unwrap_or((r[0], r[1]));
            r[2] *= 0.98;
            r[3] = (dt + r[3]) * 0.98;
            r[0] += dt * 62.499_996 * r[2];
            r[1] -= dt * 62.499_996 * r[3];
            segment(&mut triangles, (r[0], r[1]), prev, 1.0, [1.0, 1.0, 0.0, 1.0]);
            f.history.push_back(vec![(r[0], r[1])]);
            if f.history.len() > 2 {
                f.history.pop_front();
            }
            if r[1] >= f.target {
                if f.scale.0 >= 1.8 {
                    sparks += 1;
                }
                f.exploded = true;
                f.history.clear();
            }
        } else {
            let mut color = f.colors.first().copied().unwrap_or([1.0; 4]);
            let mut ring = 1.0f32;
            let mut left = f.per_ring as f32;
            let fade = f.life / f.life_total;
            for i in 0..f.particles.len() {
                let p = &mut f.particles[i];
                p[2] *= 0.98;
                p[3] = (p[3] + dt) * 0.98;
                p[0] += dt * 62.499_996 * p[2];
                p[1] = (f64::from(p[1]) - f64::from(dt) * 62.5 * f64::from(p[3])) as f32;
                let alpha = (game.rng.lrand48() as f32 * 4.656_613e-10 + 0.5) * fade;
                let prev = f.history.front().and_then(|h| h.get(i).copied()).unwrap_or((p[0], p[1]));
                segment(&mut triangles, (p[0], p[1]), prev, 1.5, [color[0], color[1], color[2], alpha]);
                // Anneau suivant : sa couleur (la dernière reste au-delà).
                left -= 1.0;
                if left <= 0.0 {
                    ring += 1.0;
                    left = ring * f.per_ring as f32;
                    if ring <= f.colors.len() as f32 {
                        color = f.colors[ring as usize - 1];
                    }
                }
            }
            f.life -= dt;
            f.history.push_back(f.particles.iter().map(|p| (p[0], p[1])).collect());
            if f.history.len() >= f.trail {
                f.history.pop_front();
            }
        }
        f.life > 0.0
    });
    state(game).expect("feux").list = list;
    if let Some(n) = game.g.get_mut(draw)
        && let Kind::Draw(t) = &mut n.kind
    {
        *t = triangles;
    }
    for _ in 0..sparks {
        spark(game);
    }
}
