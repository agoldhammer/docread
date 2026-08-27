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

    #[test]
    fn test_make_fnames_no_matches_returns_empty() -> anyhow::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("notes.txt"))?;

        let f = make_fnames(dir.path().to_str().unwrap(), ".docx")?;
        assert!(f.fnames.is_empty());
        Ok(())
    }

    #[test]
    fn test_make_fnames_missing_dir_is_empty_not_error() -> anyhow::Result<()> {
        let f = make_fnames("/definitely/not/a/real/dir", ".docx")?;
        assert!(f.fnames.is_empty());
        Ok(())
    }

    #[test]
    fn test_make_fnames_trailing_slash_matches_without() -> anyhow::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("a.docx"))?;
        let base = dir.path().to_str().unwrap();

        let plain = make_fnames(base, ".docx")?.fnames;
        let slashed = make_fnames(&format!("{base}/"), ".docx")?.fnames;
        assert_eq!(plain, slashed);
        assert_eq!(plain.len(), 1);
        Ok(())
    }

    #[test]
    fn test_make_fnames_filters_by_suffix_only() -> anyhow::Result<()> {
        let dir = tempdir()?;
        File::create(dir.path().join("a.docx"))?;
        File::create(dir.path().join("b.docx.txt"))?;
        File::create(dir.path().join("c.docx2"))?;
        File::create(dir.path().join("d.DOCX"))?;

        let f = make_fnames(dir.path().to_str().unwrap(), ".docx")?;
        assert_eq!(f.fnames.len(), 1);
        assert!(f.fnames[0].ends_with("a.docx"));
        Ok(())
    }

    #[test]
    fn test_make_fnames_invalid_pattern_is_error() {
        // "[**/*.docx" contains an unclosed character class, which the glob crate rejects.
        assert!(make_fnames("[", ".docx").is_err());
    }
}
