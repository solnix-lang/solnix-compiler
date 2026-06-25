use std::collections::HashMap;
use std::ops::Range;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FileId(pub u32);

#[derive(Debug, Clone)]
pub struct Span {
    pub file: FileId,
    pub range: Range<usize>,
}

pub struct SourceFile {
    pub name: String,
    pub content: Arc<String>,
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
        self.files.insert(
            id,
            SourceFile {
                name,
                content: Arc::new(content),
            },
        );
        id
    }

    pub fn get_named_source(
        &self,
        id: FileId,
    ) -> Option<miette::NamedSource<std::sync::Arc<String>>> {
        self.files
            .get(&id)
            .map(|f| miette::NamedSource::new(f.name.clone(), f.content.clone()))
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
