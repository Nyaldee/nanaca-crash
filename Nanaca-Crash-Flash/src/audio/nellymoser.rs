//! Décodeur Nellymoser Asao, le codec des voix de Flash.
//!
//! Chaque bloc de 64 octets donne 256 échantillons, en deux moitiés de 128.
//! Le bloc commence par l'enveloppe du spectre (une valeur par bande de
//! fréquences, codée par différences) ; elle sert à la fois à répartir les
//! 198 bits de chaque moitié entre les 124 coefficients et à les mettre à
//! l'échelle. Chaque moitié est ensuite ramenée au temps par une transformée
//! en cosinus inverse (IMDCT) de 128 points, fenêtrée et recouverte avec la
//! précédente.
//!
//! Les tables sont celles du format.

use std::sync::OnceLock;

/// Octets par bloc.
const BLOCK: usize = 64;
/// Coefficients par moitié de bloc, dont seuls les 124 premiers sont codés.
const HALF: usize = 128;
const CODED: usize = 124;
/// Bits de l'en-tête (l'enveloppe), puis bits de chaque moitié.
const HEADER_BITS: usize = 116;
const DETAIL_BITS: i32 = 198;
/// Bits au plus par coefficient.
const MAX_BITS: i32 = 6;

/// Taille de chacune des 23 bandes de fréquences.
const BANDS: [usize; 23] = [2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3, 4, 4, 5, 6, 6, 7, 8, 9, 10, 12, 14, 15];

/// Valeur de la première bande (6 bits).
const FIRST: [u16; 64] = [
    3134, 5342, 6870, 7792, 8569, 9185, 9744, 10191, 10631, 11061, 11434, 11770, 12116, 12513, 12925, 13300, 13674, 14027, 14352, 14716, 15117, 15477,
    15824, 16157, 16513, 16804, 17090, 17401, 17679, 17948, 18238, 18520, 18764, 19078, 19381, 19640, 19921, 20205, 20500, 20813, 21162, 21465, 21794,
    22137, 22453, 22756, 23067, 23350, 23636, 23926, 24227, 24521, 24819, 25107, 25414, 25730, 26120, 26497, 26895, 27344, 27877, 28463, 29426, 31355,
];

/// Différence d'une bande à la suivante (5 bits).
const DELTA: [i16; 32] = [
    -11725, -9420, -7910, -6801, -5948, -5233, -4599, -4039, -3507, -3030, -2596, -2170, -1774, -1383, -1016, -660, -329, -1, 337, 696, 1085, 1512, 1962,
    2433, 2968, 3569, 4314, 5279, 6622, 8154, 10076, 12975,
];

/// Valeur d'un coefficient codé sur `n` bits avec le code `v` : à
/// l'indice 2ⁿ − 1 + v.
const LEVELS: [f32; 127] = [
    0.0000000000, -0.8472560048, 0.7224709988, -1.5247479677, -0.4531480074, 0.3753609955, 1.4717899561, -1.9822579622, -1.1929379702, -0.5829370022,
    -0.0693780035, 0.3909569979, 0.9069200158, 1.4862740040, 2.2215409279, -2.3887870312, -1.8067539930, -1.4105420113, -1.0773609877, -0.7995010018,
    -0.5558109879, -0.3334020078, -0.1324490011, 0.0568020009, 0.2548770010, 0.4773550034, 0.7386850119, 1.0443060398, 1.3954459429, 1.8098750114,
    2.3918759823, -2.3893830776, -1.9884680510, -1.7514040470, -1.5643119812, -1.3922129869, -1.2164649963, -1.0469499826, -0.8905100226, -0.7645580173,
    -0.6454579830, -0.5259280205, -0.4059549868, -0.3029719889, -0.2096900046, -0.1239869967, -0.0479229987, 0.0257730000, 0.1001340002, 0.1737180054,
    0.2585540116, 0.3522900045, 0.4569880068, 0.5767750144, 0.7003160119, 0.8425520062, 1.0093879700, 1.1821349859, 1.3534560204, 1.5320819616,
    1.7332619429, 1.9722349644, 2.3978140354, -2.5756309032, -2.0573320389, -1.8984919786, -1.7727810144, -1.6662600040, -1.5742180347, -1.4993319511,
    -1.4316639900, -1.3652280569, -1.3000990152, -1.2280930281, -1.1588579416, -1.0921250582, -1.0135740042, -0.9202849865, -0.8287050128, -0.7374889851,
    -0.6447759867, -0.5590940118, -0.4857139885, -0.4110319912, -0.3459700048, -0.2851159871, -0.2341620028, -0.1870580018, -0.1442500055, -0.1107169986,
    -0.0739680007, -0.0365610011, -0.0073290002, 0.0203610007, 0.0479039997, 0.0751969963, 0.0980999991, 0.1220389977, 0.1458999962, 0.1694349945,
    0.1970459968, 0.2252430022, 0.2556869984, 0.2870100141, 0.3197099864, 0.3525829911, 0.3889069855, 0.4334920049, 0.4769459963, 0.5204820037,
    0.5644530058, 0.6122040153, 0.6685929894, 0.7341650128, 0.8032159805, 0.8784040213, 0.9566209912, 1.0397069454, 1.1293770075, 1.2211159468,
    1.3080279827, 1.4024800062, 1.5056819916, 1.6227730513, 1.7724959850, 1.9430880547, 2.2903931141,
];

