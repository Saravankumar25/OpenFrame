//! Formatted prose (the Story page): lines of text runs that may be bold, italic, underlined,
//! highlighted or set at their own point size.
//!
//! The plain text stays the source of truth. A formatting is a list of lines, one per line of the
//! text (split at `\n`), and the runs of each line joined together must equal that line exactly.
//! A reader that ignores the formatting (plain text export, search, packages) loses nothing, and
//! a formatting that no longer matches its text is refused by [`check`], never applied to it.
//!
//! Stored as compact JSON: `[[{"t":"Hero ","b":true},{"t":"walks","s":14}],[]]`.

use serde::{Deserialize, Serialize};

/// Smallest and largest point size a run may ask for.
pub const SIZE_MIN: f32 = 6.0;
pub const SIZE_MAX: f32 = 96.0;
/// Size of text that has no size of its own, in points (the editor shows the same size).
pub const BODY_SIZE: f32 = 12.0;

/// A stretch of one line in one style.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProseRun {
    /// The text. Never holds a line break.
    pub t: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub b: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub i: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub u: bool,
    /// Highlighted (a yellow marker behind the text).
    #[serde(default, skip_serializing_if = "is_false")]
    pub h: bool,
    /// Point size; none means [`BODY_SIZE`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub s: Option<f32>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

impl ProseRun {
    pub fn plain(t: impl Into<String>) -> ProseRun {
        ProseRun {
            t: t.into(),
            ..ProseRun::default()
        }
    }
    pub fn size(&self) -> f32 {
        self.s.unwrap_or(BODY_SIZE)
    }
    /// Does the run carry any formatting at all?
    pub fn styled(&self) -> bool {
        self.b || self.i || self.u || self.h || self.s.is_some()
    }
}

/// The runs of one line of text (empty for an empty line).
pub type ProseLine = Vec<ProseRun>;

/// The text as unformatted lines.
pub fn plain_lines(text: &str) -> Vec<ProseLine> {
    text.split('\n')
        .map(|l| {
            if l.is_empty() {
                Vec::new()
            } else {
                vec![ProseRun::plain(l)]
            }
        })
        .collect()
}

/// Does any run of the lines carry formatting?
pub fn any_styled(lines: &[ProseLine]) -> bool {
    lines.iter().flatten().any(ProseRun::styled)
}

/// Check that `lines` formats exactly `text` (line endings already `\n`) with sizes in range.
/// The error names the first problem in plain words.
pub fn check(text: &str, lines: &[ProseLine]) -> Result<(), String> {
    let want: Vec<&str> = text.split('\n').collect();
    if want.len() != lines.len() {
        return Err(format!(
            "the formatting has {} lines and the text has {}",
            lines.len(),
            want.len()
        ));
    }
    for (n, (w, line)) in want.iter().zip(lines).enumerate() {
        let mut got = String::with_capacity(w.len());
        for r in line {
            if r.t.contains('\n') || r.t.contains('\r') {
                return Err(format!("line {} has a run with a line break", n + 1));
            }
            if let Some(s) = r.s
                && !(s.is_finite() && (SIZE_MIN..=SIZE_MAX).contains(&s))
            {
                return Err(format!("line {} asks for a size of {s} points", n + 1));
            }
            got.push_str(&r.t);
        }
        if got != *w {
            return Err(format!("line {} does not match the text", n + 1));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_lines_cover_the_text_and_pass_the_check() {
        let text = "One\n\nTwo three";
        let lines = plain_lines(text);
        assert_eq!(lines.len(), 3);
        assert!(lines[1].is_empty());
        assert!(check(text, &lines).is_ok());
        assert!(!any_styled(&lines));
    }

    #[test]
    fn a_formatting_that_does_not_match_its_text_is_refused() {
        let lines = vec![vec![
            ProseRun {
                t: "Hero ".into(),
                b: true,
                ..ProseRun::default()
            },
            ProseRun::plain("walks"),
        ]];
        assert!(check("Hero walks", &lines).is_ok());
        assert!(any_styled(&lines));
        assert!(check("Hero runs", &lines).is_err());
        assert!(check("Hero walks\nmore", &lines).is_err());
        let big = vec![vec![ProseRun {
            t: "x".into(),
            s: Some(400.0),
            ..ProseRun::default()
        }]];
        assert!(check("x", &big).unwrap_err().contains("size"));
    }
}
