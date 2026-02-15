use std::path::Path;
use std::process::Command;

use anyhow::{Context, Result};

use super::{DiffLine, FileDiff, FileStatus, Hunk, LineOrigin};

pub(crate) fn diff_commit(workdir: &Path, oid: &str) -> Result<Vec<FileDiff>> {
    anyhow::ensure!(
        oid.bytes().all(|b| b.is_ascii_hexdigit()),
        "invalid commit OID: {oid}"
    );

    let output = Command::new("git")
        .args(["diff-tree", "-p", "--no-commit-id", "-M", oid])
        .current_dir(workdir)
        .output()
        .context("failed to run git diff-tree")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("git diff-tree failed: {}", stderr.trim());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // If output is empty (root commit has no parent), retry with --root
    if stdout.trim().is_empty() {
        let output = Command::new("git")
            .args(["diff-tree", "-p", "--no-commit-id", "--root", "-M", oid])
            .current_dir(workdir)
            .output()
            .context("failed to run git diff-tree --root")?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("git diff-tree --root failed: {}", stderr.trim());
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        return parse_unified_diff(&stdout);
    }

    parse_unified_diff(&stdout)
}

pub fn parse_unified_diff(input: &str) -> Result<Vec<FileDiff>> {
    let mut files = Vec::new();
    let mut lines = input.lines().peekable();

    while let Some(line) = lines.peek() {
        if !line.starts_with("diff --git") {
            lines.next();
            continue;
        }

        // Parse file header
        let diff_line = lines.next().unwrap();
        let (path, status) = parse_diff_header(diff_line);

        // Skip extended header lines (index, old mode, new mode, etc.)
        let mut file_status = status;
        let mut old_path: Option<String> = None;
        while let Some(line) = lines.peek() {
            if line.starts_with("---") || line.starts_with("diff --git") || line.starts_with("@@") {
                break;
            }
            let header_line = lines.next().unwrap();
            if header_line.starts_with("new file") {
                file_status = FileStatus::Added;
            } else if header_line.starts_with("deleted file") {
                file_status = FileStatus::Deleted;
            } else if let Some(from_path) = header_line.strip_prefix("rename from ") {
                file_status = FileStatus::Renamed;
                old_path = Some(from_path.to_string());
            } else if header_line.starts_with("rename to") {
                file_status = FileStatus::Renamed;
            }
        }

        // Skip --- and +++ lines
        if lines.peek().is_some_and(|l| l.starts_with("---")) {
            lines.next();
        }
        if lines.peek().is_some_and(|l| l.starts_with("+++")) {
            lines.next();
        }

        // Parse hunks
        let mut hunks = Vec::new();
        while let Some(line) = lines.peek() {
            if line.starts_with("diff --git") {
                break;
            }
            if line.starts_with("@@") {
                let hunk = parse_hunk(&mut lines);
                hunks.push(hunk);
            } else {
                lines.next();
            }
        }

        files.push(FileDiff {
            path,
            old_path,
            status: file_status,
            hunks,
        });
    }

    Ok(files)
}

fn parse_diff_header(line: &str) -> (String, FileStatus) {
    // "diff --git a/path b/path"
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() >= 4 {
        let b_path = parts[3].strip_prefix("b/").unwrap_or(parts[3]);
        (b_path.to_string(), FileStatus::Modified)
    } else {
        ("unknown".to_string(), FileStatus::Modified)
    }
}

fn parse_hunk(lines: &mut std::iter::Peekable<std::str::Lines<'_>>) -> Hunk {
    let header_line = lines.next().unwrap_or_default();
    let (old_start, old_count, new_start, new_count) = parse_hunk_header(header_line);

    let mut old_line = old_start;
    let mut new_line = new_start;
    let mut hunk_lines = Vec::new();
    while let Some(line) = lines.peek() {
        if line.starts_with("@@") || line.starts_with("diff --git") {
            break;
        }
        let line = lines.next().unwrap();
        if let Some(content) = line.strip_prefix('+') {
            hunk_lines.push(DiffLine {
                origin: LineOrigin::Addition,
                content: content.to_string(),
                old_line_no: None,
                new_line_no: Some(new_line),
                change_spans: Vec::new(),
            });
            new_line += 1;
        } else if let Some(content) = line.strip_prefix('-') {
            hunk_lines.push(DiffLine {
                origin: LineOrigin::Deletion,
                content: content.to_string(),
                old_line_no: Some(old_line),
                new_line_no: None,
                change_spans: Vec::new(),
            });
            old_line += 1;
        } else if let Some(content) = line.strip_prefix(' ') {
            hunk_lines.push(DiffLine {
                origin: LineOrigin::Context,
                content: content.to_string(),
                old_line_no: Some(old_line),
                new_line_no: Some(new_line),
                change_spans: Vec::new(),
            });
            old_line += 1;
            new_line += 1;
        } else if line.starts_with('\\') {
            // "\ No newline at end of file"
            continue;
        } else {
            hunk_lines.push(DiffLine {
                origin: LineOrigin::Context,
                content: line.to_string(),
                old_line_no: Some(old_line),
                new_line_no: Some(new_line),
                change_spans: Vec::new(),
            });
            old_line += 1;
            new_line += 1;
        }
    }

    Hunk {
        header: header_line.to_string(),
        old_start,
        old_count,
        new_start,
        new_count,
        lines: hunk_lines,
    }
}

