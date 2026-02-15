use similar::{Algorithm, ChangeTag, TextDiff};

use super::{Hunk, InlineSpan, LineOrigin};

/// Walk each hunk and compute word-level inline change spans for paired
/// deletion/addition runs. Unpaired lines keep empty `change_spans`.
pub fn compute_inline_changes(hunks: &mut [Hunk]) {
    for hunk in hunks.iter_mut() {
        let lines = &mut hunk.lines;
        let len = lines.len();
        let mut i = 0;

        while i < len {
            // Find a contiguous run of deletions
            let del_start = i;
            while i < len && lines[i].origin == LineOrigin::Deletion {
                i += 1;
            }
            let del_end = i;

            // Find an immediately following contiguous run of additions
            let add_start = i;
            while i < len && lines[i].origin == LineOrigin::Addition {
                i += 1;
            }
            let add_end = i;

            let del_count = del_end - del_start;
            let add_count = add_end - add_start;

            if del_count == 0 || add_count == 0 {
                // No pairable run — skip non-deletion/addition lines
                if del_count == 0 && add_count == 0 {
                    i += 1;
                }
                continue;
            }

            // Pair deletions with additions 1:1 up to min(del_count, add_count)
            let pairs = del_count.min(add_count);
            for p in 0..pairs {
                let del_idx = del_start + p;
                let add_idx = add_start + p;
                let (del_spans, add_spans) =
                    word_diff(&lines[del_idx].content, &lines[add_idx].content);
                lines[del_idx].change_spans = del_spans;
                lines[add_idx].change_spans = add_spans;
            }
        }
    }
}

/// Compute word-level diff between two lines, returning byte-offset spans of
/// changed regions for the old and new content respectively.
fn word_diff(old: &str, new: &str) -> (Vec<InlineSpan>, Vec<InlineSpan>) {
    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_words(old, new);

    let mut old_spans = Vec::new();
    let mut new_spans = Vec::new();

    for change in diff.iter_all_changes() {
        let value = change.value();
        match change.tag() {
            ChangeTag::Delete => {
                let range = byte_range_in(old, value);
                debug_assert!(range.is_some(), "similar returned non-sub-slice for delete");
                if let Some(range) = range {
                    old_spans.push(InlineSpan {
                        start: range.0,
                        end: range.1,
                    });
                }
            }
            ChangeTag::Insert => {
                let range = byte_range_in(new, value);
                debug_assert!(range.is_some(), "similar returned non-sub-slice for insert");
                if let Some(range) = range {
                    new_spans.push(InlineSpan {
                        start: range.0,
                        end: range.1,
                    });
                }
            }
            ChangeTag::Equal => {}
        }
    }

    (old_spans, new_spans)
}

