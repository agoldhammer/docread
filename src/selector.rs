/// Takes a path from the command line, trims any trailing slashes, and appends
/// "/**/*.docx" if the path does not end with .docx or .zip. This is used to
/// convert the user's path into a glob pattern that will match all .docx
/// files in the given directory and its subdirectories.
pub fn make_path(cli_path: &str) -> String {
    let mut path = cli_path.to_string();
    path = path.trim_end_matches('/').to_string();
    path = path.trim_end_matches(".zip").to_string();
    if path.ends_with(".docx") {
        path.clone()
    } else {
        path.push_str("/**/*.docx");
        path.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_path() {
        assert_eq!(make_path("test"), "test/**/*.docx");
        assert_eq!(make_path("test.docx"), "test.docx");
        assert_eq!(make_path("test.zip"), "test/**/*.docx");
        assert_eq!(make_path("test/"), "test/**/*.docx");
        assert_eq!(make_path("."), "./**/*.docx");
    }
}
