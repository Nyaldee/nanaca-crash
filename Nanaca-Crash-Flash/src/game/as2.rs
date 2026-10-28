//! Ce que la logique reprend du comportement d'ActionScript 2 : nombres en
//! texte, arrondis, textes en nombres, tirage au hasard.

/// Un nombre en texte, comme ActionScript (15 chiffres significatifs).
pub fn number_to_string(n: f64) -> String {
    if n.is_nan() {
        return "NaN".into();
    }
    if n.is_infinite() {
        return if n > 0.0 { "Infinity".into() } else { "-Infinity".into() };
    }
    if n == 0.0 {
        return "0".into();
    }
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    // 15 chiffres significatifs, sans zéros inutiles.
    let s = format!("{n:.14e}");
    let (mantissa, exponent) = s.split_once('e').unwrap_or((&s, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let digits = digits.trim_end_matches('0');
    let mut out = String::new();
    if negative {
        out.push('-');
    }
    if !(-5..15).contains(&exponent) {
        out.push_str(&digits[..1]);
        if digits.len() > 1 {
            out.push('.');
            out.push_str(&digits[1..]);
        }
        out.push_str(&format!("e{}{}", if exponent > 0 { "+" } else { "-" }, exponent.abs()));
    } else if exponent < 0 {
        out.push_str("0.");
        out.push_str(&"0".repeat((-exponent - 1) as usize));
        out.push_str(digits);
    } else {
        let point = exponent as usize + 1;
        if digits.len() <= point {
            out.push_str(digits);
            out.push_str(&"0".repeat(point - digits.len()));
        } else {
            out.push_str(&digits[..point]);
            out.push('.');
            out.push_str(&digits[point..]);
        }
    }
    out
}

/// Un texte en nombre (Number(texte)) : NaN s'il n'en est pas un, ou vide.
pub fn string_to_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return f64::NAN;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return i64::from_str_radix(hex, 16).map_or(f64::NAN, |v| v as f64);
    }
    t.parse().unwrap_or(f64::NAN)
}

/// Math.round : au plus proche, les moitiés vers le haut.
pub fn round(x: f64) -> f64 {
    (x + 0.5).floor()
}

/// Les mètres affichés par le jeu : partie entière, point, deux chiffres
/// (`Math.floor(x) + "." + Math.floor((Math.abs(x) + 1) * 100).toString().substr(-2)`).
pub fn meters(x: f64) -> String {
    let cents = number_to_string(((x.abs() + 1.0) * 100.0).floor());
    let tail: String = cents.chars().rev().take(2).collect::<Vec<_>>().into_iter().rev().collect();
    format!("{}.{tail}", number_to_string(x.floor()))
}

/// Le caractère d'un texte à une position (charAt) : "" hors du texte.
pub fn char_at(s: &str, i: i64) -> String {
    if i < 0 {
        return String::new();
    }
    s.chars().nth(i as usize).map(String::from).unwrap_or_default()
}

/// Math.random : un générateur xorshift, amorcé par l'heure.
pub struct Random(u64);

impl Random {
    pub fn new() -> Random {
        let seed = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0x9E37_79B9_7F4A_7C15, |d| d.as_nanos() as u64);
        Random(seed | 1)
    }

    /// Un nombre dans [0, 1).
    pub fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers() {
        assert_eq!(number_to_string(100.0), "100");
        assert_eq!(number_to_string(-0.0), "0");
        assert_eq!(number_to_string(0.1 + 0.2), "0.3");
        assert_eq!(number_to_string(12.5), "12.5");
        assert_eq!(number_to_string(1e21), "1e+21");
        assert_eq!(number_to_string(0.0001), "0.0001");
        assert_eq!(number_to_string(0.000001), "1e-6");
        assert!(string_to_number("").is_nan());
        assert_eq!(string_to_number("7"), 7.0);
    }

    #[test]
    fn meter_text() {
        assert_eq!(meters(0.0), "0.00");
        assert_eq!(meters(12.345), "12.34");
        assert_eq!(meters(-1.5), "-2.50");
    }
}