/// Lecture de bits, poids faible d'abord.
struct Bits<'a> {
    data: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn read(&mut self, n: usize) -> u32 {
        let mut v = 0;
        for i in 0..n {
            let bit = self.data.get(self.pos / 8).map_or(0, |b| (b >> (self.pos % 8)) & 1);
            v |= u32::from(bit) << i;
            self.pos += 1;
        }
        v
    }
}

/// Décode des blocs Nellymoser en échantillons (-1..1).
pub fn decode(data: &[u8]) -> Vec<f32> {
    let mut out = Vec::with_capacity(data.len() / BLOCK * 2 * HALF);
    let mut overlap = [0f32; HALF];
    let mut noise = 0x2545_F491u32;
    for block in data.chunks_exact(BLOCK) {
        let (levels, scales) = envelope(block);
        let bits = allocate(&levels);
        for half in 0..2 {
            let mut reader = Bits { data: block, pos: HEADER_BITS + half * DETAIL_BITS as usize };
            let mut coefs = [0f32; HALF];
            for j in 0..CODED {
                let value = if bits[j] > 0 {
                    let n = bits[j] as usize;
                    LEVELS[(1 << n) - 1 + reader.read(n) as usize]
                } else {
                    // Coefficient sans bits : un bruit de même énergie, de
                    // signe au hasard.
                    noise ^= noise << 13;
                    noise ^= noise >> 17;
                    noise ^= noise << 5;
                    if noise & 1 == 0 { std::f32::consts::FRAC_1_SQRT_2 } else { -std::f32::consts::FRAC_1_SQRT_2 }
                };
                coefs[j] = value * scales[j];
            }
            let time = imdct(&coefs);
            for k in 0..HALF {
                out.push((overlap[k] + time[k]) / 32768.0);
                overlap[k] = time[HALF + k];
            }
        }
    }
    out
}

/// L'enveloppe du spectre : la valeur de chaque coefficient (échelle
/// logarithmique, 2048 par octave) et son facteur d'échelle.
fn envelope(block: &[u8]) -> ([f32; HALF], [f32; HALF]) {
    let mut reader = Bits { data: block, pos: 0 };
    let mut levels = [0f32; HALF];
    let mut scales = [0f32; HALF];
    let mut value = f32::from(FIRST[reader.read(6) as usize]);
    let mut j = 0;
    for (band, &size) in BANDS.iter().enumerate() {
        if band > 0 {
            value += f32::from(DELTA[reader.read(5) as usize]);
        }
        let scale = (value / 2048.0).exp2();
        for _ in 0..size {
            levels[j] = value;
            scales[j] = scale;
            j += 1;
        }
    }
    (levels, scales)
}

/// Décalage à gauche (ou à droite si négatif).
fn shift(v: i32, n: i32) -> i32 {
    if n > 0 { v << n } else { v >> -n }
}

/// Le décalage qui amène la valeur absolue juste sous le bit de signe ; la
/// valeur est décalée d'autant.
fn normalize(v: &mut i32) -> i32 {
    if *v == 0 {
        return 31;
    }
    let n = v.unsigned_abs().leading_zeros() as i32 - 1;
    *v <<= n;
    n
}

