# NANACA†CRASH!! — portages des originaux

*[Read in English](README.md)*

Portages Windows et Linux non officiels des jeux **NANACA†CRASH!!** d'origine,
écrits en Rust à partir de leur code décompilé : la version **Flash 1.10** et
la version **Android 1.2.0**. Ils cherchent à jouer exactement comme les
originaux, bugs compris.

Ce dépôt ne contient **aucun fichier sous droits** : ni ressources du jeu, ni
code d'origine. Chaque portage lit le fichier original que vous fournissez,
posé à côté de son exécutable, et vérifie son CRC32 avant de démarrer. Le
fichier peut aussi être ailleurs : indiquez son chemin (ou celui du dossier qui
le contient) dans `"assets"` → `"path"` de la sauvegarde ; un chemin relatif
part de l'exécutable. Une fois vérifié, le fichier est reconnu à sa taille et à
sa date (`"assets"` → `"checked"`).

Les sauvegardes vont dans `Documents\My Games` sous Windows (le dossier
Documents tel que Windows le connaît, déplacé ou synchronisé compris) et dans
`~/.local/share` sous Linux. Avec un fichier `portable.txt` à côté de
l'exécutable, elles vont plutôt dans un dossier `saves` à côté de lui.

## Flash 1.10

- Fichier nécessaire : le SWF original de NANACA†CRASH!! 1.10 (CRC32
  `53A65665`), sous n'importe quel nom finissant par `.swf`.
- Posez-le à côté de `nanaca_crash_flash.exe` (`nanaca_crash_flash` sous
  Linux) et lancez l'exécutable.
- Se joue à la souris, comme l'original (scène de 700 × 400, 30 images par
  seconde), ou à la manette : la croix ou le stick gauche choisit
  un bouton, A clique (sans bouton choisi : un clic pour la jauge de
  lancement et les attaques), B ferme HOW TO PLAY, Start met en pause,
  Start + Select quitte.
- Alt+Entrée ou F11 : plein écran sans bordure.
- Sauvegarde : `Documents\My Games\Nanaca Crash\flash_save.json` sous Windows,
  `~/.local/share/Nanaca Crash/flash_save.json` sous Linux.

## Android 1.2.0

- Fichier nécessaire : l'APK de NANACA†CRASH!! 1.2.0 (CRC32 `BCBAB1C3`), sous
  n'importe quel nom finissant par `.apk`.
- Posez-le à côté de `nanaca_crash_android.exe` (`nanaca_crash_android` sous
  Linux) et lancez l'exécutable.
- Se joue à la souris au lieu du tactile, au clavier ou à la manette : la
  croix, le stick ou les flèches choisissent un bouton, A ou Entrée le
  touche (sans bouton choisi : un toucher pour la jauge et les attaques),
  B ou Échap font la touche retour d'Android, Start met en pause,
  Start + Select quitte.
- Alt+Entrée ou F11 : plein écran sans bordure.
- Les achats intégrés sont débloqués ; les publicités, Kamcord, Twitter, la
  boutique et Google Play sont retirés. Les succès sont gardés dans la
  sauvegarde, comme avec un compte connecté.
- Sauvegarde : `Documents\My Games\Nanaca Crash\android_save.json` sous
  Windows, `~/.local/share/Nanaca Crash/android_save.json` sous Linux. Les fonds
  d'écran de la galerie s'enregistrent dans `Images\Nanaca Crash Android`.

### Bugs du jeu volontairement non corrigés

Pour rester fidèle à l'original, ces bugs sont gardés :

- Sur un écran plus étroit que 16:9 (en portrait), Nanaca dépasse Taichi
  quand elle le percute : sa place au départ et à l'aérien est fixe (environ
  240 points depuis le bord gauche), alors que Taichi est au milieu de
  l'écran. En 16:9 et plus large, elle le touche normalement.
- La couleur de certaines polices s'adapte au niveau, d'autres non.
- À la fin de la frappe de départ, Taichi se téléporte d'environ 26 points
  vers le haut : la frappe le pose à la moitié de sa hauteur, le vol part de
  sa hauteur de lancement (0,75 m).
- Décors : des éléments défilent moins vite que ce qu'ils cachent, comme
  s'ils étaient plus loin tout en étant devant. L'île de la plage passe
  devant la mer en défilant six fois moins vite qu'elle ; au chemin de
  l'école, le sanctuaire passe devant la forêt la plus lointaine en étant
  plus lent qu'elle, et les passants devant la forêt proche en étant plus
  lents qu'elle.
