//! Les actions de Cocos2d 2.x (CCAction) : déplacements, échelles,
//! rotations, fondus, teintes, sauts, clignotements, animations d'images,
//! appels, séquences, ensembles, boucles et adoucissements, avec leurs
//! particularités (le premier pas n'avance pas le temps, les déplacements
//! relatifs se cumulent).

use std::rc::Rc;

use crate::assets::Frame;
use crate::node::{Kind, Node, NodeId};

/// Un appel de la logique, lancé après la mise à jour des actions.
pub type Callback = Rc<dyn Fn(&mut crate::game::Game, NodeId)>;

/// Ce qu'une action demande à la logique.
pub enum Event {
    Call(Callback, NodeId),
    /// CCRemoveSelf.
    Remove(NodeId),
}

#[derive(Clone, Copy, Debug)]
pub enum Ease {
    In(f32),
    Out(f32),
    BounceOut,
}

impl Ease {
    fn apply(self, t: f32) -> f32 {
        match self {
            Ease::In(r) => t.powf(r),
            Ease::Out(r) => t.powf(1.0 / r),
            Ease::BounceOut => bounce(t),
        }
    }
}

fn bounce(t: f32) -> f32 {
    if t < 1.0 / 2.75 {
        7.5625 * t * t
    } else if t < 2.0 / 2.75 {
        let t = t - 1.5 / 2.75;
        7.5625 * t * t + 0.75
    } else if t < 2.5 / 2.75 {
        let t = t - 2.25 / 2.75;
        7.5625 * t * t + 0.9375
    } else {
        let t = t - 2.625 / 2.75;
        7.5625 * t * t + 0.984375
    }
}

/// Ce que fait une action ; l'état de départ est pris au lancement.
#[derive(Clone)]
pub enum Op {
    Delay,
    MoveTo { end: (f32, f32), start: (f32, f32), delta: (f32, f32), previous: (f32, f32) },
    MoveBy { delta: (f32, f32), start: (f32, f32), previous: (f32, f32) },
    JumpBy { delta: (f32, f32), height: f32, jumps: u32, start: (f32, f32), previous: (f32, f32) },
    ScaleTo { end: (f32, f32), start: (f32, f32) },
    /// Rotations x et y (CCRotateTo à deux angles).
    RotateTo { end: (f32, f32), start: (f32, f32), diff: (f32, f32) },
    RotateBy { angle: f32, start: (f32, f32) },
    FadeTo { end: u8, start: u8 },
    FadeIn,
    FadeOut,
    TintTo { end: [u8; 3], start: [u8; 3] },
    Blink { times: u32, original: bool },
    /// Images d'une animation, le départ de chacune (fraction d'une
    /// boucle), le nombre de boucles.
    Animate { frames: Rc<Vec<Rc<Frame>>>, splits: Rc<Vec<f32>>, loops: u32, restore: bool, original: Option<Rc<Frame>>, next: usize, executed: u32 },
    ProgressTo { end: f32, start: f32 },
    /// CCOrbitCamera, réduite à un retournement horizontal (rotation autour
    /// de l'axe vertical vue de face).
    Orbit { start_z: f32, delta_z: f32 },
    Sequence { actions: Box<[Action; 2]>, split: f32, last: i32 },
    Spawn { actions: Box<[Action; 2]> },
    RepeatForever { inner: Box<Action> },
    Ease { inner: Box<Action>, ease: Ease },
    Show,
    Hide,
    Call(Callback),
    RemoveSelf,
}

#[derive(Clone)]
pub struct Action {
    pub op: Op,
    pub duration: f32,
    elapsed: f32,
    first_tick: bool,
    target: Option<NodeId>,
}

impl Action {
    fn new(op: Op, duration: f32) -> Action {
        Action { op, duration, elapsed: 0.0, first_tick: true, target: None }
    }

    // --- Fabriques (comme les create de Cocos2d) ------------------------------

