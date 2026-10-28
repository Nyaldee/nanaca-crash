# NANACA†CRASH!! — original ports

*[Lire en français](README.fr.md)*

Unofficial Windows and Linux ports of the original **NANACA†CRASH!!** games,
written in Rust from their decompiled code: the **Flash 1.10** version and
the **Android 1.2.0** version. They aim to play exactly like the originals,
bugs included.

This repository contains **no copyrighted file**: no game assets, no original
code. Each port reads the original game file you provide, placed next to its
executable, and checks it by its CRC32 before starting. The file can also live
elsewhere: set its path (or the folder holding it) in `"assets"` → `"path"` of
the save file; a relative path starts from the executable. Once checked, the
file is recognized by its size and date (`"assets"` → `"checked"`).

Saves go to `Documents\My Games` on Windows (the Documents folder as Windows
knows it, moved or synced included) and `~/.local/share` on Linux. With a
`portable.txt` file next to the executable, they go to a `saves` folder next to
it instead.

## Flash 1.10

- Required file: the original SWF of NANACA†CRASH!! 1.10 (CRC32
  `53A65665`), with any name ending in `.swf`.
- Put it next to `nanaca_crash_flash.exe` (`nanaca_crash_flash` on Linux)
  and run the executable.
- Played with the mouse, like the original (700 × 400 stage, 30 fps), or
  with a gamepad: the D-pad or the left stick picks a button, A
  clicks (with no button picked: a click for the launch gauge and the
  attacks), B closes HOW TO PLAY, Start pauses, Start + Select quits.
- Alt+Enter or F11: borderless fullscreen.
- Save file: `Documents\My Games\Nanaca Crash\flash_save.json` on Windows,
  `~/.local/share/Nanaca Crash/flash_save.json` on Linux.

## Android 1.2.0

- Required file: the APK of NANACA†CRASH!! 1.2.0 (CRC32 `BCBAB1C3`), with
  any name ending in `.apk`.
- Put it next to `nanaca_crash_android.exe` (`nanaca_crash_android` on
  Linux) and run the executable.
- Played with the mouse instead of touch, with the keyboard or with a
  gamepad: the D-pad, the stick or the arrow keys pick a button, A or Enter
  touches it (with no button picked: a touch for the gauge and the attacks),
  B or Escape act as Android's back key, Start pauses, Start + Select quits.
- Alt+Enter or F11: borderless fullscreen.
- In-app purchases are unlocked; advertisements, Kamcord, Twitter, the store
  and Google Play are removed. Achievements are kept in the save file, as
  with a signed-in account.
- Save file: `Documents\My Games\Nanaca Crash\android_save.json` on Windows,
  `~/.local/share/Nanaca Crash/android_save.json` on Linux. Gallery
  wallpapers are saved to `Pictures\Nanaca Crash Android`.

### Game bugs deliberately left unfixed

To stay faithful to the original, these bugs are kept:

- On a screen narrower than 16:9 (portrait), Nanaca overshoots Taichi when
  she crashes into him: her place at the start and in aerial attacks is
  fixed (about 240 points from the left edge), while Taichi is in the middle
  of the screen. At 16:9 and wider, she hits him normally.
- The color of some fonts follows the stage, others don't.
- At the end of the opening crash, Taichi teleports about 26 points up: the
  crash places him at half his height, the flight starts from his launch
  height (0.75 m).
- Scenery: some elements scroll slower than what they hide, as if they were
  farther away while being in front. The beach island passes in front of
  the sea while scrolling six times slower than it; on the school path, the
  shrine passes in front of the farthest forest while being slower than it,
  and the passers-by in front of the near forest while being slower than it.
- Scenery: some rows lose their last image, which stays still (8th
  "shorefront" in summer, 9th "n_woods0" and 8th "n_woods1" in the
  fireworks stage, 6th far mountain at night).
- Character animations (Cocos2d-x): on the last frame of a movement, a bone
  that should end invisible keeps the transparency of the previous frame
  for one more step (for example Kiri's "BOOST", still at 17 % opacity
  instead of 0): on a keyframe segment with no color, Cocos does not update
  the color.
- Passed characters: the distance at which they leave the race (and go back
  in the queue) depends on the screen width, as it is computed from their
  place on screen.
- Speed achievements (100 and 300 km/h): the speed is only checked one frame
  out of five, and not during specials or gliding (speed hidden): a speed
  peak in between may not count.
- Beach palm trees: their height offset comes from their number in the code
  (3, −4, 4, −4, 4, −5, 5…, spreading wider along the row); as the row loops
  every four palm trees, a palm tree jumps by one point on each loop.
- Start and Misato's special: the yellow flash waits 0.66 s and blinks
  twice (reused action array) instead of starting with the rays.
- Combos: each combo creates one more "COMBO" text, never shown nor removed.
- When a character catches Taichi, Taichi leaves flat (rotation read from
  the wrong layer) instead of leaving from the rotation he was held at.
- After Kiri's and Miki's specials, the speed counter stays hidden
  ("---.--") until the next special of Touko or Misato.
- DOUBLE force field after a CHARGE relaunch: DOUBLE's green flash stays 10
  points lower, at the place the relaunch's orange flash gave it (the same
  image is used for both, and its place is never reset).
- Miki's simple hit: she hits with the momentum she has in memory (11 m/s
  at 70° at first), which the end of a COMBO she concludes replaces with the
  COMBO relaunch's (24 m/s at 45°) for the rest of the race: her simple hits
  become twice as strong and lower, with nothing showing it. Kiri, whose
  code is the same, resets hers (10 m/s at 45°) before each hit; the line is
  missing for Miki.

## Checking your file

The CRC32 tells whether a file is the right version. Check it with 7-Zip
(right click > CRC SHA > CRC-32) or with Python:

```
python Tools/crc32.py <file>
```

## Building

Requirements: [Rust](https://rustup.rs) (stable).

### Windows

```
build.bat
```

builds the ports in release and writes the ready-to-share archives
(executables only, no game file): `target\Nanaca.Crash.Flash.Windows.zip`,
`target\Nanaca.Crash.Android.Windows.zip`, plus
`target\Nanaca.Crash.All.Windows.zip` with both executables.

### Linux

`pkg-config` and the ALSA headers are also needed
(`sudo apt install pkg-config libasound2-dev` on Ubuntu). On Linux:

```
./build.sh
```

From Windows, `build_linux.bat` does the same in WSL (Ubuntu). The archives
are written to `target/`: `Nanaca.Crash.Flash.Linux.tar.gz`,
`Nanaca.Crash.Android.Linux.tar.gz`, plus `Nanaca.Crash.All.Linux.tar.gz`
with both.

## Layout

| Folder | Contents |
|---|---|
| `Nanaca-Crash-Flash/` | port of the Flash 1.10 version |
| `Nanaca-Crash-Android/` | port of the Android 1.2.0 version |
| `Tools/` | tools used to study the originals (checks, decompilation) |

## Credits

NANACA†CRASH!! is a fan game of *CROSS†CHANNEL* (FlyingShine / WillPlus,
written by Romeo Tanaka). Game design and original Flash version by Takahiro
Narumiya; iOS/Android version by WillPlus. All rights to the games, their
characters and assets belong to their respective owners. These ports are
unofficial and not affiliated with them.

## License

Copyright (C) 2026 Nyaldee. Licensed under the [GNU General Public License v3.0](LICENSE) — see the `LICENSE` file for the full text.
