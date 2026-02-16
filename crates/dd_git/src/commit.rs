#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: String,
    pub short_oid: String,
    pub tree_oid: String,
    pub author_name: String,
    pub author_email: String,
    pub date: i64,
    pub committer_name: String,
    pub committer_email: String,
    pub committer_date: i64,
    pub subject: String,
    pub body: String,
    pub parent_oids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SignatureStatus {
    Good,
    Bad,
    Unknown,
    None,
}

impl SignatureStatus {
    pub fn from_git_char(c: char) -> Self {
        match c {
            'G' => Self::Good,
            'B' => Self::Bad,
            'U' | 'X' | 'Y' | 'R' | 'E' => Self::Unknown,
            _ => Self::None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Good => "Valid",
            Self::Bad => "Invalid",
            Self::Unknown => "Unknown",
            Self::None => "None",
        }
    }
}
