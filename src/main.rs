//! dead-links
//!
//! Scan through every .md file in the given directory (or the current working directory
//! if not given), filtering out those .md files that are ignored by .gitignore-files,
//! (or not, if --no-gitignore is given). Find all links, and write out the links that
//! does not point to a file that exists.
//!
//! Ideas:
//!   - Unreachable check: If given the root directory, traverse all links from index.md,
//!     to see which files are not reachable from the site root. This requires that the
//!     cache contains all the links (but they do, the link_url).

mod cache;
mod cli_parsing;
mod diagnostic;
mod file;
mod git;
mod link;
mod path_extras;
mod to_utf8_str;
mod invalid_external_link_serde;

use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};

use rayon::prelude::*;

use crate::diagnostic::Diagnostic;
use crate::link::{parse_external_link, parse_internal_link};
use crate::path_extras::path_strip_prefix;

use crate::cli_parsing::{OutputFormat, parse_cli_args};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (path, no_gitignore, verbose, format, diags) = parse_cli_args()?;
    let diags: HashSet::<&str> = diags.iter().map(|diag| diag.as_str()).collect();

    let Some(root) = find_site_root(&path)? else {
        return Err(Box::from(format!(
            "Not a jekyll site '{}': No _config.yml or _config.toml found among its parents",
            path.display()
        )));
    };

    // simple helper to output in json-line or string format, depending on if --json
    // argument was given or not
    let output = |diag: &Diagnostic| {
        if !diags.contains(diag.kind.typename()) {
            return;
        }
        let s = match format {
            OutputFormat::Oneline => diag.to_string(),
            OutputFormat::Json => diag.to_json_line(),
            OutputFormat::Fancy => {
                match diag.to_miette_fancy(&root) {
                    Ok(s) => s,
                    Err(e) => {
                        let p = root.join(&diag.file);
                        eprintln!("E: could not read '{}': {e}", p.display());
                        return;
                    }
                }
            }
            OutputFormat::Multiline => diag.to_multiline_string(),
        };
        println!("{s}");
    };

    // cache: a set of all .md files, with content hash, and diagnostics for that hash,
    // for the last run.
    let mut cache = crate::cache::Cache::new(root.as_path())?;

    // Set: Files: All .md files. (new files could exist, also, files could have been
    // deleted)
    let files = find_md_files(&path, !no_gitignore);

    if verbose {
        eprintln!("INFO: Found {} .md files", files.len());
    }

    // When we iterate files, we want to iterate only those files that have changed,
    // the other diagnostics we can just directly print from the cache.

    // remove entries from the cache that is missing in the set of current files, these
    // files have been removed
    cache.remove_missing(&files);

    // print diagnostics from the cache first
    for x in cache.all_diagnostics() {
        output(x);
    }
    let nfiles = files.len();

    let cache = std::sync::Arc::new(std::sync::Mutex::new(cache));

    files
        .into_par_iter()
        .filter_map(|path| {
            std::fs::File::open(&path)
                .inspect_err(|e| eprintln!("E: can't open file {}: {e}", path.display()))
                .ok()
                .map(|fp| (fp, path))
        })
        .map(move |(fp, path)| (unsafe { memmap2::Mmap::map(&fp).unwrap() }, path))
        .map(|(mm, path)| (path, calculate_hash(&mm[..]), mm))
        .filter(|(path, hash, _mm)| {
            // Filter out every file whose hash in the cache is the same as those we just
            // calucalted
            match cache.lock().expect("not poisoned").get(&path) {
                // file path was in cache, check if the hash is the same now as in the
                // cache. if they are not the same, it means that the content has changed,
                // so we should keep it.
                Some(obj) => obj.hash != *hash,
                // file path was not found in cache, so it's a new file, so we want to
                // keep it, i.e. not filter it out
                None => true,
            }
        })
        .filter_map(|(path, hash, mm)| {
            let buf = std::str::from_utf8(&mm[..])
                .inspect_err(|e| {
                    eprintln!("W: file contains invalid utf-8 '{}': {e}", path.display());
                })
                .ok()?;

            let ast = crate::file::parse_md(buf).expect("markdown can't faile to parse");
            let links = crate::file::mdast_find_links(&ast);
            let file = crate::file::File { path, links };
            Some((file, hash))
        })
        .map(|(file, hash)| (diagnose(&file, &root), file, hash))
        .for_each(|(diagnostics, file, hash)| {
            diagnostics.iter().for_each(output);
            cache
                .lock()
                .expect("not poisoned")
                .insert(&file.path, hash, diagnostics);
        });

    let cache = std::sync::Arc::into_inner(cache).expect("we are the only strong ref");
    let cache = std::sync::Mutex::into_inner(cache).expect("not poisoned");
    let ndiagnostics = cache.all_diagnostics().count();
    cache.save().expect("could save");
    if verbose {
        eprintln!("cache saved to .dead-links-cache ({ndiagnostics} diagnostics over {nfiles} files)");
    }

    Ok(())
}

