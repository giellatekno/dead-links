use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::diagnostic::Diagnostic;

/// The file cache is a single file, located in `SITE_ROOT/.dead-links-cache`.
/// After the first run, the program writes one line for each .md file found, in
/// the following format:
///
/// ```not_rust
/// HASH <space> PATH <space> <DIAGS>
/// ```
///
/// where HASH is the sha-1 hash of the contents of the file located in path PATH,
/// and DIAGS is a json-encoded list of diagnostics found for that file.
///
/// The PATH is written as an absolute path, but the root is the root of the site, not
/// the filesystem. For example, if the SITE is located at the filesystem path
/// `/home/user/repos/github-org/docs`, and the PATH stored in the cache file is
/// `/subdir/file.md`, the full path to that file on the filesystem, would be
/// `/home/user/repos/github-org/docs/subdir/file.md`.
pub struct Cache {
    /// The path to the cache file
    path: PathBuf,
    /// path -> (hash, json-encoded-diags)
    files: HashMap<PathBuf, CacheEntry>,
}

pub struct CacheEntry {
    pub hash: String,
    pub diagnostics: Vec<Diagnostic>,
}

impl Cache {
    const CACHE_FILE_NAME: &str = ".dead-links-cache";

    pub fn new(root: &Path) -> Result<Self, std::io::Error> {
        use std::io::{BufRead, BufReader};

        let path = root.join(Self::CACHE_FILE_NAME);
        let mut files = HashMap::new();
        let fp = match std::fs::File::open(&path) {
            Ok(fp) => fp,
            Err(e) if matches!(e.kind(), std::io::ErrorKind::NotFound) => {
                return Ok(Self { path, files });
            }
            Err(e) => {
                eprintln!(
                    "Fatal: error when opening cache file {}: {e}",
                    path.display()
                );
                return Err(e);
            }
        };
        let bufreader = BufReader::new(fp);
        for line in bufreader.lines() {
            let line = line?;
            let mut it = line.split(' ');
            let hash = it.next().expect("has a hash").to_owned();
            let path = it.next().expect("hash a path");
            let path = PathBuf::from(path);
            let diagnostics: String = it.collect();
            let diagnostics: Vec<Diagnostic> =
                serde_json::from_str(&diagnostics).expect("can parse as json");
            files.insert(path, CacheEntry { hash, diagnostics });
        }
        Ok(Self { path, files })
    }

    /// Get a cache entry belonging to a path.
    pub fn get(&self, path: &Path) -> Option<&CacheEntry> {
        self.files.get(path)
    }

    pub fn remove_missing(&mut self, paths: &[PathBuf]) {
        let set: HashSet<&PathBuf> = paths.iter().collect();
        self.files.retain(|k, _v| set.contains(k));
    }

    pub fn insert(&mut self, path: &Path, hash: String, diagnostics: Vec<Diagnostic>) {
        let entry = CacheEntry { hash, diagnostics };
        self.files.insert(path.to_path_buf(), entry);
    }

    pub fn save(&self) -> std::io::Result<()> {
        use std::fmt::Write;
        let mut data = String::new();
        for (path, entry) in self.files.iter() {
            let hash = &entry.hash;
            let json_line = serde_json::to_string(&entry.diagnostics)
                .expect("diagnostics serializes succesfully");
            writeln!(data, "{hash} {} {json_line}", path.display())
                .expect("writing to string never fails");
        }

        let fp_result = std::fs::OpenOptions::new().write(true).create(true).open(&self.path);
        let fp = match fp_result {
            Ok(fp) => fp,
            Err(e) => {
                eprintln!(
                    "E: Can't open cache file for writing '{}': {e}",
                    self.path.display()
                );
                return Err(e);
            }
        };
        std::fs::write(&self.path, data)?;
        Ok(())
    }

    pub fn all_diagnostics(&self) -> impl Iterator<Item = &Diagnostic> {
        self.files.iter()
            .flat_map(|(path, entry)| {
                entry.diagnostics.iter()
            })
    }
}
