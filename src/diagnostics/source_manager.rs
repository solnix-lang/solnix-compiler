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
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
            next_id: 0,
        }
    }

    pub fn add_file(&mut self, name: String, content: String) -> FileId {
        let id = FileId(self.next_id);
        self.next_id += 1;

        self.files.insert(id, SourceFile { name, content });

        id
    }

    pub fn file_content(&self, id: FileId) -> Option<&str> {
        self.files.get(&id).map(|f| f.content.as_str())
    }

    pub fn file_name(&self, id: FileId) -> Option<&str> {
        self.files.get(&id).map(|f| f.name.as_str())
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