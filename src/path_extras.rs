use std::path::{Path, PathBuf};

/// Compare the components of `path` with those of `prefix`, pairwise. For each
/// component they have in common, strip that away. Returns a new [`PathBuf`] for the
/// `path`, where all leading components that are the same as those of prefix has been
/// removed.
///
/// Returns [`None`] if they have no components in common, or if the prefix completely
/// "absorbed" the path, that is, if the prefix is equal to the path, execpt a bit
/// longer.
///
/// ```
/// let path = PathBuf::from("/home/user/some/file.txt");
/// let prefix = PathBuf::from("/home/user/");
/// let prefix_removed = PathBuf::from("some/file.txt");
/// assert_eq!(path_remove_prefix(path, prefix), prefix_removed);
/// ```
pub fn path_strip_prefix<'a>(path: &'a Path, prefix: &'a Path) -> Option<PathBuf> {
    // TODO possible to return a &'a Path, referring to the same path as `path`,
    // and allocating a new pathbuf?
    let mut path_components = path.components();
    let mut prefix_components = prefix.components();
    loop {
        match (path_components.next(), prefix_components.next()) {
            (Some(path_component), Some(prefix_component)) => {
                // they differ now, so return the rest of the path_components
                if path_component != prefix_component {
                    return Some(PathBuf::from_iter(path_components));
                }
            }
            (Some(path_component), None) => {
                // path was shorter than prefix. In other words, the entire "prefix"
                // was "stripped"
                let iter = std::iter::chain(std::iter::once(path_component), path_components);
                return Some(PathBuf::from_iter(iter));
            }
            (None, Some(_prefix_component)) => {
                // the rest of the component of the path is what we want to return
                return None;
            }
            (None, None) => {
                return None;
                return Some(PathBuf::from("/"));
            }
        }
    }
}

#[derive(Debug)]
pub struct NormalizeError;

/// Lifted from stdlib, because it's nightly only (for now)
pub fn path_normalize_lexically(path: &Path) -> Result<PathBuf, NormalizeError> {
    use std::path::Component;
    let mut lexical = PathBuf::new();
    let mut iter = path.components().peekable();

    // Find the root, if any, and add it to the lexical path.
    // Here we treat the Windows path "C:\" as a single "root" even though
    // `components` splits it into two: (Prefix, RootDir).
    let root = match iter.peek() {
        Some(Component::ParentDir) => return Err(NormalizeError),
        Some(p @ Component::RootDir) | Some(p @ Component::CurDir) => {
            lexical.push(p);
            iter.next();
            lexical.as_os_str().len()
        }
        Some(Component::Prefix(prefix)) => {
            lexical.push(prefix.as_os_str());
            iter.next();
            if let Some(p @ Component::RootDir) = iter.peek() {
                lexical.push(p);
                iter.next();
            }
            lexical.as_os_str().len()
        }
        None => return Ok(PathBuf::new()),
        Some(Component::Normal(_)) => 0,
    };

    for component in iter {
        match component {
            Component::RootDir => unreachable!(),
            Component::Prefix(_) => return Err(NormalizeError),
            Component::CurDir => continue,
            Component::ParentDir => {
                // It's an error if ParentDir causes us to go above the "root".
                if lexical.as_os_str().len() == root {
                    return Err(NormalizeError);
                } else {
                    lexical.pop();
                }
            }
            Component::Normal(path) => lexical.push(path),
        }
    }
    Ok(lexical)
}
