use std::fs::File;

use zip::ZipArchive;

#[derive(Debug)]
pub(crate) struct ZipEntry {
    pub(crate) archive_name: String,
    pub(crate) entry_name: String,
}

pub(crate) fn zip_to_zipentries(zip_path: &str) -> anyhow::Result<Vec<ZipEntry>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut zipentries = Vec::<ZipEntry>::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let file_name = file.name();

        if file_name.ends_with(".docx") && !file_name.contains("__MACOSX") {
            let zip_entry = ZipEntry {
                archive_name: zip_path.to_string(),
                entry_name: file_name.to_string(),
            };
            zipentries.push(zip_entry);
        }
    }

    Ok(zipentries)
}
#[cfg(test)]

mod tests {
    use super::*;
    use std::io::Write;
    // use std::io::{self, Read};

    use tempfile::tempdir;
    use zip::write::SimpleFileOptions;
    use zip::ZipWriter;

    /// Test that `zip_to_zipentries` returns a list of zip entries whose names end with ".docx".
    ///
    /// This test creates a temporary zip file containing three files: "test1.docx", "test2.txt", and
    /// "test3.docx". It then calls `zip_to_zipentries` to get a list of zip entries, and checks
    /// that the returned list contains only the two docx files.
    #[test]
    fn test_zip_to_zipentries() -> anyhow::Result<()> {
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

        let docx_files = zip_to_zipentries(zip_path.to_str().unwrap())?;

        assert_eq!(docx_files.len(), 2);
        assert_eq!(docx_files[0].entry_name, "test1.docx");
        assert_eq!(docx_files[1].entry_name, "test3.docx");

        Ok(())
    }

    #[test]
    fn test_read_test_archive() -> anyhow::Result<()> {
        let docx_files = zip_to_zipentries("src/TestArchive.zip")?;
        assert_eq!(docx_files.len(), 2);
        assert_eq!(docx_files[0].entry_name, "BookNotes.docx");
        assert_eq!(docx_files[1].entry_name, "testdoc.docx");
        for ze in docx_files {
            println!("{:?}", ze);
        }
        Ok(())
    }
}
