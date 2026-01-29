//! This modue defines the errors that can be found for a link (called a "diagnostic"),
//! and how to present them in json-line and string forms.
//!
//! It does NOT contain code on a link to determine which of these ones it is (if any).

use std::fmt::Write;
use std::path::{Path, PathBuf};

/// An error (or likely probable error) that can occur with a link. This is the common
/// properties.
pub struct Diagnostic {
    /// Path to the file with the error.
    file: PathBuf,
    /// Line number where this link occurs in the file.
    lineno: usize,
    /// The link url, where the link points to.
    url: String,
    /// Which kind of error this is.
    kind: DiagnosticKind,
}

/// The kinds of diagnostic for this link.
pub enum DiagnosticKind {
    /// The link url is empty. Contained value is the link text.
    Empty { link_text: String },
    /// Invalid external link. Link couldn't be parsed as an external link  by the `url`
    /// crate.
    InvalidExternalLink { error: url::ParseError },
    /// Link points to a nonexistant file. The contained value is the resolved path to
    /// where it points to (the file that wasn't found)
    Md { resolved_path: PathBuf },
    /// A relative link points outside (below) the root.
    Nonexistant { resolved_path: PathBuf },
    /// A link targets a .md file (which is probably not intented). The contained value
    /// is the resolved path to where the link points to.
    OutsideRoot,
}

impl DiagnosticKind {
    /// The name of this kind, in ALL CAPS.
    fn typename(&self) -> &'static str {
        use DiagnosticKind::*;
        match self {
            Empty { .. } => "empty",
            Nonexistant { .. } => "nonexistant",
            Md { .. } => "md",
            OutsideRoot => "outside-root",
            InvalidExternalLink { .. } => "external-link-error",
        }
    }

    fn typename_upper(&self) -> &'static str {
        use DiagnosticKind::*;
        match self {
            Empty { .. } => "EMPTY",
            Nonexistant { .. } => "NONEXISTANT",
            Md { .. } => "MD",
            OutsideRoot => "OUTSIDE-ROOT",
            InvalidExternalLink { .. } => "EXTERNAL-LINK-ERROR",
        }
    }

    /// Get a human-readable description of this error kind
    fn description(&self) -> &'static str {
        use DiagnosticKind::*;
        match self {
            Empty { .. } => "Link is empty, or consists only of whitespace",
            Nonexistant { .. } => "Link points to a file path that doesn't exist",
            Md { .. } => "Link points to file with .md extention, this is probably not intenteded",
            OutsideRoot => "Link points to path outside (below) the root.",
            InvalidExternalLink { .. } => "The external link contained errors.",
        }
    }
}

pub struct DiagnosticBuilder<'a> {
    file: &'a Path,
    lineno: usize,
    url: &'a str,
}

impl DiagnosticBuilder<'_> {
    pub fn empty(&self, link_text: &str) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::Empty {
                link_text: link_text.to_owned(),
            },
            file: self.file.to_owned(),
            lineno: self.lineno,
            url: self.url.to_owned(),
        }
    }

    pub fn md(&self, resolved_to: &Path) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::Md {
                resolved_path: resolved_to.to_owned(),
            },
            file: self.file.to_owned(),
            lineno: self.lineno,
            url: self.url.to_owned(),
        }
    }

    pub fn nonexistant(&self, resolved_to: &Path) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::Nonexistant {
                resolved_path: resolved_to.to_owned(),
            },
            file: self.file.to_owned(),
            lineno: self.lineno,
            url: self.url.to_owned(),
        }
    }

    pub fn outside_root(&self) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::OutsideRoot,
            file: self.file.to_owned(),
            lineno: self.lineno,
            url: self.url.to_owned(),
        }
    }

    pub fn invalid_external_link(&self, error: url::ParseError) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::InvalidExternalLink { error },
            file: self.file.to_owned(),
            lineno: self.lineno,
            url: self.url.to_owned(),
        }
    }
}

impl Diagnostic {
    pub fn new<'a>(file: &'a Path, lineno: usize, url: &'a str) -> DiagnosticBuilder<'a> {
        DiagnosticBuilder { file, lineno, url }
    }

