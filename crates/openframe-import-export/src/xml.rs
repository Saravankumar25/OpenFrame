//! Minimal, bounded XML tree built on quick-xml for FDX and DOCX parsing.
//!
//! Safety: quick-xml never expands DTD/external entities (no XXE, no
//! "billion laughs"); unknown entity references are kept literally. Depth and
//! node counts are capped so hostile documents cannot exhaust memory.

use std::fmt::Write as _;

use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

pub const MAX_DEPTH: usize = 256;
pub const MAX_NODES: usize = 2_000_000;

#[derive(Debug, Clone, Default)]
pub struct Node {
    /// Local name (namespace prefix removed), e.g. `p` for `w:p`.
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Child>,
}

#[derive(Debug, Clone)]
pub enum Child {
    Node(Node),
    Text(String),
}

#[derive(Debug)]
pub enum XmlError {
    Malformed(String),
    TooLarge,
}

impl Node {
    pub fn attr(&self, local: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == local)
            .map(|(_, v)| v.as_str())
    }
    pub fn elements(&self) -> impl Iterator<Item = &Node> {
        self.children.iter().filter_map(|c| match c {
            Child::Node(n) => Some(n),
            Child::Text(_) => None,
        })
    }
    pub fn child(&self, name: &str) -> Option<&Node> {
        self.elements().find(|n| n.name == name)
    }
    pub fn children_named<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Node> + 'a {
        self.elements().filter(move |n| n.name == name)
    }
    /// Concatenated text of this node and all descendants.
    pub fn text(&self) -> String {
        let mut s = String::new();
        self.collect_text(&mut s);
        s
    }
    fn collect_text(&self, s: &mut String) {
        for c in &self.children {
            match c {
                Child::Text(t) => s.push_str(t),
                Child::Node(n) => n.collect_text(s),
            }
        }
    }
    /// Depth-first search for descendants with the given local name (not descending into matches).
    pub fn find_all<'a>(&'a self, name: &str, out: &mut Vec<&'a Node>) {
        for n in self.elements() {
            if n.name == name {
                out.push(n);
            } else {
                n.find_all(name, out);
            }
        }
    }
}

fn local(name: &[u8]) -> String {
    let s = String::from_utf8_lossy(name);
    match s.rfind(':') {
        Some(i) => s[i + 1..].to_string(),
        None => s.into_owned(),
    }
}

fn start_node(e: &BytesStart<'_>) -> Result<Node, XmlError> {
    let mut attrs = Vec::new();
    for a in e.attributes() {
        let a = a.map_err(|err| XmlError::Malformed(err.to_string()))?;
        let key = local(a.key.as_ref());
        let value = a
            .unescape_value()
            .map(|v| v.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&a.value).into_owned());
        attrs.push((key, value));
    }
    Ok(Node {
        name: local(e.name().as_ref()),
        attrs,
        children: Vec::new(),
    })
}

fn entity(name: &str) -> Option<&'static str> {
    Some(match name {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        _ => return None,
    })
}