/// Bits de chaque coefficient quand on retire `offset` aux niveaux.
fn bits_for(levels: &[i16; HALF], precision: i32, offset: i32, out: &mut [i32; HALF]) -> i32 {
    let mut total = 0;
    for j in 0..CODED {
        let b = ((i32::from(levels[j]) - offset) >> (precision - 1)) + 1 >> 1;
        out[j] = b.clamp(0, MAX_BITS);
        total += out[j];
    }
    total
}

/// La répartition des 198 bits d'une moitié entre ses coefficients : les
/// plus forts en reçoivent le plus. On cherche, en virgule fixe comme le
/// codeur, le décalage des niveaux qui dépense au plus près les 198 bits.
fn allocate(levels: &[f32; HALF]) -> [i32; HALF] {
    let mut max = levels.iter().fold(0, |m, &v| m.max(v as i32));
    let mut precision = normalize(&mut max) - 16;
    let mut scaled = [0i16; HALF];
    for j in 0..CODED {
        let s = shift(levels[j] as i32, precision) as i16;
        scaled[j] = ((3 * i32::from(s)) >> 2) as i16;
    }
    let mut sum: i32 = scaled.iter().map(|&s| i32::from(s)).sum();
    precision += 11;
    sum -= DETAIL_BITS << precision;
    let n = precision + normalize(&mut sum);
    // Première estimation du décalage.
    let mut offset = shift((4228 * (sum >> 16)) >> 15, precision - (19 + n - 31));
    let mut bits = [0i32; HALF];
    let mut total = bits_for(&scaled, precision, offset, &mut bits);
    if total != DETAIL_BITS {
        // Pas de recherche, puis encadrement…
        let mut step = total - DETAIL_BITS;
        let mut k = 0;
        while step.abs() <= 16383 {
            step *= 2;
            k += 1;
        }
        step = shift((step * 4228) >> 15, precision - (19 + k - 15));
        let (mut last_offset, mut last_total, mut tries) = (offset, total, 0);
        for j in 1..20 {
            last_offset = offset;
            last_total = total;
            tries = j;
            offset += step;
            total = bits_for(&scaled, precision, offset, &mut bits);
            if (total - DETAIL_BITS) * (last_total - DETAIL_BITS) <= 0 {
                break;
            }
        }
        // … puis dichotomie.
        let (mut big, mut big_total, mut small, mut small_total) =
            if total > DETAIL_BITS { (offset, total, last_offset, last_total) } else { (last_offset, last_total, offset, total) };
        while total != DETAIL_BITS && tries <= 19 {
            let middle = (big + small) >> 1;
            total = bits_for(&scaled, precision, middle, &mut bits);
            if total > DETAIL_BITS {
                (big, big_total) = (middle, total);
            } else {
                (small, small_total) = (middle, total);
            }
            tries += 1;
        }
        // Le plus proche des deux bornes.
        (offset, total) = if (big_total - DETAIL_BITS).abs() >= (small_total - DETAIL_BITS).abs() { (small, small_total) } else { (big, big_total) };
    }
    bits_for(&scaled, precision, offset, &mut bits);
    // Trop de bits : les derniers coefficients n'en ont plus.
    if total > DETAIL_BITS {
        let mut spent = 0;
        let mut j = 0;
        while spent < DETAIL_BITS {
            spent += bits[j];
            j += 1;
        }
        bits[j - 1] -= spent - DETAIL_BITS;
        for b in &mut bits[j..CODED] {
            *b = 0;
        }
    }
    bits
}

/// Transformée en cosinus inverse de 128 coefficients vers 256
/// échantillons, fenêtrés (fenêtre sinus, gain 1/8).
fn imdct(coefs: &[f32; HALF]) -> [f32; 2 * HALF] {
    static TABLE: OnceLock<Vec<f32>> = OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let n = HALF as f64;
        let mut t = Vec::with_capacity(2 * HALF * HALF);
        for k in 0..2 * HALF {
            let window = ((k as f64 + 0.5) / (2.0 * n) * std::f64::consts::PI).sin() / 8.0;
            for i in 0..HALF {
                let angle = std::f64::consts::PI / n * (k as f64 + 0.5 + n / 2.0) * (i as f64 + 0.5);
                t.push((angle.cos() * window) as f32);
            }
        }
        t
    });
    let mut out = [0f32; 2 * HALF];
    for (k, row) in table.chunks_exact(HALF).enumerate() {
        out[k] = row.iter().zip(coefs).map(|(a, b)| a * b).sum();
    }
    out
}
