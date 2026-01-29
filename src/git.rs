use std::path::{Path, PathBuf};

/// Run `git check-ignore` with `files` as the input. Returns the stdout of
/// `git check-ignore`.
pub fn git_check_ignore(
    files: &[PathBuf],
    git_work_tree: &Path,
    verbose: bool,
) -> Result<Vec<PathBuf>, std::io::Error> {
    let mut child = std::process::Command::new("git")
        .arg("check-ignore")
        .arg("--stdin")
        .env("GIT_WORK_TREE", git_work_tree)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    let mut stdin = child.stdin.take().expect("we captured stdin");
    let mut stdout = child.stdout.take().expect("we captured stdout");
    let data = files
        .iter()
        .map(|pathbuf| pathbuf.to_str().expect("all paths are valid unicode"))
        .collect::<Vec<&str>>()
        .join("\n");

    let t0 = std::time::Instant::now();
    let writer_thread = std::thread::spawn(move || {
        use std::io::Write;

        let vec = data.split("\n").collect::<Vec<_>>().join("\n");
        stdin
            .write_all(vec.as_bytes())
            .expect("can write to stdin of child");
    });

    let reader_thread = std::thread::spawn(move || {
        use std::io::Read;
        let mut output = String::new();
        stdout
            .read_to_string(&mut output)
            .expect("can read stdout from child");
        output
    });

    writer_thread.join().expect("writing thread finished");
    let stdout = reader_thread.join().expect("reading thread finished");

    match child.wait()?.code() {
        Some(0) => {
            if verbose {
                eprintln!(
                    "'git check-ignore' child exited succesfully. read {} bytes, took {:?}",
                    stdout.len(),
                    t0.elapsed()
                );
            }
            Ok(stdout.split("\n").map(|s| PathBuf::from(s)).collect())
        }
        Some(_) => Err(std::io::Error::other(
            "child exited with non-zero exit status",
        )),
        None => Err(std::io::Error::other("child exited unexpectedly by signal")),
    }
}
