//! Les décors de chaque skin (Backgrounds_* et BackgroundsFar_*) : des
//! rangées de sprites qui défilent à leurs vitesses. Les particularités
//! d'origine sont gardées : quelques sprites ne bougent jamais (leur
//! pointeur est écrasé par la rangée suivante).

use crate::game::Game;
use crate::node::NodeId;

use super::play;

/// Le décor proche et le lointain d'un skin.
#[derive(Clone)]
pub enum Background {
    WayToSchool { rail: Vec<NodeId>, woods0: Vec<NodeId>, woods1: Vec<NodeId>, woods2: Vec<NodeId>, hokora: NodeId, mountains: Vec<NodeId>, clouds1: [NodeId; 2], clouds2: [NodeId; 2] },
    LastSummer { front: Vec<NodeId>, back: Vec<NodeId>, palms: Vec<NodeId>, toy: NodeId, sealine: Vec<NodeId>, islands: [NodeId; 2], clouds1: [NodeId; 2], clouds2: [NodeId; 2] },
    FireworkShow { woods0: Vec<NodeId>, woods1: Vec<NodeId>, rail: Vec<NodeId>, lights: Vec<NodeId>, mountains: Vec<NodeId>, night: Vec<NodeId> },
}

/// Une rangée de `n` sprites côte à côte (ancrés en bas au milieu).
fn row(game: &mut Game, batch: NodeId, frame: &str, n: usize, y: f32, z: i32, first_tag: i32) -> Vec<NodeId> {
    let width = {
        let probe = game.sprite(frame);
        let w = game.size(probe).0;
        game.g.remove(probe);
        w
    };
    (0..n)
        .map(|i| {
            let s = game.sprite(frame);
            game.set_pos(s, width * i as f32, y);
            game.set_anchor(s, 0.5, 0.0);
            game.add(batch, s, z, first_tag + i as i32);
            s
        })
        .collect()
}