    pub fn delay(d: f32) -> Action {
        Action::new(Op::Delay, d)
    }
    pub fn move_to(d: f32, end: (f32, f32)) -> Action {
        Action::new(Op::MoveTo { end, start: (0.0, 0.0), delta: (0.0, 0.0), previous: (0.0, 0.0) }, d)
    }
    pub fn move_by(d: f32, delta: (f32, f32)) -> Action {
        Action::new(Op::MoveBy { delta, start: (0.0, 0.0), previous: (0.0, 0.0) }, d)
    }
    pub fn jump_by(d: f32, delta: (f32, f32), height: f32, jumps: u32) -> Action {
        Action::new(Op::JumpBy { delta, height, jumps, start: (0.0, 0.0), previous: (0.0, 0.0) }, d)
    }
    pub fn scale_to(d: f32, s: f32) -> Action {
        Action::new(Op::ScaleTo { end: (s, s), start: (0.0, 0.0) }, d)
    }
    pub fn scale_to_xy(d: f32, sx: f32, sy: f32) -> Action {
        Action::new(Op::ScaleTo { end: (sx, sy), start: (0.0, 0.0) }, d)
    }
    pub fn rotate_to(d: f32, angle: f32) -> Action {
        Action::rotate_to_xy(d, angle, angle)
    }
    pub fn rotate_to_xy(d: f32, x: f32, y: f32) -> Action {
        Action::new(Op::RotateTo { end: (x, y), start: (0.0, 0.0), diff: (0.0, 0.0) }, d)
    }
    pub fn rotate_by(d: f32, angle: f32) -> Action {
        Action::new(Op::RotateBy { angle, start: (0.0, 0.0) }, d)
    }
    pub fn fade_to(d: f32, opacity: u8) -> Action {
        Action::new(Op::FadeTo { end: opacity, start: 0 }, d)
    }
    pub fn fade_in(d: f32) -> Action {
        Action::new(Op::FadeIn, d)
    }
    pub fn fade_out(d: f32) -> Action {
        Action::new(Op::FadeOut, d)
    }
    pub fn tint_to(d: f32, rgb: [u8; 3]) -> Action {
        Action::new(Op::TintTo { end: rgb, start: [0; 3] }, d)
    }
    pub fn blink(d: f32, times: u32) -> Action {
        Action::new(Op::Blink { times, original: true }, d)
    }
    /// Une animation : images et unités de délai, `delay` secondes l'unité,
    /// `loops` fois.
    pub fn animate(frames: Vec<(Rc<Frame>, f32)>, delay: f32, loops: u32, restore: bool) -> Action {
        let units: f32 = frames.iter().map(|f| f.1).sum();
        let loops = loops.max(1);
        let mut splits = Vec::with_capacity(frames.len());
        let mut acc = 0.0;
        for f in &frames {
            splits.push(if units > 0.0 { acc / units } else { 0.0 });
            acc += f.1;
        }
        let frames = frames.into_iter().map(|f| f.0).collect();
        let op = Op::Animate { frames: Rc::new(frames), splits: Rc::new(splits), loops, restore, original: None, next: 0, executed: 0 };
        Action::new(op, units * delay * loops as f32)
    }
    pub fn progress_to(d: f32, percent: f32) -> Action {
        Action::new(Op::ProgressTo { end: percent, start: 0.0 }, d)
    }
    pub fn orbit(d: f32, start_z: f32, delta_z: f32) -> Action {
        Action::new(Op::Orbit { start_z, delta_z }, d)
    }
    pub fn show() -> Action {
        Action::new(Op::Show, 0.0)
    }
    pub fn hide() -> Action {
        Action::new(Op::Hide, 0.0)
    }
    pub fn call(f: impl Fn(&mut crate::game::Game, NodeId) + 'static) -> Action {
        Action::new(Op::Call(Rc::new(f)), 0.0)
    }
    pub fn remove_self() -> Action {
        Action::new(Op::RemoveSelf, 0.0)
    }

    /// CCSequence::create : deux à deux, de gauche à droite.
    pub fn sequence(actions: Vec<Action>) -> Action {
        let mut it = actions.into_iter();
        let mut prev = it.next().unwrap_or_else(|| Action::delay(0.0));
        let mut any = false;
        for a in it {
            prev = Action::sequence2(prev, a);
            any = true;
        }
        if !any {
            // Une seule action : suivie d'un rien (comme Cocos2d).
            prev = Action::sequence2(prev, Action::new(Op::Delay, 0.0));
        }
        prev
    }

