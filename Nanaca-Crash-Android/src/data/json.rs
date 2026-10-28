//! Un lecteur JSON minimal (armatures CocoStudio).

use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub enum Json {
    #[default]
    Null,
    Bool(bool),
    Number(f64),
    Str(String),
    Array(Vec<Json>),
    Object(HashMap<String, Json>),
}

impl Json {
    /// Un champ d'objet (Null s'il manque).
    pub fn get(&self, key: &str) -> &Json {
        const NULL: Json = Json::Null;
        match self {
            Json::Object(o) => o.get(key).unwrap_or(&NULL),
            _ => &NULL,
        }
    }

    pub fn f32(&self, key: &str, default: f32) -> f32 {
        match self.get(key) {
            Json::Number(n) => *n as f32,
            _ => default,
        }
    }

    pub fn i32(&self, key: &str, default: i32) -> i32 {
        match self.get(key) {
            Json::Number(n) => *n as i32,
            _ => default,
        }
    }

    pub fn bool(&self, key: &str) -> bool {
        matches!(self.get(key), Json::Bool(true))
    }

    pub fn str(&self, key: &str) -> &str {
        match self.get(key) {
            Json::Str(s) => s,
            _ => "",
        }
    }

    pub fn items(&self, key: &str) -> &[Json] {
        match self.get(key) {
            Json::Array(a) => a,
            _ => &[],
        }
    }
}

pub fn parse(text: &str) -> Option<Json> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    p.value()
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eat(&mut self, word: &[u8]) -> bool {
        let ok = self.s[self.i..].starts_with(word);
        if ok {
            self.i += word.len();
        }
        ok
    }

    fn value(&mut self) -> Option<Json> {
        self.space();
        Some(match *self.s.get(self.i)? {
            b'{' => {
                self.i += 1;
                let mut o = HashMap::new();
                loop {
                    self.space();
                    if self.eat(b"}") {
                        break;
                    }
                    let key = self.string()?;
                    self.space();
                    if !self.eat(b":") {
                        return None;
                    }
                    o.insert(key, self.value()?);
                    self.space();
                    self.eat(b",");
                }
                Json::Object(o)
            }
            b'[' => {
                self.i += 1;
                let mut a = Vec::new();
                loop {
                    self.space();
                    if self.eat(b"]") {
                        break;
                    }
                    a.push(self.value()?);
                    self.space();
                    self.eat(b",");
                }
                Json::Array(a)
            }
            b'"' => Json::Str(self.string()?),
            b't' if self.eat(b"true") => Json::Bool(true),
            b'f' if self.eat(b"false") => Json::Bool(false),
            b'n' if self.eat(b"null") => Json::Null,
            _ => {
                let start = self.i;
                while self.i < self.s.len() && matches!(self.s[self.i], b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9') {
                    self.i += 1;
                }
                Json::Number(std::str::from_utf8(&self.s[start..self.i]).ok()?.parse().ok()?)
            }
        })
    }

    fn string(&mut self) -> Option<String> {
        if !self.eat(b"\"") {
            return None;
        }
        let mut out = Vec::new();
        loop {
            let c = *self.s.get(self.i)?;
            self.i += 1;
            match c {
                b'"' => break,
                b'\\' => {
                    let e = *self.s.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push(b'\n'),
                        b't' => out.push(b'\t'),
                        b'r' => out.push(b'\r'),
                        b'b' => out.push(8),
                        b'f' => out.push(12),
                        b'u' => {
                            let hex = std::str::from_utf8(self.s.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let c = char::from_u32(u32::from_str_radix(hex, 16).ok()?).unwrap_or('\u{FFFD}');
                            out.extend_from_slice(c.encode_utf8(&mut [0; 4]).as_bytes());
                        }
                        other => out.push(other),
                    }
                }
                other => out.push(other),
            }
        }
        Some(String::from_utf8_lossy(&out).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::Json;

    #[test]
    fn parse() {
        let j = super::parse(r#"{"a": [1, -2.5e1, true], "b": {"c": "d\"é"}, "n": null}"#).unwrap();
        assert!(matches!(j.items("a")[1], Json::Number(n) if n == -25.0));
        assert_eq!(j.get("b").str("c"), "d\"é");
        assert!(matches!(j.get("n"), Json::Null));
    }
}
