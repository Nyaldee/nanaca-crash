//! Le décor animé du skin (BackgroundsDemo_*) : sous les options, la
//! galerie et la page des musiques, il défile en boucle (moveCast).

use crate::node::NodeId;

use super::Game;

/// Un décor animé.
pub struct Demo {
    pub layer: NodeId,
    key: &'static str,
    /// Les éléments qui défilent, dans l'ordre de l'original.
    parts: Vec<NodeId>,
}

/// SkinResource::getBackgroundsDemo : le décor du skin choisi, ajouté à
/// `parent` en `z`.
pub fn create(game: &mut Game, parent: NodeId, z: i32) -> Demo {
    let (w, h) = game.design;
    let key = game.skin_key();
    let layer = game.g.layer((w, h));
    let mut parts = Vec::new();
    let batch = game.g.node();
    let gradient = |game: &mut Game, top: [u8; 4], bottom: [u8; 4]| {
        let g = game.g.gradient_layer(top, bottom, (w, h));
        if let Some(n) = game.g.get_mut(g) {
            n.anchor = (0.0, 0.0);
            n.ignore_anchor = true;
        }
        game.add(layer, g, -1, 10);
    };
    let row = |game: &mut Game, frame: &str, count: usize, y: f32, z: i32, first_tag: i32, parts: &mut Vec<NodeId>| {
        let width = {
            let probe = game.sprite(frame);
            let s = game.size(probe);
            game.g.remove(probe);
            s.0
        };
        for i in 0..count {
            let s = game.sprite(frame);
            game.set_anchor(s, 0.5, 0.0);
            game.set_pos(s, width * i as f32, y);
            game.add(batch, s, z, first_tag + i as i32);
            parts.push(s);
        }
    };
    match key {
        "last_summer" => {
            let sky = game.sprite_file("last_summer_sky.jpg");
            game.set_pos(sky, w * 0.5, h * 0.5);
            game.add(layer, sky, 0, -1);
            row(game, "sealine.png", 4, -30.0, 0, 0, &mut parts);
            let island = game.sprite("island.png");
            game.set_pos(island, 100.0, 75.0);
            game.add(batch, island, 1, 4);
        }
        "firework_show" => {
            gradient(game, [0x0E, 0x06, 0x6C, 0xFF], [0xB4, 0x94, 0xBB, 0xFF]);
            row(game, "n_mountain.png", 6, 100.0, 0, 0, &mut parts);
            row(game, "nightscape.png", 5, -30.0, 4, 6, &mut parts);
        }
        _ => {
            gradient(game, [0x80, 0xD0, 0xFD, 0xFF], [0xFF; 4]);
            row(game, "mountelement.png", 6, -10.0, 0, 0, &mut parts);
            for (frame, xs, rise, first) in [("cloud1.png", [0.0, 600.0], 200.0, 6), ("cloud2.png", [200.0, 700.0], 125.0, 8)] {
                for (i, x) in xs.into_iter().enumerate() {
                    let c = game.sprite(frame);
                    let ch = game.size(c).1;
                    game.set_pos(c, x, ch + rise);
                    game.add(batch, c, 1, first + i as i32);
                    parts.push(c);
                }
            }
        }
    }
    game.add(layer, batch, 0, -1);
    game.add(parent, layer, z, -1);
    Demo { layer, key, parts }
}

/// BackgroundsDemo_*::moveCast : les éléments défilent en boucle.
pub fn move_cast(game: &mut Game, demo: &Demo, px: f32) {
    let frac = |x: f32| x.fract();
    let place = |game: &mut Game, k: usize, x: f32| {
        if let Some(&p) = demo.parts.get(k) {
            game.set_visible(p, true);
            game.set_x(p, x);
        }
    };
    match demo.key {
        "last_summer" => {
            let f = f64::from(frac((f64::from(px) * 0.001) as f32));
            for i in 0..4 {
                place(game, i, ((i as f64 - 1.0 - f) * 640.0) as f32);
            }
        }
        "firework_show" => {
            let f = f64::from(frac((f64::from(px) * 0.003_333_333_333_333_333_5) as f32));
            for i in 0..6 {
                place(game, i, (355.0 * i as f64 - f * 355.0 - 355.0) as f32);
            }
            for i in 0..5 {
                place(game, 6 + i, (490.0 * i as f64 - f * 490.0 - 490.0) as f32);
            }
        }
        _ => {
            let x = (f64::from(px) * 0.003_333_333_333_333_333_5) as f32;
            let d = f64::from(frac(x)) * 172.0;
            for i in 0..6 {
                place(game, i, (172.0 * i as f64 - d - 172.0) as f32);
            }
            let f = frac((f64::from(px) * 0.002_222_222_222_222_222) as f32);
            place(game, 6, -600.0 * f);
            place(game, 7, (1.0 - f) * 600.0);
            let f = frac(x);
            place(game, 8, -500.0 * f);
            place(game, 9, (1.0 - f) * 500.0);
        }
    }
}

/// Le défilement des décors (update des scènes) : 10 par seconde, remis
/// à zéro à 10⁸.
pub fn advance(scroll: &mut f32, dt: f32) {
    *scroll += dt * 10.0;
    if *scroll >= 1e8 {
        *scroll = 0.0;
    }
}
