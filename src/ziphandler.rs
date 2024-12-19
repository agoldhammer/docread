use std::fs::File;
use std::io::{self, Read};
use zip::ZipArchive;

fn list_docx_files_in_zip(zip_path: &str) -> io::Result<Vec<String>> {
    let file = File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;
    let mut docx_files = Vec::new();

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        let file_name = file.name();

        if file_name.ends_with(".docx") {
            docx_files.push(file_name.to_string());
        }
    }

    Ok(docx_files)
}