- Décors : des rangées perdent leur dernière image, qui reste immobile
  (8e « shorefront » en été, 9e « n_woods0 » et 8e « n_woods1 » aux feux
  d'artifice, 6e montagne lointaine de nuit).
- Animations des personnages (Cocos2d-x) : à la dernière image d'un
  mouvement, un os qui doit finir invisible garde un pas de plus la
  transparence de l'image d'avant (par exemple le « BOOST » de Kiri, encore à
  17 % d'opacité au lieu de 0) : sur un segment d'images-clés sans couleur,
  Cocos ne remet pas la couleur à jour.
- Personnages dépassés : la distance où ils quittent la course (et
  reviennent dans la file) dépend de la largeur de l'écran, car elle est
  calculée sur leur place à l'écran.
- Succès de vitesse (100 et 300 km/h) : la vitesse n'est regardée qu'une
  image sur cinq, et pas pendant les spéciaux ni le vol plané (vitesse
  masquée) : un pic de vitesse entre deux peut ne pas compter.
- Palmiers de la plage : leur décalage en hauteur vient de leur numéro dans
  le code (3, −4, 4, −4, 4, −5, 5…, de plus en plus écartés le long de la
  rangée) ; la rangée rebouclant tous les quatre palmiers, un palmier saute
  d'un point à chaque boucle.
- Départ et spécial de Misato : le flash jaune attend 0,66 s et clignote
  deux fois (tableau d'actions réutilisé) au lieu de partir avec les rayons.
- Combos : chaque combo crée un texte « COMBO » de plus, jamais affiché ni
  supprimé.
- Quand un personnage attrape Taichi, Taichi repart à plat (rotation lue sur
  la mauvaise couche) au lieu de repartir de la rotation où il était tenu.
- Après les spéciaux de Kiri et de Miki, le compteur de vitesse reste masqué
  (« ---.-- ») jusqu'au spécial suivant de Touko ou de Misato.
- Champ de force DOUBLE après une relance de CHARGE : l'éclat vert de DOUBLE
  reste 10 points plus bas, à la place que lui a donnée l'éclat orange de la
  relance (la même image sert aux deux, et sa place n'est jamais remise).
- Coup simple de Miki : elle frappe avec l'élan qu'elle a en mémoire
  (11 m/s à 70° au départ), que la fin d'un COMBO qu'elle conclut remplace
  par celui de la relance du COMBO (24 m/s à 45°), pour toute la suite de la
  course : ses coups simples deviennent deux fois plus forts et plus bas,
  sans que rien ne l'indique. Kiri, dont le code est le même, remet le sien
  (10 m/s à 45°) avant chaque coup ; la ligne manque chez Miki.

## Vérifier son fichier

Le CRC32 indique si un fichier est la bonne version. Vérifiez-le avec 7-Zip
(clic droit > CRC SHA > CRC-32) ou avec Python :

```
python Tools/crc32.py <fichier>
```

## Compilation

Prérequis : [Rust](https://rustup.rs) (stable).

### Windows

```
build.bat
```

compile les portages en release et écrit les archives prêtes à partager (les
exécutables seuls, sans fichier du jeu) :
`target\Nanaca.Crash.Flash.Windows.zip`,
`target\Nanaca.Crash.Android.Windows.zip`, plus
`target\Nanaca.Crash.All.Windows.zip` qui réunit les deux exécutables.

### Linux

Il faut aussi `pkg-config` et les en-têtes ALSA
(`sudo apt install pkg-config libasound2-dev` sous Ubuntu). Sous Linux :

```
./build.sh
```

Depuis Windows, `build_linux.bat` fait la même chose dans WSL (Ubuntu).
Les archives sont écrites dans `target/` :
`Nanaca.Crash.Flash.Linux.tar.gz`, `Nanaca.Crash.Android.Linux.tar.gz`,
plus `Nanaca.Crash.All.Linux.tar.gz` qui réunit les deux.

## Organisation

| Dossier | Contenu |
|---|---|
| `Nanaca-Crash-Flash/` | portage de la version Flash 1.10 |
| `Nanaca-Crash-Android/` | portage de la version Android 1.2.0 |
| `Tools/` | outils d'étude des originaux (vérification, décompilation) |

## Crédits

NANACA†CRASH!! est un jeu dérivé de *CROSS†CHANNEL* (FlyingShine / WillPlus,
écrit par Romeo Tanaka). Conception et version Flash d'origine par Takahiro
Narumiya ; version iOS/Android par WillPlus. Tous les droits sur les jeux,
leurs personnages et leurs ressources appartiennent à leurs propriétaires.
Ces portages sont non officiels et sans lien avec eux.

## Licence

Copyright (C) 2026 Nyaldee. Distribué sous licence [GNU General Public License v3.0](LICENSE) — voir le fichier `LICENSE` pour le texte complet.