/// Compute the byte offset range of `substr` within `source` using pointer
/// arithmetic. Returns `None` if `substr` is not a sub-slice of `source`.
fn byte_range_in(source: &str, substr: &str) -> Option<(usize, usize)> {
    let source_start = source.as_ptr() as usize;
    let source_end = source_start + source.len();
    let sub_start = substr.as_ptr() as usize;
    let sub_end = sub_start + substr.len();

    if sub_start >= source_start && sub_end <= source_end {
        let offset = sub_start - source_start;
        Some((offset, offset + substr.len()))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffLine, Hunk, LineOrigin};

    fn make_line(origin: LineOrigin, content: &str) -> DiffLine {
        DiffLine {
            origin,
            content: content.to_string(),
            old_line_no: None,
            new_line_no: None,
            change_spans: Vec::new(),
        }
    }

    #[test]
    fn test_word_diff_single_word_change() {
        let (old_spans, new_spans) = word_diff("hello world", "hello earth");
        // "world" changed to "earth"
        assert_eq!(old_spans.len(), 1);
        assert_eq!(
            &"hello world"[old_spans[0].start..old_spans[0].end],
            "world"
        );
        assert_eq!(new_spans.len(), 1);
        assert_eq!(
            &"hello earth"[new_spans[0].start..new_spans[0].end],
            "earth"
        );
    }

    #[test]
    fn test_word_diff_appended_word() {
        let (old_spans, new_spans) = word_diff("hello", "hello world");
        // old has no change spans (nothing was removed)
        assert!(old_spans.is_empty());
        // new has inserted spans covering " world"
        assert!(!new_spans.is_empty());
        // Concatenate all inserted spans to verify the full appended text
        let inserted: String = new_spans
            .iter()
            .map(|s| &"hello world"[s.start..s.end])
            .collect();
        assert_eq!(inserted, " world");
    }

    #[test]
    fn test_word_diff_identical_lines() {
        let (old_spans, new_spans) = word_diff("same content", "same content");
        assert!(old_spans.is_empty());
        assert!(new_spans.is_empty());
    }

    #[test]
    fn test_word_diff_completely_different() {
        let (old_spans, new_spans) = word_diff("foo bar", "baz qux");
        assert!(!old_spans.is_empty());
        assert!(!new_spans.is_empty());
    }

    #[test]
    fn test_compute_inline_changes_paired_lines() {
        let mut hunks = vec![Hunk {
            header: "@@ -1,3 +1,3 @@".into(),
            old_start: 1,
            old_count: 3,
            new_start: 1,
            new_count: 3,
            lines: vec![
                make_line(LineOrigin::Context, "unchanged"),
                make_line(LineOrigin::Deletion, "    println!(\"hello\");"),
                make_line(LineOrigin::Addition, "    println!(\"hello world\");"),
                make_line(LineOrigin::Context, "unchanged end"),
            ],
        }];

        compute_inline_changes(&mut hunks);

        // Context lines should have empty spans
        assert!(hunks[0].lines[0].change_spans.is_empty());
        assert!(hunks[0].lines[3].change_spans.is_empty());

        // Deletion line should have spans marking "hello"→"hello world" difference
        let del_spans = &hunks[0].lines[1].change_spans;
        assert!(
            !del_spans.is_empty(),
            "deletion line should have change spans"
        );

        // Addition line should have spans
        let add_spans = &hunks[0].lines[2].change_spans;
        assert!(
            !add_spans.is_empty(),
            "addition line should have change spans"
        );
    }

    #[test]
    fn test_compute_inline_changes_unpaired_additions() {
        let mut hunks = vec![Hunk {
            header: "@@ -1,1 +1,3 @@".into(),
            old_start: 1,
            old_count: 1,
            new_start: 1,
            new_count: 3,
            lines: vec![
                make_line(LineOrigin::Deletion, "old line"),
                make_line(LineOrigin::Addition, "new line 1"),
                make_line(LineOrigin::Addition, "new line 2"),
                make_line(LineOrigin::Addition, "new line 3"),
            ],
        }];

        compute_inline_changes(&mut hunks);

        // First pair (del[0] + add[0]) should have spans
        assert!(!hunks[0].lines[0].change_spans.is_empty());
        assert!(!hunks[0].lines[1].change_spans.is_empty());

        // Unpaired additions (add[1], add[2]) should have empty spans
        assert!(hunks[0].lines[2].change_spans.is_empty());
        assert!(hunks[0].lines[3].change_spans.is_empty());
    }

    #[test]
    fn test_compute_inline_changes_only_additions() {
        let mut hunks = vec![Hunk {
            header: "@@ -0,0 +1,2 @@".into(),
            old_start: 0,
            old_count: 0,
            new_start: 1,
            new_count: 2,
            lines: vec![
                make_line(LineOrigin::Addition, "new line 1"),
                make_line(LineOrigin::Addition, "new line 2"),
            ],
        }];

        compute_inline_changes(&mut hunks);

        // No paired deletions, so additions should have empty spans
        assert!(hunks[0].lines[0].change_spans.is_empty());
        assert!(hunks[0].lines[1].change_spans.is_empty());
    }

    #[test]
    fn test_byte_range_in_basic() {
        let s = "hello world";
        let sub = &s[6..]; // "world"
        let range = byte_range_in(s, sub).unwrap();
        assert_eq!(range, (6, 11));
    }
}