/// Parse a document and return its root element.
pub fn parse(bytes: &[u8]) -> Result<Node, XmlError> {
    let text = std::str::from_utf8(bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes))
        .map(|s| s.to_string())
        .unwrap_or_else(|_| {
            bytes
                .iter()
                .map(|&b| crate::winansi::decode_byte(b))
                .collect()
        });
    let mut reader = Reader::from_str(&text);
    reader.config_mut().trim_text(false);
    let mut stack: Vec<Node> = vec![Node {
        name: "#document".into(),
        ..Default::default()
    }];
    let mut count = 0usize;
    loop {
        let ev = reader
            .read_event()
            .map_err(|e| XmlError::Malformed(format!("{e} at {}", reader.error_position())))?;
        count += 1;
        if count > MAX_NODES {
            return Err(XmlError::TooLarge);
        }
        match ev {
            Event::Start(e) => {
                if stack.len() > MAX_DEPTH {
                    return Err(XmlError::TooLarge);
                }
                stack.push(start_node(&e)?);
            }
            Event::Empty(e) => {
                let n = start_node(&e)?;
                stack.last_mut().unwrap().children.push(Child::Node(n));
            }
            Event::End(_) => {
                if stack.len() <= 1 {
                    return Err(XmlError::Malformed("unbalanced end tag".into()));
                }
                let n = stack.pop().unwrap();
                stack.last_mut().unwrap().children.push(Child::Node(n));
            }
            Event::Text(t) => {
                let s = t
                    .xml_content()
                    .map_err(|e| XmlError::Malformed(e.to_string()))?;
                push_text(stack.last_mut().unwrap(), &s);
            }
            Event::CData(t) => {
                let s = String::from_utf8_lossy(&t).into_owned();
                push_text(stack.last_mut().unwrap(), &s);
            }
            Event::GeneralRef(r) => {
                let resolved = match r.resolve_char_ref() {
                    Ok(Some(c)) => c.to_string(),
                    _ => {
                        let name = r.decode().map(|c| c.into_owned()).unwrap_or_default();
                        entity(&name)
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| format!("&{name};"))
                    }
                };
                push_text(stack.last_mut().unwrap(), &resolved);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err(XmlError::Malformed(
            "document ended before all elements were closed".into(),
        ));
    }
    let doc = stack.pop().unwrap();
    doc.children
        .into_iter()
        .find_map(|c| match c {
            Child::Node(n) => Some(n),
            Child::Text(_) => None,
        })
        .ok_or_else(|| XmlError::Malformed("no root element".into()))
}

fn push_text(node: &mut Node, s: &str) {
    if let Some(Child::Text(t)) = node.children.last_mut() {
        t.push_str(s);
    } else {
        node.children.push(Child::Text(s.to_string()));
    }
}

/// Escape text for XML element content / attribute values. Characters that
/// are not allowed in XML 1.0 are dropped.
pub fn escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            '\t' | '\n' | '\r' => out.push(c),
            c if (c as u32) < 0x20 || c == '\u{FFFE}' || c == '\u{FFFF}' => {}
            c => out.push(c),
        }
    }
    out
}

/// Tiny XML writer helper.
pub struct XmlWriter {
    pub out: String,
}

impl XmlWriter {
    pub fn new() -> Self {
        XmlWriter {
            out: String::from("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n"),
        }
    }
    pub fn open(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.tag(name, attrs, false);
    }
    pub fn empty(&mut self, name: &str, attrs: &[(&str, &str)]) {
        self.tag(name, attrs, true);
    }
    fn tag(&mut self, name: &str, attrs: &[(&str, &str)], empty: bool) {
        self.out.push('<');
        self.out.push_str(name);
        for (k, v) in attrs {
            let _ = write!(self.out, " {k}=\"{}\"", escape(v));
        }
        self.out.push_str(if empty { "/>" } else { ">" });
    }
    pub fn close(&mut self, name: &str) {
        let _ = write!(self.out, "</{name}>");
    }
    pub fn text(&mut self, t: &str) {
        self.out.push_str(&escape(t));
    }
    pub fn raw(&mut self, t: &str) {
        self.out.push_str(t);
    }
}

impl Default for XmlWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_entities_and_namespaces() {
        let root = parse(br#"<?xml version="1.0"?><w:doc xmlns:w="x"><w:p w:val="a&amp;b">Tom &amp; Jerry &#233; &unknown;</w:p></w:doc>"#).unwrap();
        assert_eq!(root.name, "doc");
        let p = root.child("p").unwrap();
        assert_eq!(p.attr("val"), Some("a&b"));
        assert_eq!(p.text(), "Tom & Jerry é &unknown;");
    }

    #[test]
    fn rejects_malformed() {
        assert!(parse(b"<a><b></a>").is_err());
        assert!(parse(b"<a>").is_err());
        assert!(parse(b"not xml at all").is_err());
    }

    #[test]
    fn doctype_entities_are_not_expanded() {
        let evil = br#"<?xml version="1.0"?><!DOCTYPE x [<!ENTITY lol "lol"><!ENTITY lol2 "&lol;&lol;&lol;">]><x>&lol2;</x>"#;
        let root = parse(evil).unwrap();
        assert_eq!(root.text(), "&lol2;");
    }
}
