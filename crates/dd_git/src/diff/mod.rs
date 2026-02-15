mod parse;

use std::path::Path;

use anyhow::Result;

pub use parse::parse_unified_diff;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LineOrigin {
    Context,
    Addition,
    Deletion,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    /// Byte offset into `DiffLine::content` where the changed region starts.
    pub start: usize,
    /// Byte offset into `DiffLine::content` where the changed region ends.
    pub end: usize,
}

#[derive(Debug, Clone)]
pub struct DiffLine {
    pub origin: LineOrigin,
    pub content: String,
    pub old_line_no: Option<u32>,
    pub new_line_no: Option<u32>,
    /// Byte-offset spans within `content` that were changed (word-level).
    pub change_spans: Vec<InlineSpan>,
}

#[derive(Debug, Clone)]
pub struct Hunk {
    pub header: String,
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileStatus {
    Added,
    Deleted,
    Modified,
    Renamed,
}

#[derive(Debug, Clone)]
pub struct FileDiff {
    pub path: String,
    /// The original path before a rename, if applicable.
    pub old_path: Option<String>,
    pub status: FileStatus,
    pub hunks: Vec<Hunk>,
}

pub(crate) fn diff_commit(workdir: &Path, oid: &str) -> Result<Vec<FileDiff>> {
    parse::diff_commit(workdir, oid)
}