/// Les décors du skin : (proche, lointain).
pub fn create(game: &mut Game, size: (f32, f32)) -> (NodeId, NodeId, Background) {
    let near = game.g.layer(size);
    let far = game.g.layer(size);
    let near_batch = game.g.node();
    let far_batch = game.g.node();
    let background = match game.skin_key() {
        "firework_show" => {
            let woods0 = row(game, near_batch, "n_woods0.png", 9, 0.0, 2, 0);
            let woods1 = row(game, near_batch, "n_woods1.png", 8, 50.0, 1, 8);
            let rail = row(game, near_batch, "n_rail.png", 7, 0.0, 5, 0xf);
            let lights = [-1100.0, 100.0, 1300.0, 2500.0, 3700.0]
                .into_iter()
                .enumerate()
                .map(|(i, x)| {
                    let l = game.sprite("light.png");
                    game.set_anchor(l, 0.5, 0.0);
                    game.set_x(l, x);
                    game.add(near_batch, l, 4, 0x16 + i as i32);
                    l
                })
                .collect();
            game.add(near, near_batch, 0, -1);
            let mountains = row(game, far_batch, "n_mountain.png", 6, 100.0, 0, 0);
            let night = row(game, far_batch, "nightscape.png", 4, -10.0, 1, 5);
            game.add(far, far_batch, 0, -1);
            Background::FireworkShow { woods0, woods1, rail, lights, mountains, night }
        }
        "last_summer" => {
            let front = row(game, near_batch, "shorefront.png", 8, 0.0, 0x10, 0);
            let back = row(game, near_batch, "shoreback.png", 8, 10.0, 1, 7);
            let mut palms = Vec::new();
            let mut x = -300.0;
            for k in 15..0x1a {
                let (name, flipped, y) = match k & 3 {
                    0 => ("palmtree.png", true, -((k >> 2) as f32)),
                    3 => ("palmtree.png", false, (k >> 2) as f32),
                    1 => ("palmtree2.png", false, (k >> 2) as f32),
                    _ => ("palmtree2.png", true, -((k >> 2) as f32)),
                };
                let p = game.sprite(name);
                game.set_y(p, y);
                game.set_anchor(p, 0.5, 0.0);
                game.set_x(p, x);
                if flipped && let Some(n) = game.g.get_mut(p) {
                    n.scale.0 = -1.0;
                }
                game.add(near_batch, p, 4, k);
                palms.push(p);
                x += 400.0;
            }
            let toy = game.sprite("yusa.png");
            game.set_anchor(toy, 0.5, 0.0);
            game.set_pos(toy, 400.0, 20.0);
            game.add(near_batch, toy, 3, 0x15);
            game.add(near, near_batch, 0, -1);
            let (w, h) = size;
            let sky = game.sprite_file("last_summer_sky.jpg");
            game.set_pos(sky, w * 0.5, h * 0.5);
            game.add(far, sky, -1, -1);
            let sealine = row(game, far_batch, "sealine.png", 4, -20.0, 2, 0);
            let islands = [50.0, 750.0].map(|x| {
                let s = game.sprite("island.png");
                game.set_pos(s, x, 85.0);
                game.add(far_batch, s, 3, 4);
                s
            });
            let lens = game.sprite("lensfrare.png");
            let (lw, lh) = game.size(lens);
            game.set_pos(lens, lw * 0.5, h - lh * 0.5);
            game.add(far_batch, lens, 6, 5);
            let clouds = |game: &mut Game, name: &str, ys: [f32; 2], tags: [i32; 2]| {
                let a = game.sprite(name);
                let b = game.sprite(name);
                let ch = game.size(a).1;
                for (k, (c, x)) in [(a, 0.0), (b, 700.0)].into_iter().enumerate() {
                    if let Some(n) = game.g.get_mut(c) {
                        n.scale.0 = -1.0;
                    }
                    game.add(far_batch, c, 1, tags[k]);
                    game.set_pos(c, x, ch + ys[k]);
                }
                [a, b]
            };
            let clouds1 = clouds(game, "cloud1.png", [65.0, 65.0], [6, 7]);
            let clouds2 = clouds(game, "cloud2.png", [150.0, 160.0], [8, 9]);
            game.add(far, far_batch, 0, -1);
            Background::LastSummer { front, back, palms, toy, sealine, islands, clouds1, clouds2 }
        }
        _ => {
            let rail = row(game, near_batch, "railelement.png", 6, 0.0, 5, 0);
            let woods0 = row(game, near_batch, "woods0.png", 7, -30.0, 4, 6);
            let woods1 = row(game, near_batch, "woods1.png", 6, 0.0, 4, 0xd);
            let woods2 = row(game, near_batch, "woods2.png", 6, 0.0, 2, 0x13);
            let hokora = game.sprite("hokora.png");
            game.set_anchor(hokora, 0.5, 0.0);
            game.set_pos(hokora, 2048.0, 0.0);
            game.add(near_batch, hokora, 3, 10);
            let band = game.g.color_layer([0x6d, 0xcc, 0xc9, 0xff], (2000.0, 100.0));
            game.set_pos(band, -200.0, 40.0);
            game.set_anchor(band, 0.0, 0.0);
            game.add(near, band, 0, -1);
            game.add(near, near_batch, 0, -1);
            let mountains = row(game, far_batch, "mountelement.png", 6, 30.0, 0, 0);
            let cloud = |game: &mut Game, name: &str, xs: [f32; 2], dy: f32, tags: [i32; 2]| {
                let a = game.sprite(name);
                let b = game.sprite(name);
                let ch = game.size(a).1;
                game.add(far_batch, a, 1, tags[0]);
                game.add(far_batch, b, 1, tags[1]);
                game.set_pos(a, xs[0], ch + dy);
                game.set_pos(b, xs[1], ch + dy);
                [a, b]
            };
            let clouds1 = cloud(game, "cloud1.png", [0.0, 600.0], 200.0, [6, 7]);
            let clouds2 = cloud(game, "cloud2.png", [200.0, 700.0], 125.0, [8, 9]);
            game.add(far, far_batch, 0, -1);
            Background::WayToSchool { rail, woods0, woods1, woods2, hokora, mountains, clouds1, clouds2 }
        }
    };
    (near, far, background)
}

/// La partie fractionnaire de x × taux (modff, en flottant).
fn frac(p: f64, rate: f64) -> f32 {
    ((p * rate) as f32).fract()
}

/// Une rangée qui défile : x = i × largeur − décalage − largeur.
fn scroll(game: &mut Game, sprites: &[NodeId], width: f32, shift: f32, y: Option<f32>) {
    for (i, &s) in sprites.iter().enumerate() {
        game.set_x(s, (f64::from(width) * i as f64 - f64::from(shift) - f64::from(width)) as f32);
        if let Some(y) = y {
            game.set_y(s, y);
        }
    }
}

/// Deux sprites qui se relaient : −l × f et (1 − f) × l.
fn pair(game: &mut Game, nodes: &[NodeId; 2], length: f32, f: f32) {
    game.set_x(nodes[0], -(length * f));
    game.set_x(nodes[1], ((1.0 - f64::from(f)) * f64::from(length)) as f32);
}

