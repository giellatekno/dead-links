//! dead-links
//!
//! Scan through every .md file in the given directory (or the current working directory
//! if not given), filtering out those .md files that are ignored by .gitignore-files,
//! (or not, if --no-gitignore is given). Find all links, and write out the links that
//! does not point to a file that exists.
//!
//! Bugs:
//!   - Does not find links that span multiple lines (uses regex per line searching)
//!
//! Ideas:
//!   - Give an index find, and traverse all links. Report all files that are not part
//!     of the traversed path.

mod diagnostic;
mod file;
mod git;
mod link;
mod path_extras;
mod to_utf8_str;

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use clap::{Parser, ValueEnum};

use crate::diagnostic::Diagnostic;
use crate::file::File;
use crate::link::{parse_external_link, parse_internal_link};
use crate::path_extras::path_strip_prefix;

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, ValueEnum)]
enum DiagnosticName {
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
}

impl DiagnosticName {
    /// Get a `&'static str` for the variant.
    fn as_str(&self) -> &'static str {
        // must be the same as `crate::diagnostic::DiagnosticKind`
        match self {
            Self::Empty => "empty",
            Self::Nonexistant => "nonexistant",
            Self::Md => "md",
            Self::OutsideRoot => "outside-root",
            Self::InvalidExternalLink => "external-link-error",
        }
    }
}

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Args {
    /// The directory to scan. Defaults to the current working directory if not specified.
    #[arg(default_value = ".")]
    directory: PathBuf,

    /// Be more verbose
    #[arg(short, long, default_value_t = false)]
    verbose: bool,

    /// Use json-line output for the diagnostics. Useful for testing, and other automated
    /// things
    #[arg(long, default_value_t = false)]
    json: bool,

    /// Write each diagnostics on a single line. Has no effect if --json is given, as
    /// that always outputs each diagnostic as a single json object on each line
    #[arg(long, default_value_t = false)]
    oneline: bool,

    /// Check ALL markdown files, even if they are ignored by .gitignore(s)
    #[arg(short, long, default_value_t = false)]
    no_gitignore: bool,

    /// Turn off diagnostics. Can be given multiple times.
    #[arg(long, use_value_delimiter = true, value_delimiter = ',')]
    no_diag: Vec<DiagnosticName>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Args {
        directory,
        no_gitignore,
        verbose,
        json,
        oneline,
        no_diag,
    } = Args::parse();
    let root = directory
        .canonicalize()
        .map_err(|e| format!("directory {directory:?}: {e}"))?;

    let files = find_md_files(&root, !no_gitignore);
    if verbose {
        eprintln!("INFO: Found {} .md files", files.len());
    }
    let files = read_md_files(&files);

    let ignored_diags: HashSet<&str> = no_diag.iter().map(|val| val.as_str()).collect();

    // simple helper to output in json-line or string format, depending on if --json
    // argument was given or not
    let output = |diag: &Diagnostic| {
        if ignored_diags.contains(diag.kind.typename()) {
            return;
        }
        println!(
            "{}",
            if json {
                diag.to_json_line()
            } else if oneline {
                diag.to_string()
            } else {
                diag.to_multiline_string()
            }
        );
    };

    for file in files {
        for line in file.lines {
            for link in line.links {
                let rel_file =
                    path_strip_prefix(&file.path, &root).expect("file has root as prefix");
                let link_url = link.url(&line.string);
                let link_text = link.text(&line.string);
                let diag = Diagnostic::new(&rel_file, line.lineno, link_url);

                if link_url.trim().is_empty() {
                    output(&diag.empty(link_text));
                    continue;
                }

                // check: link points to invalid external link
                let parsed = match parse_external_link(link_url) {
                    Ok(Some(_parsed_external)) => {
                        // TODO: an idea: check external links for 200 ...? or not
                        continue;
                    }
                    Ok(None) => parse_internal_link(&file.path, &root, link_url),
                    Err(error) => {
                        output(&diag.invalid_external_link(error));
                        continue;
                    }
                };

                // TODO: check fragments
                let _is_fragment_only = link_url.starts_with("#");

                // if it was a broken external, it has already been handled
                assert!(parsed.scheme() == "file");

                let resolved_to = parsed
                    .to_file_path()
                    .expect("already checked scheme == \"file\"");
                //let resolved_to = path_normalize_lexically(&resolved_to)
                //    .expect("can always normalize all paths");

                // check: link points to a .md file
                if let Some(ext) = resolved_to.extension()
                    && ext == "md"
                {
                    output(&diag.md(&resolved_to));
                    continue;
                }

                // check if the link points to a file outside of the root
                let p = path_strip_prefix(&resolved_to, &root);
                if let Some(k) = p
                    && k.to_str().unwrap() == ""
                {
                    output(&diag.outside_root());
                    continue;
                }

                // check if the link points to a nonexistant file
                //
                // corner case: if it's an .html link, actually check the .md file
                // instead
                let resolved_to = match resolved_to.extension() {
                    Some(ext) if ext == "html" => resolved_to.with_extension("md"),
                    Some(_) | None => resolved_to,
                };
                match std::fs::exists(&resolved_to) {
                    Ok(true) => {}
                    Ok(false) => {
                        output(&diag.nonexistant(&resolved_to));
                    }
                    Err(error) => {
                        eprintln!("ERROR checking if file exists: '{resolved_to:?}': {error}");
                        continue;
                    }
                };
            }
        }
    }

    Ok(())
}

fn find_md_files<P: AsRef<Path>>(directory: P, git_ignore: bool) -> Vec<PathBuf> {
    fn handle_direntry_error(error: ignore::Error) {
        match error {
            ignore::Error::Loop { ancestor, child } => {
                eprintln!("link loop detected: {ancestor:?}, {child:?}");
            }
            ignore::Error::Io(error) => {
                eprintln!("io error: {error}");
            }
            error => {
                eprintln!("unhandled error: {error}");
            }
        }
    }

    fn is_md_file(entry: &ignore::DirEntry) -> bool {
        let path = entry.path();
        path.is_file() && path.extension().is_some_and(|extension| extension == "md")
    }

    let walker = ignore::WalkBuilder::new(directory.as_ref())
        .git_ignore(git_ignore)
        .build();
    let mut set: HashMap<PathBuf, ()> = HashMap::new();
    walker
        .into_iter()
        .filter_map(|res| res.map_err(handle_direntry_error).ok())
        .filter(is_md_file)
        .map(|entry| entry.into_path())
        .for_each(|pathbuf| {
            assert!(
                path_strip_prefix(&pathbuf, &directory.as_ref()).is_some(),
                "find_files() only finds file under the root we give"
            );

            let old = set.insert(pathbuf, ());
            assert!(old.is_none(), "pathbuf already added to set!");
        });
    Vec::from_iter(set.iter().map(|(k, ())| k.to_owned()))
}

fn read_md_files(files: &[PathBuf]) -> Vec<crate::File> {
    let re = regex::Regex::new(r#"\[(?<text>[^\]]*)\]\((?<link>[^\)]*)\)"#).unwrap();

    Vec::from_iter(files.into_iter().filter_map(|path| {
        File::read(&path, &re)
            .inspect_err(|error| {
                eprintln!("ERROR: reading file {path:?}: {error}");
            })
            .ok()
    }))
}
