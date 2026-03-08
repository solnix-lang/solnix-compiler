

use std::collections::HashMap;
use std::ops::Range;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);


#[derive(Debug, Clone)]
pub struct Span {
    pub file: FileId,
    pub range: Range<usize>,
}

pub struct SourceFile {
    pub name: String,
    pub content: String,
}

pub struct SourceManager {
    files: HashMap<FileId, SourceFile>,
    next_id: u32,
}

impl SourceManager {
    /// Create a new source manager
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_id: 0,
        }
    }

    /// Add a new source file and return its ID
    pub fn add_file(&mut self, name: String, content: String) -> FileId {
        let id = FileId(self.next_id);
        self.next_id += 1;
        self.files.insert(id, SourceFile { name, content });
        id
    }

    /// Get a source file by ID
    pub fn get(&self, id: FileId) -> Option<&SourceFile> {
        self.files.get(&id)
    }

    /// Get the file name for a given FileId
    pub fn file_name(&self, id: FileId) -> Option<&str> {
        self.files.get(&id).map(|f| f.name.as_str())
    }

    /// Get the file content for a given FileId
    pub fn file_content(&self, id: FileId) -> Option<&str> {
        self.files.get(&id).map(|f| f.content.as_str())
    }

    /// Get line and column information for a byte offset
    pub fn line_col(&self, file: FileId, byte_offset: usize) -> Option<(usize, usize)> {
        let content = self.file_content(file)?;

        let mut line = 1;
        let mut col = 1;

        for (i, ch) in content.chars().enumerate() {
            if i >= byte_offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }

        Some((line, col))
    }
}

impl Span {
    pub fn new(file: FileId, range: Range<usize>) -> Self {
        Self { file, range }
    }
    pub fn to_source_span(&self) -> miette::SourceSpan {
        (self.range.start..self.range.end).into()
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}