    fn sequence2(one: Action, two: Action) -> Action {
        let d = one.duration + two.duration;
        let split = one.duration / d.max(f32::EPSILON);
        Action::new(Op::Sequence { actions: Box::new([one, two]), split, last: -1 }, d)
    }

    /// CCSpawn::create : deux à deux, la plus courte prolongée d'une attente.
    pub fn spawn(actions: Vec<Action>) -> Action {
        let mut it = actions.into_iter();
        let mut prev = it.next().unwrap_or_else(|| Action::delay(0.0));
        for a in it {
            prev = Action::spawn2(prev, a);
        }
        prev
    }

    fn spawn2(mut one: Action, mut two: Action) -> Action {
        let (d1, d2) = (one.duration, two.duration);
        if d1 > d2 {
            two = Action::sequence2(two, Action::delay(d1 - d2));
        } else if d1 < d2 {
            one = Action::sequence2(one, Action::delay(d2 - d1));
        }
        Action::new(Op::Spawn { actions: Box::new([one, two]) }, d1.max(d2))
    }

    pub fn forever(inner: Action) -> Action {
        Action::new(Op::RepeatForever { inner: Box::new(inner) }, 0.0)
    }

    pub fn ease(inner: Action, ease: Ease) -> Action {
        let d = inner.duration;
        Action::new(Op::Ease { inner: Box::new(inner), ease }, d)
    }

    fn instant(&self) -> bool {
        matches!(self.op, Op::Show | Op::Hide | Op::Call(_) | Op::RemoveSelf)
    }

    // --- Déroulement -----------------------------------------------------------

    /// startWithTarget.
    pub fn start(&mut self, id: NodeId, node: &mut Node) {
        self.target = Some(id);
        self.elapsed = 0.0;
        self.first_tick = true;
        match &mut self.op {
            Op::MoveTo { end, start, delta, previous } => {
                *start = node.pos;
                *previous = node.pos;
                *delta = (end.0 - start.0, end.1 - start.1);
            }
            Op::MoveBy { start, previous, .. } | Op::JumpBy { start, previous, .. } => {
                *start = node.pos;
                *previous = node.pos;
            }
            Op::ScaleTo { start, .. } => *start = node.scale,
            Op::RotateTo { end, start, diff } => {
                // Le plus court chemin, pour x et pour y.
                let shortest = |r: f32, e: f32| {
                    let s = if r > 0.0 { r % 360.0 } else { r % -360.0 };
                    let mut d = e - s;
                    if d > 180.0 {
                        d -= 360.0;
                    }
                    if d < -180.0 {
                        d += 360.0;
                    }
                    (s, d)
                };
                let (sx, dx) = shortest(node.rotation.0, end.0);
                let (sy, dy) = shortest(node.rotation.1, end.1);
                *start = (sx, sy);
                *diff = (dx, dy);
            }
            Op::RotateBy { start, .. } => *start = node.rotation,
            Op::FadeTo { start, .. } => *start = node.opacity,
            Op::TintTo { start, .. } => *start = node.color,
            Op::Blink { original, .. } => *original = node.visible,
            Op::Animate { original, next, executed, restore, .. } => {
                *next = 0;
                *executed = 0;
                *original = match &node.kind {
                    Kind::Sprite(s) if *restore => s.frame.clone(),
                    _ => None,
                };
            }
            Op::ProgressTo { start, .. } => {
                if let Kind::Progress(p) = &node.kind {
                    *start = p.percent;
                }
            }
            Op::Sequence { last, .. } => *last = -1,
            Op::Spawn { actions } => {
                for a in actions.iter_mut() {
                    a.start(id, node);
                }
            }
            Op::RepeatForever { inner } | Op::Ease { inner, .. } => inner.start(id, node),
            _ => {}
        }
    }

