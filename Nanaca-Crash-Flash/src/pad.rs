//! Les manettes (XInput sous Windows, /dev/input/js* sous Linux), comme
//! dans le remake : la croix et le stick gauche passent d'un bouton à
//! l'autre, A valide, B revient en arrière, Start met en pause, Start +
//! Select quitte.

/// Les boutons des manettes branchées, réunis.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct Buttons {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub a: bool,
    pub b: bool,
    pub start: bool,
    pub select: bool,
}

#[cfg(windows)]
pub use windows::Pads;

#[cfg(not(windows))]
pub use unix::Pads;

/// Stick gauche : seuil de navigation (deux fois la zone morte conseillée
/// par XInput, un stick au repos ne bouge rien).
const STICK: i16 = 7849 * 2;
/// Images avant de réinterroger une manette absente.
const RETRY: u32 = 120;

#[cfg(not(windows))]
mod unix {
    use std::fs::File;
    use std::io::{ErrorKind, Read};
    use std::os::unix::fs::OpenOptionsExt;

    use super::{Buttons, RETRY, STICK};

    /// O_NONBLOCK : lire sans attendre d'événement.
    const O_NONBLOCK: i32 = 0o4000;
    /// js_event : type bouton, axe ; drapeau de l'état initial.
    const JS_BUTTON: u8 = 1;
    const JS_AXIS: u8 = 2;
    const JS_INIT: u8 = 0x80;

    /// Une manette ouverte : l'état de ses boutons et de ses axes.
    struct Pad {
        file: File,
        buttons: [bool; 16],
        axes: [i16; 8],
    }

    /// Les manettes de l'interface joystick du noyau (js0 à js3), dans la
    /// disposition des manettes Xbox (pilote xpad) : A 0, B 1, Select 6,
    /// Start 7 ; stick gauche axes 0 et 1, croix axes 6 et 7.
    pub struct Pads {
        /// Ouverte, ou images avant de réessayer.
        slots: [Result<Pad, u32>; 4],
    }

    impl Pads {
        pub fn new() -> Pads {
            Pads { slots: [Err(0), Err(0), Err(0), Err(0)] }
        }

        /// Les boutons de toutes les manettes branchées.
        pub fn poll(&mut self) -> Buttons {
            let mut pad = Buttons::default();
            for (i, slot) in self.slots.iter_mut().enumerate() {
                if let Err(wait) = slot {
                    if *wait > 0 {
                        *wait -= 1;
                        continue;
                    }
                    match std::fs::OpenOptions::new().read(true).custom_flags(O_NONBLOCK).open(format!("/dev/input/js{i}")) {
                        Ok(file) => *slot = Ok(Pad { file, buttons: [false; 16], axes: [0; 8] }),
                        Err(_) => {
                            *slot = Err(RETRY);
                            continue;
                        }
                    }
                }
                let Ok(p) = slot else { continue };
                if !p.drain() {
                    // Débranchée : on réessaiera plus tard.
                    *slot = Err(RETRY);
                    continue;
                }
                let (b, a) = (&p.buttons, &p.axes);
                pad.up |= a[7] < -STICK || a[1] < -STICK;
                pad.down |= a[7] > STICK || a[1] > STICK;
                pad.left |= a[6] < -STICK || a[0] < -STICK;
                pad.right |= a[6] > STICK || a[0] > STICK;
                pad.a |= b[0];
                pad.b |= b[1];
                pad.select |= b[6];
                pad.start |= b[7];
            }
            pad
        }
    }

    impl Pad {
        /// Lit les événements en attente ; faux si la manette a disparu.
        fn drain(&mut self) -> bool {
            let mut event = [0u8; 8];
            loop {
                match self.file.read(&mut event) {
                    Ok(8) => {
                        // js_event : temps (u32), valeur (i16), type, numéro.
                        let value = i16::from_ne_bytes([event[4], event[5]]);
                        let (kind, number) = (event[6] & !JS_INIT, event[7] as usize);
                        match kind {
                            JS_BUTTON if number < self.buttons.len() => self.buttons[number] = value != 0,
                            JS_AXIS if number < self.axes.len() => self.axes[number] = value,
                            _ => {}
                        }
                    }
                    Ok(_) => return false,
                    Err(e) if e.kind() == ErrorKind::WouldBlock => return true,
                    Err(e) if e.kind() == ErrorKind::Interrupted => {}
                    Err(_) => return false,
                }
            }
        }
    }
}

#[cfg(windows)]
mod windows {
    use super::{Buttons, RETRY, STICK};

    #[repr(C)]
    #[derive(Default)]
    struct XInputState {
        packet: u32,
        buttons: u16,
        triggers: [u8; 2],
        thumbs: [i16; 4],
    }

    #[link(name = "xinput")]
    unsafe extern "system" {
        fn XInputGetState(index: u32, state: *mut XInputState) -> u32;
    }

    pub struct Pads {
        /// Branchée, ou images avant de réessayer.
        state: [Result<(), u32>; 4],
    }

    impl Pads {
        pub fn new() -> Pads {
            Pads { state: [Ok(()); 4] }
        }

        /// Les boutons de toutes les manettes branchées.
        pub fn poll(&mut self) -> Buttons {
            let mut pad = Buttons::default();
            for (i, state) in self.state.iter_mut().enumerate() {
                if let Err(wait) = state {
                    if *wait > 0 {
                        *wait -= 1;
                        continue;
                    }
                }
                let mut s = XInputState::default();
                // SAFETY : `s` a la forme de XINPUT_STATE.
                if unsafe { XInputGetState(i as u32, &mut s) } != 0 {
                    *state = Err(RETRY);
                    continue;
                }
                *state = Ok(());
                let (b, [lx, ly, ..]) = (s.buttons, s.thumbs);
                pad.up |= b & 0x0001 != 0 || ly > STICK;
                pad.down |= b & 0x0002 != 0 || ly < -STICK;
                pad.left |= b & 0x0004 != 0 || lx < -STICK;
                pad.right |= b & 0x0008 != 0 || lx > STICK;
                pad.start |= b & 0x0010 != 0;
                pad.select |= b & 0x0020 != 0;
                pad.a |= b & 0x1000 != 0;
                pad.b |= b & 0x2000 != 0;
            }
            pad
        }
    }
}
