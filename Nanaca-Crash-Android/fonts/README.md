# Polices

Le jeu est écrit avec Cocos2d-x, un moteur multiplateforme (Android et
iOS). Ses textes en police système (CCLabelTTF) donnent un nom de police
dans le code, « Arial » ou « monospace », sans fournir de fichier de
police : c'est le système qui choisit. iOS a une Arial ; Android non, et il
remplace ce nom par sa police par défaut. Sur Android, ces textes étaient
donc en Roboto (Droid Sans Mono pour « monospace »), complétée par Noto
Sans CJK pour le japonais et le chinois. Ce sont ces polices qui sont
fournies ici, embarquées dans l'exécutable :

| Fichier | Origine | Licence |
|---|---|---|
| `Roboto-Regular.ttf` | Roboto 2.138, version « android » ([googlefonts/roboto](https://github.com/googlefonts/roboto/releases/tag/v2.138)) | Apache 2.0, `LICENSE-Roboto.txt` |
| `DroidSansMono.ttf` | Android Open Source Project ([frameworks/base/data/fonts](https://android.googlesource.com/platform/frameworks/base/+/refs/heads/main/data/fonts/)) | Apache 2.0, `LICENSE-DroidSansMono.txt` |
| `NotoSansJP-Regular-Nanaca.otf` | Noto Sans JP 2.004 ([notofonts/noto-cjk](https://github.com/notofonts/noto-cjk/releases/tag/Sans2.004)), réduite aux caractères des textes du jeu | SIL OFL 1.1, `LICENSE-NotoSansCJK.txt` |
| `NotoSansTC-Regular-Nanaca.otf` | Noto Sans TC 2.004 (même origine), réduite de même | SIL OFL 1.1, `LICENSE-NotoSansCJK.txt` |

Les deux Noto sont refaites par `Tools/Android/fonts.py` (fontTools), à
partir des polices d'origine et des textes de l'APK.
