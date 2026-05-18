//! file.rs - Reading and parsing .md files, and finding the links they contain

use std::path::PathBuf;

pub struct File {
    pub path: PathBuf,
    pub links: Vec<MdLink>,
}

/// A link in a markdown file. This includes the entire link.
///
/// ```not_rust
///  --title--  --url---
/// [link text](link url)
/// ```
///
/// Notice that link references, written with only the square brackets, and defined
/// later, is *not* supported. Link references looks like `[text]`.
pub struct MdLink {
    /// The url of the link.
    pub url: String,
    /// The title of the link, or `None` if it's a bare link.
    pub title: Option<String>,
    /// The line (1-based) in the file the link starts at
    pub lineno: usize,
    /// The column (1-based) on the line where the link starts
    pub colno: usize,
    /// The line (1-based) where the link ends
    pub endlineno: usize,
    /// The column (1-based) where the link ends
    pub endcolno: usize,
}

pub fn mdast_find_links(root_node: &markdown::mdast::Node) -> Vec<MdLink> {
    assert!(matches!(root_node, markdown::mdast::Node::Root(_)));
    let mut links = vec![];
    _mdast_find_links(root_node, &mut links);
    links
}

fn _mdast_find_links(node: &markdown::mdast::Node, links: &mut Vec<MdLink>) {
    use markdown::mdast::{Link, Node};
    if let Node::Link(Link {
        url,
        title,
        position,
        ..
    }) = node
    {
        let pos = position
            .as_ref()
            .expect("all links in an md document has a position");
        links.push(MdLink {
            url: url.to_string(),
            title: title.clone(),
            lineno: pos.start.line,
            colno: pos.start.column,
            endlineno: pos.end.line,
            endcolno: pos.end.column,
        });
    }

    if let Some(children) = node.children() {
        for child in children.iter() {
            _mdast_find_links(child, links);
        }
    }
}

pub fn parse_md(content: &str) -> Option<markdown::mdast::Node> {
    let default_opts = markdown::ParseOptions::default();
    match markdown::to_mdast(content, &default_opts) {
        Ok(node) => Some(node),
        Err(msg) => {
            eprintln!("markdown parse error: {msg}");
            None
        }
    }
}
