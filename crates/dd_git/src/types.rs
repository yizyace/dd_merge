#[derive(Debug, Clone)]
pub struct BranchInfo {
    pub name: String,
    pub is_head: bool,
}

#[derive(Debug, Clone)]
pub struct RemoteInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct TagInfo {
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct StashInfo {
    pub message: String,
}