/// moveCast des décors proche et lointain.
pub fn move_cast(game: &mut Game, px: f32, py: f32, _r: f32) {
    let game_layer = play(game).game_layer;
    let Some(bg) = play(game).background.clone() else { return };
    let p = f64::from(px);
    let low = py.min(17.0);
    match bg {
        Background::WayToSchool { rail, woods0, woods1, woods2, hokora, mountains, clouds1, clouds2 } => {
            scroll(game, &rail, 512.0, frac(p, 0.2) * 512.0, None);
            scroll(game, &woods0, 581.0, frac(p, 0.114_942_528_735_632_18) * 581.0, None);
            let (y1, y2) = if low > 2.0 { (-(low - 2.0) * 2.0, (f64::from(low - 2.0) * 1.2) as f32) } else { (-0.0, 0.0) };
            scroll(game, &woods1, 581.0, frac(p, 0.049_261_083_743_842_37) * 581.0, Some(y1));
            scroll(game, &woods2, 431.0, frac(p, 0.05) * 431.0, Some(y2));
            game.set_x(hokora, (2048.0 - f64::from(frac(p, 0.008_210_180_623_973_728)) * 2560.0) as f32);
            // Le lointain : les montagnes ne se voient que de loin.
            let f = frac(p, 0.003_333_333_333_333_333_5);
            let zoomed_out = game.scale(game_layer) <= 0.5;
            for &m in &mountains {
                game.set_visible(m, zoomed_out);
            }
            if zoomed_out {
                scroll(game, &mountains, 172.0, (f64::from(f) * 172.0) as f32, None);
            }
            pair(game, &clouds1, 600.0, frac(p, 0.002_222_222_222_222_222));
            pair(game, &clouds2, 500.0, f);
        }
        Background::LastSummer { front, back, palms, toy, sealine, islands, clouds1, clouds2 } => {
            // La huitième rive avant ne bouge pas (pointeur écrasé).
            scroll(game, &front[..7], 356.0, frac(p, 0.2) * 356.0, None);
            let y = if low > 2.0 { 10.0 - (low - 2.0) } else { 10.0 };
            scroll(game, &back, 356.0, frac(p, 0.040_000_000_000_000_01) * 356.0, Some(y));
            let base = -300.0 - f64::from(frac(p, 0.041_666_666_666_666_664)) * 1600.0;
            for (k, &palm) in palms.iter().enumerate() {
                game.set_x(palm, (base + 400.0 * k as f64) as f32);
            }
            // L'objet de plage, et le suivant tiré au hasard.
            let f = f64::from(frac(p, 0.025));
            let toy_base = play(game).toy_base;
            game.set_x(toy, (f64::from(toy_base) - f * 2560.0) as f32);
            if f >= 0.99 {
                play(game).toy_base = 2048.0;
                let k = (game.rng.lrand48() as f32 * 4.656_613e-7) as i32;
                let name = match k % 4 {
                    1 => "ball.png",
                    2 => "watermelon.png",
                    0 => "sandcastle.png",
                    _ => "yusa.png",
                };
                let assets = game.assets.clone();
                game.g.set_frame(&assets, toy, name);
            }
            let ys = if low > 2.0 { (f64::from(low - 2.0) * 0.45 - 20.0) as f32 } else { -20.0 };
            let fs = f64::from(frac(p, 0.002));
            for (i, &s) in sealine.iter().enumerate() {
                game.set_visible(s, true);
                game.set_x(s, ((i as f64 - 1.0 - fs) * 640.0) as f32);
                game.set_y(s, ys);
            }
            pair(game, &islands, 700.0, frac(p, 3.333_333_333_333_333e-4));
            pair(game, &clouds1, 700.0, frac(p, 6.666_666_666_666_666e-4));
            pair(game, &clouds2, 700.0, frac(p, 0.001));
        }
        Background::FireworkShow { woods0, woods1, rail, lights, mountains, night } => {
            let (y0, y1) = if low <= 2.0 { (-0.0, 50.0) } else { (-((f64::from(low - 2.0) * 3.6) as f32), (50.0 - f64::from(low - 2.0) * 4.2) as f32) };
            // Le neuvième bois du fond et le huitième du milieu ne bougent pas.
            scroll(game, &woods0[..8], 355.0, (f64::from(frac(p, 0.066_666_666_666_666_67)) * 355.0) as f32, Some(y0));
            scroll(game, &woods1[..7], 355.0, (f64::from(frac(p, 0.040_000_000_000_000_01)) * 355.0) as f32, Some(y1));
            scroll(game, &rail, 355.0, (f64::from(frac(p, 0.2)) * 355.0) as f32, None);
            let base = -1100.0 - f64::from(frac(p, 0.016_666_666_666_666_666)) * 2400.0;
            for (k, &l) in lights.iter().enumerate() {
                game.set_x(l, (base + 1200.0 * k as f64) as f32);
            }
            // Le lointain : la hauteur n'y est pas bornée à 17.
            let f = frac(p, 6.666_666_666_666_666e-4);
            let (ym, yn) = if py >= 19.0 {
                (59.2, -55.5)
            } else if py <= 2.0 {
                (100.0, -30.0)
            } else {
                ((100.0 - f64::from(py - 2.0) * 2.4) as f32, (-30.0 - f64::from(py - 2.0) * 1.5) as f32)
            };
            // La sixième montagne ne bouge pas (pointeur écrasé).
            scroll(game, &mountains[..5], 355.0, (f64::from(f) * 355.0) as f32, Some(ym));
            scroll(game, &night, 490.0, f * 490.0, Some(yn));
        }
    }
}