fn find_md_files<P: AsRef<Path>>(path: P, git_ignore: bool) -> Vec<PathBuf> {
    let path = path.as_ref();
    match std::fs::metadata(path) {
        Ok(meta) => {
            if meta.is_file() {
                let canon = match path.canonicalize() {
                    Ok(canon) => canon,
                    Err(e) => {
                        eprintln!("E: can't canonicalize '{}': {e}", path.display());
                        return vec![];
                    }
                };
                return vec![canon];
            }
        }
        Err(e) => {
            eprintln!("E: Can't stat {}: {e}", path.display());
            return vec![];
        }
    }

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

    let walker = ignore::WalkBuilder::new(path)
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
                path_strip_prefix(&pathbuf, &path).is_some(),
                "find_files() only finds file under the root we give"
            );

            let old = set.insert(pathbuf, ());
            assert!(old.is_none(), "pathbuf already added to set!");
        });
    Vec::from_iter(set.iter().map(|(k, ())| k.to_owned()))
}

/// Finds the site root of a jekyll site. It's the directory up through the parents
/// that contains a `_config.(yml|toml)` file. Returns the path to the directory, or
/// `None`.
fn find_site_root<P: AsRef<Path>>(path: P) -> io::Result<Option<PathBuf>> {
    let path = path.as_ref();
    let mut maybe_dir: Option<&Path> = dir_or_parent(path)?;

    while let Some(dir) = maybe_dir {
        for entry in std::fs::read_dir(dir)? {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    eprintln!("E: can't read dir entry: {e}");
                    continue;
                }
            };

            let filename = entry.file_name();
            let is_config_file = filename == "_config.yml" || filename == "_config.toml";
            if !is_config_file {
                continue;
            }

            let filetype = match entry.file_type() {
                Ok(ft) => ft,
                Err(e) => {
                    eprintln!("E: Can't read file type of {filename:?}: {e}");
                    continue;
                }
            };

            if filetype.is_file() {
                return Ok(Some(dir.to_path_buf()));
            }
        }

        maybe_dir = dir.parent();
    }

    Ok(None)
}

/// If `path` is a directory, return itself. If it's a file, return the path of
/// the parent (the directory the file is in). Returns `Ok(None)` if path points
/// to the root.
fn dir_or_parent<'a>(path: &'a Path) -> io::Result<Option<&'a Path>> {
    let meta = std::fs::metadata(path)?;
    if meta.is_dir() {
        return Ok(Some(path));
    }

    Ok(path.parent())
}

fn calculate_hash(data: &[u8]) -> String {
    use sha1::{Digest, Sha1};
    let mut hasher = Sha1::new();
    hasher.update(&data[..]);
    let hash = hasher.finalize();
    hex::encode(hash)
}

/// Find the diagnostics ("to diagnose") of a file.
fn diagnose(file: &crate::file::File, root: &Path) -> Vec<Diagnostic> {
    let mut diagnostics = vec![];

    for link in &file.links {
        let rel_file = path_strip_prefix(&file.path, &root).expect("file has root as prefix");
        let link_url = link.url.trim();
        let link_text = match link.title {
            Some(ref s) => s.as_str(),
            None => "<bare link>",
        };
        let lineno = link.lineno;
        let colno = link.colno;
        let endlineno = link.endlineno;
        let endcolno = link.endcolno;
        let diag = Diagnostic::new(&rel_file, link_url, lineno, colno, endlineno, endcolno);

        if link_url.is_empty() {
            diagnostics.push(diag.empty(link_text));
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
                diagnostics.push(diag.invalid_external_link(error));
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
            diagnostics.push(diag.md(&resolved_to));
            continue;
        }

        // check if the link points to a file outside of the root
        let p = path_strip_prefix(&resolved_to, &root);
        if let Some(k) = p
            && k.to_str().unwrap() == ""
        {
            diagnostics.push(diag.outside_root());
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
                diagnostics.push(diag.nonexistant(&resolved_to));
            }
            Err(error) => {
                eprintln!("ERROR checking if file exists: '{resolved_to:?}': {error}");
                continue;
            }
        };
    }

    diagnostics
}
