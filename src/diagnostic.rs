//! This modue defines the errors that can be found for a link (called a "diagnostic"),
//! and how to present them in json-line and string forms.
//!
//! It does NOT contain code on a link to determine which of these ones it is (if any).

use std::fmt::Write;
use std::path::{Path, PathBuf};

use crate::to_utf8_str::ToUtf8Str;

/// An error (or likely probable error) that can occur with a link.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Diagnostic {
    /// Which kind of error this is.
    pub kind: DiagnosticKind,
    /// Path to the file with the error.
    pub file: PathBuf,
    /// The link url, where the link points to.
    url: String,
    /// Line number where this link is.
    lineno: usize,
    /// Column number on that line where the link is.
    colno: usize,
    endlineno: usize,
    endcolno: usize,
}

/// The kinds of diagnostic for this link.
#[derive(serde::Serialize, serde::Deserialize)]
pub enum DiagnosticKind {
    /// The link url is empty. Contained value is the link text.
    Empty { link_text: String },
    /// Invalid external link. Link couldn't be parsed as an external link  by the `url`
    /// crate.

    #[serde(with = "crate::invalid_external_link_serde")]
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
    pub fn typename(&self) -> &'static str {
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
    url: &'a str,
    lineno: usize,
    colno: usize,
    endlineno: usize,
    endcolno: usize,
}

impl DiagnosticBuilder<'_> {
    pub fn empty(&self, link_text: &str) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::Empty {
                link_text: link_text.to_owned(),
            },
            file: self.file.to_owned(),
            url: self.url.to_owned(),
            lineno: self.lineno,
            colno: self.colno,
            endlineno: self.endlineno,
            endcolno: self.endcolno,
        }
    }

    pub fn md(&self, resolved_to: &Path) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::Md {
                resolved_path: resolved_to.to_owned(),
            },
            file: self.file.to_owned(),
            url: self.url.to_owned(),
            lineno: self.lineno,
            colno: self.colno,
            endlineno: self.endlineno,
            endcolno: self.endcolno,
        }
    }

    pub fn nonexistant(&self, resolved_to: &Path) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::Nonexistant {
                resolved_path: resolved_to.to_owned(),
            },
            file: self.file.to_owned(),
            url: self.url.to_owned(),
            lineno: self.lineno,
            colno: self.colno,
            endlineno: self.endlineno,
            endcolno: self.endcolno,
        }
    }

    pub fn outside_root(&self) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::OutsideRoot,
            file: self.file.to_owned(),
            url: self.url.to_owned(),
            lineno: self.lineno,
            colno: self.colno,
            endlineno: self.endlineno,
            endcolno: self.endcolno,
        }
    }

    pub fn invalid_external_link(&self, error: url::ParseError) -> Diagnostic {
        Diagnostic {
            kind: DiagnosticKind::InvalidExternalLink { error },
            file: self.file.to_owned(),
            url: self.url.to_owned(),
            lineno: self.lineno,
            colno: self.colno,
            endlineno: self.endlineno,
            endcolno: self.endcolno,
        }
    }
}

impl Diagnostic {
    pub fn new<'a>(
        file: &'a Path,
        url: &'a str,
        lineno: usize,
        colno: usize,
        endlineno: usize,
        endcolno: usize,
    ) -> DiagnosticBuilder<'a> {
        DiagnosticBuilder { file, url, lineno, colno, endlineno, endcolno }
    }

    pub fn to_multiline_string(&self) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "{}:", self.kind.typename());
        let _ = writeln!(out, "  File: {}", self.file.to_utf8_str());
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
                let resolved = resolved_path.to_utf8_str();
                let _ = writeln!(out, "  Link resolved to: {resolved}");
            }
            DiagnosticKind::Nonexistant { resolved_path } => {
                let resolved = resolved_path.to_utf8_str();
                let _ = writeln!(out, "  Link resolved to: {resolved}");
            }
            DiagnosticKind::OutsideRoot => {}
        }

        out
    }

    pub fn to_json_line(&self) -> String {
        let s = serde_json::to_string(self)
            .expect("diagnostic is serializable to json");
        // so important that the json doesn't contain a newline, it will break
        // the format, so assert on it
        assert!(!s.contains('\n'), "json contains no newline");
        s
    }

    pub fn to_miette_fancy(&self, site_root: &Path) -> std::io::Result<String> {
        let theme = miette::GraphicalTheme::unicode();
        let reporter = miette::GraphicalReportHandler::new_themed(theme)
            .with_context_lines(4);

        diagnostic_to_miette_fancy(self, site_root, &reporter)
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let link = self.url.as_str();
        let line = self.lineno;
        let file = self.file.to_utf8_str();
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
                let resolved = resolved_path.to_utf8_str();
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

use miette::{NamedSource, SourceSpan, SourceOffset};

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("oops!")]
#[diagnostic(
    code(empty),
    help("provide the link url")
)]
struct MietteDiagEmpty {
    file: PathBuf,
    // The Source that we're gonna be printing snippets out of.
    // This can be a String if you don't have or care about file names.
    #[source_code]
    src: NamedSource<String>,
    // Snippets and highlights can be included in the diagnostic!
    #[label("This bit here")]
    bad_bit: SourceSpan,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("link points to a file that doesn't exist")]