    /// stop : rend ce qu'il faut (clignotement, image d'origine).
    pub fn stop(&mut self, node: &mut Node) {
        match &mut self.op {
            Op::Blink { original, .. } => node.visible = *original,
            Op::Animate { restore: true, original: Some(f), .. } => {
                if let Kind::Sprite(s) = &mut node.kind {
                    node.size = s.set_frame(f);
                }
            }
            Op::Sequence { actions, last, .. } => {
                if *last >= 0 {
                    actions[*last as usize].stop(node);
                }
            }
            Op::Spawn { actions } => {
                for a in actions.iter_mut() {
                    a.stop(node);
                }
            }
            Op::RepeatForever { inner } | Op::Ease { inner, .. } => inner.stop(node),
            _ => {}
        }
    }

    pub fn done(&self) -> bool {
        match &self.op {
            Op::RepeatForever { .. } => false,
            _ if self.instant() => true,
            _ => self.elapsed >= self.duration,
        }
    }

    /// Un pas de `dt` secondes (CCActionInterval::step).
    pub fn step(&mut self, dt: f32, node: &mut Node, events: &mut Vec<Event>) {
        if let Op::RepeatForever { inner } = &mut self.op {
            inner.step(dt, node, events);
            if inner.done() {
                let diff = inner.elapsed - inner.duration;
                let id = self.target.unwrap_or_else(|| inner.target.expect("cible"));
                inner.start(id, node);
                inner.step(0.0, node, events);
                inner.step(diff, node, events);
            }
            return;
        }
        if self.instant() {
            self.update(1.0, node, events);
            return;
        }
        if self.first_tick {
            self.first_tick = false;
            self.elapsed = 0.0;
        } else {
            self.elapsed += dt;
        }
        let t = (self.elapsed / self.duration.max(f32::EPSILON)).clamp(0.0, 1.0);
        self.update(t, node, events);
    }

    /// L'état à la fraction `t` de la durée.
    pub fn update(&mut self, t: f32, node: &mut Node, events: &mut Vec<Event>) {
        let target = self.target;
        match &mut self.op {
            Op::Delay => {}
            Op::MoveTo { start, delta, previous, .. } | Op::MoveBy { start, previous, delta } => {
                // Cumulables : un autre déplacement en même temps s'ajoute.
                let cur = node.pos;
                start.0 += cur.0 - previous.0;
                start.1 += cur.1 - previous.1;
                let p = (start.0 + delta.0 * t, start.1 + delta.1 * t);
                node.pos = p;
                *previous = p;
            }
            Op::JumpBy { delta, height, jumps, start, previous } => {
                let frac = (t * *jumps as f32) % 1.0;
                let y = *height * 4.0 * frac * (1.0 - frac) + delta.1 * t;
                let x = delta.0 * t;
                let cur = node.pos;
                start.0 += cur.0 - previous.0;
                start.1 += cur.1 - previous.1;
                let p = (start.0 + x, start.1 + y);
                node.pos = p;
                *previous = p;
            }
            Op::ScaleTo { end, start } => node.scale = (start.0 + (end.0 - start.0) * t, start.1 + (end.1 - start.1) * t),
            Op::RotateTo { start, diff, .. } => node.rotation = (start.0 + diff.0 * t, start.1 + diff.1 * t),
            Op::RotateBy { angle, start } => node.rotation = (start.0 + *angle * t, start.1 + *angle * t),
            Op::FadeTo { end, start } => node.opacity = (f32::from(*start) + (f32::from(*end) - f32::from(*start)) * t) as u8,
            Op::FadeIn => node.opacity = (255.0 * t) as u8,
            Op::FadeOut => node.opacity = (255.0 * (1.0 - t)) as u8,
            Op::TintTo { end, start } => {
                for i in 0..3 {
                    node.color[i] = (f32::from(start[i]) + (f32::from(end[i]) - f32::from(start[i])) * t) as u8;
                }
            }
            Op::Blink { times, .. } => {
                if t < 1.0 {
                    let slice = 1.0 / *times as f32;
                    let m = t % slice;
                    node.visible = m > slice / 2.0;
                }
            }
            Op::Animate { frames, splits, loops, next, executed, .. } => {
                let mut t = t;
                if t < 1.0 {
                    t *= *loops as f32;
                    if t as u32 > *executed {
                        *next = 0;
                        *executed += 1;
                    }
                    t %= 1.0;
                }
                for i in *next..frames.len() {
                    if splits[i] > t {
                        break;
                    }
                    if let Kind::Sprite(s) = &mut node.kind {
                        node.size = s.set_frame(&frames[i]);
                    }
                    *next = i + 1;
                }
            }
            Op::ProgressTo { end, start } => {
                if let Kind::Progress(p) = &mut node.kind {
                    p.percent = *start + (*end - *start) * t;
                }
            }
            Op::Orbit { start_z, delta_z } => {
                // Vue de face, la rotation autour de l'axe vertical écrase x.
                let angle = (*start_z + *delta_z * t).to_radians();
                node.scale.0 = angle.cos();
            }
            Op::Sequence { actions, split, last } => {
                let (found, new_t) = if t < *split {
                    (0, if *split != 0.0 { t / *split } else { 1.0 })
                } else {
                    (1, if *split == 1.0 { 1.0 } else { (t - *split) / (1.0 - *split) })
                };
                let id = target.expect("cible");
                if found == 1 {
                    if *last == -1 {
                        // La première a été sautée : jouée d'un coup.
                        actions[0].start(id, node);
                        actions[0].update(1.0, node, events);
                        actions[0].stop(node);
                    } else if *last == 0 {
                        actions[0].update(1.0, node, events);
                        actions[0].stop(node);
                    }
                } else if *last == 1 {
                    actions[1].update(0.0, node, events);
                    actions[1].stop(node);
                }
                if found == *last && actions[found as usize].done() {
                    return;
                }
                if found != *last {
                    actions[found as usize].start(id, node);
                }
                actions[found as usize].update(new_t, node, events);
                *last = found;
            }
            Op::Spawn { actions } => {
                for a in actions.iter_mut() {
                    a.update(t, node, events);
                }
            }
            Op::RepeatForever { .. } => {}
            Op::Ease { inner, ease } => {
                let e = ease.apply(t);
                inner.update(e, node, events);
            }
            Op::Show => node.visible = true,
            Op::Hide => node.visible = false,
            Op::Call(f) => events.push(Event::Call(f.clone(), target.expect("cible"))),
            Op::RemoveSelf => events.push(Event::Remove(target.expect("cible"))),
        }
    }
}

