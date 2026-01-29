//! file.rs - Reading and parsing .md files, and finding the links they contain

use std::path::{Path, PathBuf};

/// A link in a file.
#[derive(Debug)]
pub struct Link {
    /// Which column index the text of the link starts at
    pub text_start: usize,
    /// Which column index the text of the link ends at
    pub text_end: usize,
    /// Which column index the link of the link starts at
    pub url_start: usize,
    /// Which column index the link of the link ends at
    pub url_end: usize,
}

pub struct Line {
    /// Line number, starting from 1
    pub lineno: usize,
    pub string: String,
    pub links: Vec<Link>,
}

pub struct File {
    pub path: PathBuf,
    pub lines: Vec<Line>,
}

impl File {
    pub fn read<P: AsRef<Path>>(path: P, re: &regex::Regex) -> Result<File, std::io::Error> {
        use std::io::{BufRead, BufReader};
        let fp = std::fs::OpenOptions::new().read(true).open(path.as_ref())?;
        let mut reader = BufReader::new(fp);
        let mut lines = vec![];
        let mut lineno = 1usize;
        loop {
            let mut string = String::new();
            match reader.read_line(&mut string) {
                Ok(0) => break,
                Ok(_n_bytes_read) => {}
                Err(error) => return Err(error),
            }
            let links = find_links(&string, re);
            lines.push(Line {
                lineno,
                string,
                links,
            });
            lineno += 1;
        }

        Ok(File {
            path: path.as_ref().to_owned(),
            lines,
        })
    }

    pub fn path_as_str(&self) -> &str {
        self.path.to_str().expect("all file paths are valid utf-8")
    }
}

impl Link {
    fn new(text_start: usize, text_end: usize, url_start: usize, url_end: usize) -> Self {
        Self {
            text_start: text_start,
            text_end: text_end,
            url_start,
            url_end,
        }
    }

    pub fn text<'a>(&self, line: &'a str) -> &'a str {
        &line[self.text_start..self.text_end]
    }

    pub fn url<'a>(&self, line: &'a str) -> &'a str {
        &line[self.url_start..self.url_end]
    }
}

fn find_links(line: &str, re: &regex::Regex) -> Vec<Link> {
    let mut links = vec![];
    let mut locs = re.capture_locations();
    let mut col = 0usize;
    while re.captures_read_at(&mut locs, line, col).is_some() {
        let (text_s, text_e) = locs.get(1).expect("we have 2 capture groups");
        let (url_s, url_e) = locs.get(2).expect("we have 2 capture groups");
        links.push(Link::new(text_s, text_e, url_s, url_e));
        col = url_e;
    }
    links
}
