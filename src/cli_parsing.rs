use clap::{Args, Parser, ValueEnum};

use std::path::PathBuf;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum DiagnosticName {
    /// Link url is empty.
    Empty,
    /// External link is invalid.
    InvalidExternalLink,
    /// Link points to .md (instead of .html).
    Md,
    /// The pointed-to file does not exist.
    Nonexistant,
    /// Relative link points to a path outside of the root.
    OutsideRoot,
    /// Document is unreachable from the root. That is, there are no paths of links
    /// that reaches this document, when starting from the root.
    Unreachable,
}

const ALL_DIAGNOSTICS: [DiagnosticName; 5] = [
    DiagnosticName::Empty,
    DiagnosticName::InvalidExternalLink,
    DiagnosticName::Md,
    DiagnosticName::Nonexistant,
    DiagnosticName::OutsideRoot,
];

impl DiagnosticName {
    /// Get a `&'static str` for the variant.
    pub fn as_str(&self) -> &'static str {
        // must be the same as `crate::diagnostic::DiagnosticKind`
        match self {
            Self::Empty => "empty",
            Self::Nonexistant => "nonexistant",
            Self::Md => "md",
            Self::OutsideRoot => "outside-root",
            Self::InvalidExternalLink => "external-link-error",
            Self::Unreachable => "unreachable",
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
pub enum OutputFormat {
    /// One diagnostic per line, more compact representation
    Oneline,
    /// The multi line format
    Multiline,
    /// Turn diagnostics into a miette report, and output them as fancy output
    Fancy,
    /// One diagnostic per line, json-encoded object.
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            OutputFormat::Oneline => "oneline",
            OutputFormat::Multiline => "multiline",
            OutputFormat::Fancy => "fancy",
            OutputFormat::Json => "json",
        })
    }
}

#[derive(Args, Debug)]
#[group(required = false, multiple = false)]
struct Format {
    /// Use json-line output for the diagnostics.
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Write each diagnostics on a single line. Has no effect if --json is given, as
    /// that always outputs each diagnostic as a single json object on each line
    #[arg(long, default_value_t = false)]
    oneline: bool,

    /// Use Miette-Fancy diagnostics style output
    #[arg(long, default_value_t = false)]
    fancy: bool,

    /// Multi line output, the default
    #[arg(long, default_value_t = false)]
    multiline: bool,

    #[arg(long)]
    format: Option<OutputFormat>,
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct ProgramArgs {
    /// The file to check, or directory to scan
    #[arg(default_value = ".")]
    path: PathBuf,

    /// Be more verbose
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    #[command(flatten)]
    format: Option<Format>,

    /// Check ALL markdown files, even if they are ignored by .gitignore(s)
    #[arg(short, long, default_value_t = false)]
    no_gitignore: bool,

    #[command(flatten)]
    diagnostics: Diagnostics,
}

#[derive(Args, Debug)]
#[group(required = false, multiple = false)]
struct Diagnostics {
    /// Turn off diagnostics. Can be given multiple times
    #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
    no_diag: Vec<DiagnosticName>,

    /// Turn on these diagnostics, can be a comma separated list, and/or can be
    /// given multiple times.
    #[arg(short, long, alias = "diagnostic", use_value_delimiter = true, value_delimiter = ',')]
    diag: Vec<DiagnosticName>,
}

pub fn parse_cli_args() -> Result<(PathBuf, bool, bool, OutputFormat, Vec<DiagnosticName>), String> {
    let ProgramArgs {
        path,
        no_gitignore,
        verbose,
        format,
        diagnostics,
    } = ProgramArgs::parse();

    let diags = match (diagnostics.diag.as_slice(), diagnostics.no_diag.as_slice()) {
        ([], []) => ALL_DIAGNOSTICS.to_vec(),
        ([..], []) => diagnostics.diag,
        ([], [no_diags @ ..]) => ALL_DIAGNOSTICS.to_vec().into_iter()
            .filter(|diag| !no_diags.contains(diag))
            .collect(),
        _ => unreachable!("clap makes sure this can't happen"),
    };

    let path = path
        .canonicalize()
        .map_err(|e| format!("path {path:?}: {e}"))?;

    let format = match format {
        Some(format) => {
            // one, but only one, of --json, --oneline, --format FORMAT etc was given
            if let Some(format) = format.format {
                // gave --format FORMAT, the inner is already an OutputFormat
                format
            // otherwise, check them all manually, and convert accordingly
            } else if format.multiline {
                OutputFormat::Multiline
            } else if format.oneline {
                OutputFormat::Oneline
            } else if format.json {
                OutputFormat::Json
            } else if format.fancy {
                OutputFormat::Fancy
            } else {
                unreachable!("clap already made sure one was chosen")
            }
        }
        None => {
            // no --json, --format FORMAT etc arg given at all
            OutputFormat::Multiline
        }
    };

    Ok((path, no_gitignore, verbose, format, diags))
}
