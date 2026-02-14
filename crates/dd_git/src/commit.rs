#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub oid: String,
    pub short_oid: String,
    pub author_name: String,
    pub author_email: String,
    pub date: i64,
    pub subject: String,
    pub body: String,
    pub parent_oids: Vec<String>,
}