    pub fn to_multiline_string(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{}:", self.kind.typename());
        let _ = writeln!(out, "  File: {}", self.file.to_str().unwrap());
        let _ = writeln!(out, "  Line: {}", self.lineno);
        let _ = writeln!(out, "  Url: {}", self.url);
        let _ = writeln!(out, "  Description: {}", self.kind.description());

        match &self.kind {
            DiagnosticKind::Empty { link_text } => {
                let _ = writeln!(out, "  Link text: '{link_text}'");
            }
            DiagnosticKind::InvalidExternalLink { error } => {
                let _ = writeln!(out, "  Error: {}", error);
            }
            DiagnosticKind::Md { resolved_path } => {
                let resolved = resolved_path.to_str().expect("paths are utf-8");
                let _ = writeln!(out, "  Link resolved to: {resolved}");
            }
            DiagnosticKind::Nonexistant { resolved_path } => {
                let resolved = resolved_path.to_str().expect("paths are utf-8");
                let _ = writeln!(out, "  Link resolved to: {resolved}");
            }
            DiagnosticKind::OutsideRoot => {
            }
        }

        out
    }

    pub fn to_json_line(&self) -> String {
        // TODO use a lib?
        let mut out = self.json_line_header();
        match &self.kind {
            DiagnosticKind::Empty { link_text } => {
                let _ = write!(out, ",\"link_text\":\"{link_text}\"");
            }
            DiagnosticKind::InvalidExternalLink { error } => {
                let _ = write!(out, ",\"error\":\"{error}\"");
            }
            DiagnosticKind::Nonexistant { resolved_path } => {
                let resolved_to = resolved_path.to_str().unwrap();
                let _ = write!(out, ",\"resolved_to\":\"{resolved_to}\"");
            }
            DiagnosticKind::Md { resolved_path } => {
                let resolved_to = resolved_path.to_str().unwrap();
                let _ = write!(out, ",\"resolved_to\":\"{resolved_to}\"");
            }
            DiagnosticKind::OutsideRoot => { /* nothing to do */ }
        }
        out.push('}');
        out
    }

    /// Get a half-done json output, with the common headers set.
    fn json_line_header(&self) -> String {
        // TODO use a lib?
        let file = self.file.to_str().expect("file path is valid utf-8");
        let typename = self.kind.typename();
        let lineno = self.lineno;
        let url = self.url.as_str();
        let mut out = String::new();
        let _ = write!(out, "{{\"type\":\"{typename}\",\"file\":\"{file}\"");
        let _ = write!(out, ",\"lineno\":{lineno},\"url\":\"{url}\"");
        out
    }
}

fn ascii_to_uppercase_inplace(input: &mut str) -> &str {
    fn upper(x: &mut u8) {
        if *x >= b'a' {
            *x = *x | 0b01000000;
        }
    }
    unsafe {
        input.as_bytes_mut().iter_mut().for_each(upper);
        input
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let link = self.url.as_str();
        let line = self.lineno;
        let file = self.file.to_str().expect("valid utf-8");
        let kind = self.kind.typename_upper();

        write!(f, "{kind}: link '{link}' on line {line} in file '{file}': ")?;

        match &self.kind {
            DiagnosticKind::Empty { link_text } => {
                writeln!(f, "(link text='{link_text}')")
            }
            DiagnosticKind::InvalidExternalLink { error } => {
                writeln!(
                    f,
                    "link looks external, but could not be parsed as such. Inner parse error: {error}"
                )
            }
            DiagnosticKind::Md { resolved_path } => {
                // TODO use resolved_path
                writeln!(
                    f,
                    "links to .md file instead of .html, probably unintentionally"
                )
            }
            DiagnosticKind::Nonexistant { resolved_path } => {
                let resolved = resolved_path.to_str().unwrap();
                writeln!(
                    f,
                    "Pointed-to file (resolved to: '{resolved}') does not exist"
                )
            }
            DiagnosticKind::OutsideRoot => {
                writeln!(f, "link points to file outside of the root")
            }
        }
    }
}
