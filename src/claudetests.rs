use super::*;
use regex::Regex;
use std::io::{Error, ErrorKind};

// Mock implementation of ReadIntoBuf for testing
struct MockFileReader {
    content: Vec<u8>,
    should_fail: bool,
}

impl MockFileReader {
    fn new(content: Vec<u8>, should_fail: bool) -> Self {
        Self {
            content,
            should_fail,
        }
    }
}

impl ReadIntoBuf for MockFileReader {
    fn read_into_buf(&self) -> anyhow::Result<Vec<u8>> {
        if self.should_fail {
            Err(Error::new(ErrorKind::Other, "Mock read error").into())
        } else {
            Ok(self.content.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a mock DOCX JSON content
    fn create_mock_docx_json(text: &str) -> String {
        format!(
            r#"{{
            "document": {{
                "body": {{
                    "paragraphs": [
                        {{
                            "runs": [
                                {{
                                    "text": "{}"
                                }}
                            ]
                        }}
                    ]
                }}
            }}
        }}"#,
            text
        )
    }

    #[test]
    fn test_successful_parse_with_matches() -> anyhow::Result<()> {
        // Arrange
        let mock_content = create_mock_docx_json("Hello world! This is a test document.");
        let file_reader = Box::new(MockFileReader::new(mock_content.as_bytes().to_vec(), false));
        let search_re = Regex::new(r"test")?;

        // Act
        let result = parse_docx(&file_reader, &search_re)?;

        // Assert
        assert!(!result.is_empty());
        assert_eq!(result[0].text, "test");
        Ok(())
    }

    #[test]
    fn test_successful_parse_no_matches() -> anyhow::Result<()> {
        // Arrange
        let mock_content = create_mock_docx_json("Hello world!");
        let file_reader = Box::new(MockFileReader::new(mock_content.as_bytes().to_vec(), false));
        let search_re = Regex::new(r"test")?;

        // Act
        let result = parse_docx(&file_reader, &search_re)?;

        // Assert
        assert!(result.is_empty());
        Ok(())
    }

    #[test]
    fn test_failed_file_read() {
        // Arrange
        let file_reader = Box::new(MockFileReader::new(vec![], true));
        let search_re = Regex::new(r"test").unwrap();

        // Act
        let result = parse_docx(&file_reader, &search_re);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_json() {
        // Arrange
        let invalid_json = "invalid json content".as_bytes().to_vec();
        let file_reader = Box::new(MockFileReader::new(invalid_json, false));
        let search_re = Regex::new(r"test").unwrap();

        // Act
        let result = parse_docx(&file_reader, &search_re);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn test_multiple_matches() -> anyhow::Result<()> {
        // Arrange
        let mock_content = create_mock_docx_json("Test one test two TEST three");
        let file_reader = Box::new(MockFileReader::new(mock_content.as_bytes().to_vec(), false));
        let search_re = Regex::new(r"(?i)test")?; // Case-insensitive match

        // Act
        let result = parse_docx(&file_reader, &search_re)?;

        // Assert
        assert_eq!(result.len(), 3);
        assert!(result.iter().any(|run| run.text.contains("Test")));
        assert!(result.iter().any(|run| run.text.contains("test")));
        assert!(result.iter().any(|run| run.text.contains("TEST")));
        Ok(())
    }
}