/// Une action en cours sur un nœud, avec son étiquette.
#[derive(Clone)]
pub struct Running {
    pub action: Action,
    pub tag: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{Kind, Node};

    fn run(action: Action, frames: usize, dt: f32) -> Node {
        let mut graph = crate::node::Graph::default();
        let id = graph.add(Node::new(Kind::Node));
        let mut node = Node::new(Kind::Node);
        let mut a = action;
        a.start(id, &mut node);
        let mut events = Vec::new();
        for _ in 0..frames {
            a.step(dt, &mut node, &mut events);
        }
        node
    }

    #[test]
    fn first_tick() {
        // Le premier pas ne compte pas : 1 s à 0.25 s le pas = 5 pas.
        let n = run(Action::move_by(1.0, (100.0, 0.0)), 4, 0.25);
        assert!((n.pos.0 - 75.0).abs() < 1e-3);
        let n = run(Action::move_by(1.0, (100.0, 0.0)), 5, 0.25);
        assert!((n.pos.0 - 100.0).abs() < 1e-3);
    }

    #[test]
    fn sequence() {
        let a = Action::sequence(vec![Action::move_by(0.5, (10.0, 0.0)), Action::move_by(0.5, (0.0, 10.0))]);
        let n = run(a, 5, 0.25);
        assert!((n.pos.0 - 10.0).abs() < 1e-3 && (n.pos.1 - 10.0).abs() < 1e-3);
    }

    #[test]
    fn rotate_to_shortest() {
        let mut graph = crate::node::Graph::default();
        let id = graph.add(Node::new(Kind::Node));
        let mut node = Node::new(Kind::Node);
        node.set_rotation(350.0);
        let mut a = Action::rotate_to(1.0, 10.0);
        a.start(id, &mut node);
        a.update(0.5, &mut node, &mut Vec::new());
        assert!((node.rotation.0 - 360.0).abs() < 1e-3);
    }
}