#[diagnostic(
    code(nonexistant),
    help("make sure the link points to a file that exists")
)]
struct MietteDiagNonexistant {
    file: PathBuf,
    #[source_code]
    src: NamedSource<String>,
    #[label("does not exist")]
    bad_bit: SourceSpan,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("relativel link points to a file outside of site root")]
#[diagnostic(
    code("outside-root"),
    help("ensure the relative link points to a file within the site root. maybe the document was moved?")
)]
struct MietteDiagOutsideRoot {
    file: PathBuf,
    #[source_code]
    src: NamedSource<String>,
    #[label("points to outside root")]
    bad_bit: SourceSpan,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("external link is malformed")]
#[diagnostic(
    code("invalid-external-link"),
    help("the external link is malformed. ensure it is correct")
)]
struct MietteDiagInvalidExternalLink {
    file: PathBuf,
    #[source_code]
    src: NamedSource<String>,
    #[label("bad external link")]
    bad_bit: SourceSpan,
}

#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[error("link points to .md file")]
#[diagnostic(
    code("markdown"),
    help("the link points to a .md file, which may not be what you want")
)]
struct MietteDiagMarkdownLink {
    file: PathBuf,
    // The Source that we're gonna be printing snippets out of.
    // This can be a String if you don't have or care about file names.
    #[source_code]
    src: NamedSource<String>,
    // Snippets and highlights can be included in the diagnostic!
    #[label("links to .md")]
    bad_bit: SourceSpan,
}

fn diagnostic_to_miette_fancy(
    diag: &Diagnostic,
    site_root: &Path,
    reporter: &miette::GraphicalReportHandler,
) -> std::io::Result<String> {
    let mut out = String::new();
    let full_path = site_root.join(&diag.file);
    let source_code = std::fs::read_to_string(full_path)?;
    let filename = diag.file.to_str().expect("filename is valid utf-8");
    let start_offset = SourceOffset::from_location(&source_code, diag.lineno, diag.colno);
    let len = SourceOffset::from_location(&source_code, diag.endlineno, diag.endcolno)
        .offset()
        .checked_sub(start_offset.offset())
        .expect("markdown parser won't ever say that end < start");
    let span = SourceSpan::new(start_offset, len);

    match &diag.kind {
        DiagnosticKind::Empty { link_text } => {
            let inst = MietteDiagEmpty {
                file: diag.file.to_owned(),
                src: NamedSource::new(filename, source_code),
                bad_bit: span,
            };
            reporter.render_report(&mut out, &inst).unwrap();
        }
        DiagnosticKind::Nonexistant { resolved_path } => {
            let inst = MietteDiagNonexistant {
                file: diag.file.to_owned(),
                src: NamedSource::new(filename, source_code),
                bad_bit: span,
            };
            reporter.render_report(&mut out, &inst).unwrap();
        }
        DiagnosticKind::OutsideRoot => {
            let inst = MietteDiagOutsideRoot {
                file: diag.file.to_owned(),
                src: NamedSource::new(filename, source_code),
                bad_bit: span,
            };
            reporter.render_report(&mut out, &inst)
                .expect("formatting to string is infallible");
        }
        DiagnosticKind::InvalidExternalLink { error } => {
            let inst = MietteDiagInvalidExternalLink {
                file: diag.file.to_owned(),
                src: NamedSource::new(filename, source_code),
                bad_bit: span,
            };
            reporter.render_report(&mut out, &inst)
                .expect("formatting to string is infallible");
        }
        DiagnosticKind::Md { resolved_path } => {
            let inst = MietteDiagMarkdownLink {
                file: diag.file.to_owned(),
                src: NamedSource::new(filename, source_code),
                bad_bit: span,
            };
            reporter.render_report(&mut out, &inst)
                .expect("formatting to string is infallible");
        }
        _ => todo!()
    }
    Ok(out)
}
