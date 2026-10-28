//! Un lecteur XML minimal : éléments, attributs et texte, assez pour les
//! plist et les fichiers de textes, d'apparences et de galerie du jeu.

#[derive(Debug, Default)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    /// Le texte direct de l'élément, entités remplacées.
    pub text: String,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs.iter().find(|a| a.0 == name).map(|a| a.1.as_str())
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|c| c.name == name)
    }

    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.children.iter().filter(move |c| c.name == name)
    }
}

/// L'élément racine du document.
pub fn parse(text: &str) -> Option<Element> {
    let mut p = Parser { s: text.as_bytes(), i: 0 };
    loop {
        p.skip_until(b'<')?;
        match p.s.get(p.i + 1)? {
            // Déclaration, commentaire, DOCTYPE : passés.
            b'?' | b'!' => p.skip_markup()?,
            _ => return p.element(),
        }
    }
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn skip_until(&mut self, c: u8) -> Option<()> {
        self.i += self.s.get(self.i..)?.iter().position(|&b| b == c)?;
        Some(())
    }

    /// <?…?>, <!--…-->, <!…> (sans crochets imbriqués).
    fn skip_markup(&mut self) -> Option<()> {
        let rest = self.s.get(self.i..)?;
        let end = if rest.starts_with(b"<!--") { find(rest, b"-->")? + 3 } else { rest.iter().position(|&b| b == b'>')? + 1 };
        self.i += end;
        Some(())
    }

    fn name(&mut self) -> String {
        let start = self.i;
        while self.i < self.s.len() && !matches!(self.s[self.i], b' ' | b'\t' | b'\r' | b'\n' | b'>' | b'/' | b'=') {
            self.i += 1;
        }
        String::from_utf8_lossy(&self.s[start..self.i]).into_owned()
    }

    fn skip_space(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    /// Un élément, `i` sur son « < ».
    fn element(&mut self) -> Option<Element> {
        self.i += 1;
        let mut e = Element { name: self.name(), ..Element::default() };
        loop {
            self.skip_space();
            match *self.s.get(self.i)? {
                b'/' => {
                    self.skip_until(b'>')?;
                    self.i += 1;
                    return Some(e);
                }
                b'>' => {
                    self.i += 1;
                    break;
                }
                _ => {
                    let key = self.name();
                    self.skip_space();
                    if self.s.get(self.i) == Some(&b'=') {
                        self.i += 1;
                        self.skip_space();
                        let quote = *self.s.get(self.i)?;
                        self.i += 1;
                        let start = self.i;
                        self.skip_until(quote)?;
                        let value = unescape(&String::from_utf8_lossy(&self.s[start..self.i]));
                        self.i += 1;
                        e.attrs.push((key, value));
                    }
                }
            }
        }
        // Le contenu, jusqu'à la balise fermante.
        loop {
            let start = self.i;
            self.skip_until(b'<')?;
            e.text.push_str(&unescape(&String::from_utf8_lossy(&self.s[start..self.i])));
            match self.s.get(self.i + 1)? {
                b'/' => {
                    self.skip_until(b'>')?;
                    self.i += 1;
                    return Some(e);
                }
                b'!' if self.s[self.i..].starts_with(b"<![CDATA[") => {
                    let rest = &self.s[self.i + 9..];
                    let end = find(rest, b"]]>")?;
                    e.text.push_str(&String::from_utf8_lossy(&rest[..end]));
                    self.i += 9 + end + 3;
                }
                b'?' | b'!' => self.skip_markup()?,
                _ => {
                    let child = self.element()?;
                    e.children.push(child);
                }
            }
        }
    }
}

fn find(s: &[u8], what: &[u8]) -> Option<usize> {
    s.windows(what.len()).position(|w| w == what)
}

/// Les entités XML (nommées et numériques).
fn unescape(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(p) = rest.find('&') {
        out.push_str(&rest[..p]);
        rest = &rest[p..];
        let Some(end) = rest.find(';') else { break };
        let entity = &rest[1..end];
        let c = match entity {
            "lt" => Some('<'),
            "gt" => Some('>'),
            "amp" => Some('&'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ => entity
                .strip_prefix("#x")
                .and_then(|h| u32::from_str_radix(h, 16).ok())
                .or_else(|| entity.strip_prefix('#').and_then(|d| d.parse().ok()))
                .and_then(char::from_u32),
        };
        match c {
            Some(c) => {
                out.push(c);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn parse() {
        let doc = super::parse("<?xml version=\"1.0\"?><!-- c --><a x='1'><b>t&amp;u</b><c/><b>v</b></a>").unwrap();
        assert_eq!(doc.name, "a");
        assert_eq!(doc.attr("x"), Some("1"));
        assert_eq!(doc.children.len(), 3);
        assert_eq!(doc.children[0].text, "t&u");
        assert_eq!(doc.children_named("b").count(), 2);
    }
}
