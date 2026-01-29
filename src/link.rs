//! link.rs
//!
//! Parsing links found in the .md files to [`url::Url`].

use std::path::Path;

/// Try to parse a link as an external link. Returns `Ok(Some(parsed))` if it could,
/// `Ok(None)` if not, and `Err()` if link failed to parse as an external link.
pub fn parse_external_link(link: &str) -> Result<Option<url::Url>, url::ParseError> {
    match url::Url::parse(link) {
        Ok(parsed) => Ok(Some(parsed)),
        Err(url::ParseError::RelativeUrlWithoutBase) => Ok(None),
        Err(e) => Err(e),
    }
}

/// Parse a link in a file. It needs the file path to the file it is parsing,
/// the root directory we're searching from, and the link url text as a string to parse.
///
/// The file `file_path` is the absolute path to the file we're checking.
///
/// NOTE: This parsing cannot fail - any string should be a valid url.
pub fn parse_internal_link(file_path: &Path, root: &Path, link: &str) -> url::Url {
    // Implementation note: We can't use Url::from_file_path - it loses the
    // fragment (thinks it's part of the file name)

    // the file path must be absolute
    assert!(file_path.is_absolute());

    // not allowed: empty link (is checked before we call this code)
    assert!(!link.is_empty());

    let (link, fragment) = match link.split_once("#") {
        Some((link, fragment)) => (link, fragment),
        None => (link, ""),
    };

    let p = file_path.parent().expect("all file paths have a parent");
    let resolved_path = p.join(Path::new(link));
    let resolved_path_str = resolved_path.to_str().unwrap();

    // absolute link. strip the "/" prefix
    let mut link_to_parse = if let Some(link) = link.strip_prefix("/") {
        let root_str = root.to_str().expect("all links are valid utf-8");
        assert!(!root_str.ends_with("/"));
        format!("file://{root_str}/{link}")
    } else if link.is_empty() {
        // if the link is ONLY a fragment, don't have a file target in the link,
        // so use the path to the file we're inside as the file.
        let file = file_path.to_str().expect("file paths are valid utf-8");
        format!("file://{file}")
    } else {
        format!("file://{resolved_path_str}")
    };

    if !fragment.is_empty() {
        link_to_parse.push('#');
        link_to_parse.push_str(fragment);
    }


    url::Url::parse(&link_to_parse).expect("no internal links can fail to parse")
}

#[cfg(test)]
mod tests {
    use super::*;
    use url::Url;

    fn t(expected: &str, path: &Path, root: &Path, link: &str) {
        assert_eq!(Url::parse(expected).unwrap(), parse_internal_link(path, root, link));
    }

    #[test]
    fn same_dir() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/index.html");
        let link = "somefile.html";
        let expected = "file:///best/root/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn relative_to_below() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/subdir/index.md");
        let link = "../somefile.html";
        let expected = "file:///best/root/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn relative_to_sibling() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/subdir/index.md");
        let link = "../sibling/somefile.html";
        let expected = "file:///best/root/sibling/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn relative_to_siblings_child() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/subdir/index.md");
        let link = "../sibling/siblings_child/somefile.html";
        let expected = "file:///best/root/sibling/siblings_child/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn absolute_link_root() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/index.md");
        let link = "/somefile.html";
        let expected = "file:///best/root/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn absolute_link_to_parent() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/subdir/index.md");
        let link = "/subdir/somefile.html";
        let expected = "file:///best/root/subdir/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn absolute_link_to_child() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/index.md");
        let link = "/subdir/somefile.html";
        let expected = "file:///best/root/subdir/somefile.html";
        t(expected, path, root, link);
    }

    #[test]
    fn with_fragment() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/index.md");
        let link = "somefile.html#some-fragment";
        let expected = "file:///best/root/somefile.html#some-fragment";
        t(expected, path, root, link);
    }

    #[test]
    fn only_fragment() {
        let root = Path::new("/best/root");
        let path = Path::new("/best/root/index.md");
        let link = "#some-fragment";
        let expected = "file:///best/root/index.md#some-fragment";
        t(expected, path, root, link);
    }
}
