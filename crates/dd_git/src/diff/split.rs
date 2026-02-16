use std::sync::Arc;

use super::{DiffLine, LineOrigin};

/// One row in a side-by-side diff view.
///
/// `left` carries the old-file side (context or deletion),
/// `right` carries the new-file side (context or addition).
/// Either side may be `None` when there is no paired line.
#[derive(Debug, Clone)]
pub struct SplitRow {
    pub left: Option<Arc<DiffLine>>,
    pub right: Option<Arc<DiffLine>>,
}

/// Convert a flat slice of diff lines (as produced by a unified diff) into
/// paired [`SplitRow`]s suitable for side-by-side rendering.
///
/// Pairing rules (mirrors the contiguous-run pattern in `inline.rs`):
/// 1. Context line → both sides populated (same line).
/// 2. Contiguous deletions followed by contiguous additions → pair 1:1.
/// 3. Excess deletions (more del than add) → `right: None`.
/// 4. Excess additions (more add than del) → `left: None`.
/// 5. Standalone additions (no preceding deletions) → `left: None`.
/// 6. Standalone deletions (no following additions) → `right: None`.
pub fn split_hunk_lines(lines: &[DiffLine]) -> Vec<SplitRow> {
    let mut rows = Vec::new();
    let len = lines.len();
    let mut i = 0;

    while i < len {
        match lines[i].origin {
            LineOrigin::Context => {
                let line = Arc::new(lines[i].clone());
                rows.push(SplitRow {
                    left: Some(Arc::clone(&line)),
                    right: Some(line),
                });
                i += 1;
            }
            LineOrigin::Deletion => {
                // Collect contiguous deletions
                let del_start = i;
                while i < len && lines[i].origin == LineOrigin::Deletion {
                    i += 1;
                }
                let del_end = i;

                // Collect immediately following contiguous additions
                let add_start = i;
                while i < len && lines[i].origin == LineOrigin::Addition {
                    i += 1;
                }
                let add_end = i;

                let del_count = del_end - del_start;
                let add_count = add_end - add_start;
                let pairs = del_count.min(add_count);

                // Paired lines
                for p in 0..pairs {
                    rows.push(SplitRow {
                        left: Some(Arc::new(lines[del_start + p].clone())),
                        right: Some(Arc::new(lines[add_start + p].clone())),
                    });
                }

                // Excess deletions
                for p in pairs..del_count {
                    rows.push(SplitRow {
                        left: Some(Arc::new(lines[del_start + p].clone())),
                        right: None,
                    });
                }

                // Excess additions
                for p in pairs..add_count {
                    rows.push(SplitRow {
                        left: None,
                        right: Some(Arc::new(lines[add_start + p].clone())),
                    });
                }
            }
            LineOrigin::Addition => {
                // Standalone addition (no preceding deletion)
                rows.push(SplitRow {
                    left: None,
                    right: Some(Arc::new(lines[i].clone())),
                });
                i += 1;
            }
        }
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffLine, LineOrigin};

    fn ctx(content: &str, old: u32, new: u32) -> DiffLine {
        DiffLine {
            origin: LineOrigin::Context,
            content: content.to_string(),
            old_line_no: Some(old),
            new_line_no: Some(new),
            change_spans: Vec::new(),
        }
    }

    fn del(content: &str, old: u32) -> DiffLine {
        DiffLine {
            origin: LineOrigin::Deletion,
            content: content.to_string(),
            old_line_no: Some(old),
            new_line_no: None,
            change_spans: Vec::new(),
        }
    }

    fn add(content: &str, new: u32) -> DiffLine {
        DiffLine {
            origin: LineOrigin::Addition,
            content: content.to_string(),
            old_line_no: None,
            new_line_no: Some(new),
            change_spans: Vec::new(),
        }
    }

    #[test]
    fn test_empty_input() {
        let rows = split_hunk_lines(&[]);
        assert!(rows.is_empty());
    }

    #[test]
    fn test_all_context() {
        let lines = vec![ctx("a", 1, 1), ctx("b", 2, 2), ctx("c", 3, 3)];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 3);
        for row in &rows {
            assert!(row.left.is_some());
            assert!(row.right.is_some());
        }
        assert_eq!(rows[0].left.as_ref().unwrap().content, "a");
        assert_eq!(rows[0].right.as_ref().unwrap().content, "a");
    }

    #[test]
    fn test_equal_del_add() {
        let lines = vec![
            del("old1", 1),
            del("old2", 2),
            add("new1", 1),
            add("new2", 2),
        ];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].left.as_ref().unwrap().content, "old1");
        assert_eq!(rows[0].right.as_ref().unwrap().content, "new1");
        assert_eq!(rows[1].left.as_ref().unwrap().content, "old2");
        assert_eq!(rows[1].right.as_ref().unwrap().content, "new2");
    }

    #[test]
    fn test_more_del_than_add() {
        let lines = vec![
            del("old1", 1),
            del("old2", 2),
            del("old3", 3),
            add("new1", 1),
        ];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 3);
        // First row: paired
        assert_eq!(rows[0].left.as_ref().unwrap().content, "old1");
        assert_eq!(rows[0].right.as_ref().unwrap().content, "new1");
        // Remaining: excess deletions
        assert_eq!(rows[1].left.as_ref().unwrap().content, "old2");
        assert!(rows[1].right.is_none());
        assert_eq!(rows[2].left.as_ref().unwrap().content, "old3");
        assert!(rows[2].right.is_none());
    }

    #[test]
    fn test_more_add_than_del() {
        let lines = vec![
            del("old1", 1),
            add("new1", 1),
            add("new2", 2),
            add("new3", 3),
        ];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 3);
        // First row: paired
        assert_eq!(rows[0].left.as_ref().unwrap().content, "old1");
        assert_eq!(rows[0].right.as_ref().unwrap().content, "new1");
        // Remaining: excess additions
        assert!(rows[1].left.is_none());
        assert_eq!(rows[1].right.as_ref().unwrap().content, "new2");
        assert!(rows[2].left.is_none());
        assert_eq!(rows[2].right.as_ref().unwrap().content, "new3");
    }

    #[test]
    fn test_standalone_addition() {
        let lines = vec![ctx("a", 1, 1), add("inserted", 2), ctx("b", 2, 3)];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 3);
        // Context
        assert!(rows[0].left.is_some());
        assert!(rows[0].right.is_some());
        // Standalone addition
        assert!(rows[1].left.is_none());
        assert_eq!(rows[1].right.as_ref().unwrap().content, "inserted");
        // Context
        assert!(rows[2].left.is_some());
        assert!(rows[2].right.is_some());
    }

    #[test]
    fn test_standalone_deletion() {
        let lines = vec![ctx("a", 1, 1), del("removed", 2), ctx("b", 3, 2)];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 3);
        // Context
        assert!(rows[0].left.is_some());
        assert!(rows[0].right.is_some());
        // Standalone deletion (no following addition)
        assert_eq!(rows[1].left.as_ref().unwrap().content, "removed");
        assert!(rows[1].right.is_none());
        // Context
        assert!(rows[2].left.is_some());
        assert!(rows[2].right.is_some());
    }

    #[test]
    fn test_mixed_sequence() {
        // context, del+add pair, standalone add, context, standalone del
        let lines = vec![
            ctx("line1", 1, 1),
            del("old2", 2),
            add("new2", 2),
            add("extra", 3),
            ctx("line3", 3, 4),
            del("gone", 4),
        ];
        let rows = split_hunk_lines(&lines);

        assert_eq!(rows.len(), 5);

        // Row 0: context
        assert_eq!(rows[0].left.as_ref().unwrap().content, "line1");
        assert_eq!(rows[0].right.as_ref().unwrap().content, "line1");

        // Row 1: paired del+add
        assert_eq!(rows[1].left.as_ref().unwrap().content, "old2");
        assert_eq!(rows[1].right.as_ref().unwrap().content, "new2");

        // Row 2: excess addition
        assert!(rows[2].left.is_none());
        assert_eq!(rows[2].right.as_ref().unwrap().content, "extra");

        // Row 3: context
        assert_eq!(rows[3].left.as_ref().unwrap().content, "line3");
        assert_eq!(rows[3].right.as_ref().unwrap().content, "line3");

        // Row 4: standalone deletion
        assert_eq!(rows[4].left.as_ref().unwrap().content, "gone");
        assert!(rows[4].right.is_none());
    }
}
