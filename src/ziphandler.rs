use std::fs::File;

use zip::ZipArchive;

struct ZipEntry {
    archive_name: String,
    entry_name: String,
}

fn list_docx_files_in_zip(zip_path: &str) -> anyhow::Result<Vec<ZipEntry>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut docx_files = Vec::<ZipEntry>::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let file_name = file.name();

        if file_name.ends_with(".docx") {
            let zip_entry = ZipEntry {
                archive_name: zip_path.to_string(),
                entry_name: file_name.to_string(),
            };
            docx_files.push(zip_entry);
        }
    }

    Ok(docx_files)
}
#[cfg(test)]

mod tests {
    use super::*;
    use std::io::Write;
    // use std::io::{self, Read};

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Test that `list_docx_files_in_zip` returns a list of zip entries whose names end with ".docx".
    ///
    /// This test creates a temporary zip file containing three files: "test1.docx", "test2.txt", and
    /// "test3.docx". It then calls `list_docx_files_in_zip` to get a list of zip entries, and checks
    /// that the returned list contains only the two docx files.
    #[test]
    fn test_list_docx_files_in_zip() -> anyhow::Result<()> {
        let dir = tempdir()?;
        let zip_path = dir.path().join("test.zip");
        let file = File::create(&zip_path)?;
        let mut zip = ZipWriter::new(file);

        let options =
            SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

        zip.start_file("test1.docx", options)?;
        zip.write_all(b"Hello, world!")?;

        zip.start_file("test2.txt", options)?;
        zip.write_all(b"Hello, world!")?;

        zip.start_file("test3.docx", options)?;
        zip.write_all(b"Hello, world!")?;

        zip.finish()?;

        let docx_files = list_docx_files_in_zip(zip_path.to_str().unwrap())?;

        assert_eq!(docx_files.len(), 2);
        assert_eq!(docx_files[0].entry_name, "test1.docx");
        assert_eq!(docx_files[1].entry_name, "test3.docx");

        Ok(())
    }
}