fn parse_hunk_header(header: &str) -> (u32, u32, u32, u32) {
    // "@@ -old_start,old_count +new_start,new_count @@"
    let header = header.trim();
    let parts: Vec<&str> = header.split_whitespace().collect();
    if parts.len() < 3 {
        return (0, 0, 0, 0);
    }

    let old = parts[1].strip_prefix('-').unwrap_or(parts[1]);
    let new = parts[2].strip_prefix('+').unwrap_or(parts[2]);

    let (old_start, old_count) = parse_range(old);
    let (new_start, new_count) = parse_range(new);

    (old_start, old_count, new_start, new_count)
}

fn parse_range(range: &str) -> (u32, u32) {
    let parts: Vec<&str> = range.split(',').collect();
    let start = parts[0].parse().unwrap_or(0);
    let count = if parts.len() > 1 {
        parts[1].parse().unwrap_or(0)
    } else {
        1
    };
    (start, count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hunk_header() {
        let (os, oc, ns, nc) = parse_hunk_header("@@ -1,3 +1,4 @@ fn main()");
        assert_eq!((os, oc, ns, nc), (1, 3, 1, 4));
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        let (os, oc, ns, nc) = parse_hunk_header("@@ -0,0 +1 @@");
        assert_eq!((os, oc, ns, nc), (0, 0, 1, 1));
    }

    #[test]
    fn test_parse_unified_diff() {
        let diff = "diff --git a/file.txt b/file.txt\n\
index abc..def 100644\n\
--- a/file.txt\n\
+++ b/file.txt\n\
@@ -1,3 +1,3 @@\n\
 line1\n\
-old line\n\
+new line\n\
 line3";
        let files = parse_unified_diff(diff).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "file.txt");
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[0].hunks.len(), 1);

        let hunk = &files[0].hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.old_count, 3);
        assert_eq!(hunk.new_start, 1);
        assert_eq!(hunk.new_count, 3);
        assert_eq!(hunk.lines.len(), 4);
        assert_eq!(hunk.lines[0].origin, LineOrigin::Context);
        assert_eq!(hunk.lines[1].origin, LineOrigin::Deletion);
        assert_eq!(hunk.lines[2].origin, LineOrigin::Addition);
        assert_eq!(hunk.lines[3].origin, LineOrigin::Context);

        // Verify line numbers computed during parsing
        assert_eq!(hunk.lines[0].old_line_no, Some(1));
        assert_eq!(hunk.lines[0].new_line_no, Some(1));
        assert_eq!(hunk.lines[1].old_line_no, Some(2)); // deletion
        assert_eq!(hunk.lines[1].new_line_no, None);
        assert_eq!(hunk.lines[2].old_line_no, None); // addition
        assert_eq!(hunk.lines[2].new_line_no, Some(2));
        assert_eq!(hunk.lines[3].old_line_no, Some(3));
        assert_eq!(hunk.lines[3].new_line_no, Some(3));

        // change_spans should be empty (populated later by inline diff)
        assert!(hunk.lines.iter().all(|l| l.change_spans.is_empty()));
    }

    #[test]
    fn test_parse_new_file_diff() {
        let diff = "\
diff --git a/new.txt b/new.txt
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new.txt
@@ -0,0 +1,2 @@
+hello
+world
";
        let files = parse_unified_diff(diff).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Added);
        assert_eq!(files[0].hunks[0].lines.len(), 2);
        assert!(files[0].hunks[0]
            .lines
            .iter()
            .all(|l| l.origin == LineOrigin::Addition));
    }

    #[test]
    fn test_parse_deleted_file_diff() {
        let diff = "\
diff --git a/old.txt b/old.txt
deleted file mode 100644
index abc1234..0000000
--- a/old.txt
+++ /dev/null
@@ -1,2 +0,0 @@
-hello
-world
";
        let files = parse_unified_diff(diff).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Deleted);
        assert_eq!(files[0].path, "old.txt");
        assert!(files[0].hunks[0]
            .lines
            .iter()
            .all(|l| l.origin == LineOrigin::Deletion));
    }

    #[test]
    fn test_parse_renamed_file_diff() {
        let diff = "\
diff --git a/old_name.txt b/new_name.txt
similarity index 100%
rename from old_name.txt
rename to new_name.txt
";
        let files = parse_unified_diff(diff).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].status, FileStatus::Renamed);
        assert_eq!(files[0].path, "new_name.txt");
        assert_eq!(files[0].old_path.as_deref(), Some("old_name.txt"));
        assert!(files[0].hunks.is_empty());
    }

    #[test]
    fn test_parse_empty_diff() {
        let files = parse_unified_diff("").unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn test_parse_multi_file_diff() {
        let diff = "\
diff --git a/a.txt b/a.txt
index abc..def 100644
--- a/a.txt
+++ b/a.txt
@@ -1 +1 @@
-old a
+new a
diff --git a/b.txt b/b.txt
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/b.txt
@@ -0,0 +1 @@
+new b
";
        let files = parse_unified_diff(diff).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "a.txt");
        assert_eq!(files[0].status, FileStatus::Modified);
        assert_eq!(files[1].path, "b.txt");
        assert_eq!(files[1].status, FileStatus::Added);
    }
}
