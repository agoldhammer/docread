use glob::glob;

#[derive(Debug)]
pub(crate) struct Fnames {
    pub fnames: Vec<String>,
}

impl TryFrom<&str> for Fnames {
    type Error = anyhow::Error;
    /// Attempts to create a `Fnames` from a glob pattern. The `glob` crate is used to find all
    /// matching files, and the resulting paths are converted to `String`s and stored in the
    /// `fnames` member of the `Fnames` struct.
    ///
    fn try_from(pattern: &str) -> anyhow::Result<Self> {
        let fpaths = glob(pattern)?;
        let fnames: Vec<String> = fpaths.flatten().map(|p| p.display().to_string()).collect();
        Ok(Fnames { fnames })
    }
}

/// Creates a `Fnames` containing all files in `base_dir` and all of its
/// subdirectories that have the given `suffix`. The `glob` crate is used to
/// find all matching files, and the resulting paths are converted to `String`s
/// and stored in the `fnames` member of the returned `Fnames` struct.
///
/// # Errors
///
/// Will return an error if the glob pattern is invalid. Returns an empty
/// `Fnames` if no files match.
pub fn make_fnames(base_dir: &str, suffix: &str) -> anyhow::Result<Fnames> {
    let mut fpath = base_dir.trim_end_matches("/").to_string();
    let extension = format!("/**/*{}", suffix);
    fpath.push_str(extension.as_str());

    Fnames::try_from(fpath.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_make_fnames_finds_suffix_recursively() -> anyhow::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("one.docx"))?;
        File::create(dir.path().join("notes.txt"))?;
        std::fs::create_dir(dir.path().join("nested"))?;
        File::create(dir.path().join("nested/two.docx"))?;

        let f = make_fnames(dir.path().to_str().unwrap(), ".docx")?;
        assert_eq!(f.fnames.len(), 2);
        Ok(())
    }
}
