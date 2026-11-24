//! Removing `[[rules]]` (or inline `rules = [...]`) entries without losing unrelated comments
//! set off by blank lines, such as section headings or commented-out rules. Comments placed
//! directly above a removed rule with no blank line stay attached to that rule and go with it.

use toml_edit::{Decor, Item, RawString, Table, Value};

use super::Doc;

impl Doc {
    pub fn remove(&mut self, index: usize) {
        enum Removed {
            None,
            Table {
                position: Option<isize>,
                comment: Option<String>,
                emptied: bool,
            },
            Inline {
                comment: Option<String>,
                emptied: bool,
            },
        }
        let removed = match self.doc.get_mut("rules") {
            Some(Item::ArrayOfTables(rules)) if index < rules.len() => {
                let (position, comment) = rules
                    .get(index)
                    .map(|table| {
                        (
                            table.position(),
                            table
                                .decor()
                                .prefix()
                                .and_then(RawString::as_str)
                                .and_then(detached_comments),
                        )
                    })
                    .unwrap_or((None, None));
                rules.remove(index);
                Removed::Table {
                    position,
                    comment,
                    emptied: rules.is_empty(),
                }
            }
            Some(Item::Value(Value::Array(rules))) if index < rules.len() => {
                let comment = rules
                    .get(index)
                    .and_then(|value| value.decor().prefix())
                    .and_then(RawString::as_str)
                    .and_then(detached_comments);
                rules.remove(index);
                if rules.is_empty() {
                    let trailing = rules
                        .trailing()
                        .as_str()
                        .map(trim_trailing_blank_lines)
                        .filter(|text| text.contains('#') && !text.is_empty())
                        .map(|text| {
                            if text.ends_with('\n') {
                                text.to_owned()
                            } else {
                                format!("{text}\n")
                            }
                        });
                    let combined = match (comment, trailing) {
                        (Some(a), Some(b)) => Some(join_comment_trailing(&a, &b)),
                        (Some(a), None) => Some(a),
                        (None, Some(b)) => Some(b),
                        (None, None) => None,
                    };
                    Removed::Inline {
                        comment: combined,
                        emptied: true,
                    }
                } else {
                    if let Some(comment) = comment {
                        if let Some(next) = rules.get_mut(index) {
                            prepend_decor_comment(next.decor_mut(), &comment);
                        } else {
                            let trailing = rules.trailing().as_str().unwrap_or_default().to_owned();
                            rules.set_trailing(join_comment_trailing(&comment, &trailing));
                        }
                    }
                    Removed::Inline {
                        comment: None,
                        emptied: false,
                    }
                }
            }
            _ => Removed::None,
        };
        match removed {
            Removed::Table {
                position,
                comment,
                emptied,
            } => {
                if emptied {
                    self.doc.remove("rules");
                }
                if let Some(comment) = comment {
                    self.splice_table_comment(position, index, &comment);
                }
            }
            Removed::Inline {
                comment,
                emptied: true,
            } => {
                let key_comment = self
                    .doc
                    .key("rules")
                    .and_then(|key| key.leaf_decor().prefix())
                    .and_then(RawString::as_str)
                    .and_then(detached_comments);
                let next_root_key = next_value_key_after(self.doc.as_table(), "rules");
                self.doc.remove("rules");
                let combined = match (key_comment, comment) {
                    (Some(a), Some(b)) => Some(join_comment_prefix(&a, &b)),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                };
                if let Some(comment) = combined {
                    if let Some(next_key) = next_root_key
                        && let Some(mut key) = self.doc.key_mut(&next_key)
                    {
                        prepend_decor_comment(key.leaf_decor_mut(), &comment);
                    } else if let Some(first_pos) = min_table_position(self.doc.as_table(), None)
                        && let Some(target) =
                            find_table_at_position_mut(self.doc.as_table_mut(), first_pos)
                    {
                        prepend_decor_comment(target.decor_mut(), &comment);
                    } else {
                        let trailing = self.doc.trailing().as_str().unwrap_or_default().to_owned();
                        self.doc
                            .set_trailing(join_comment_trailing(&comment, &trailing));
                    }
                }
            }
            Removed::Inline { emptied: false, .. } | Removed::None => {}
        }
    }

    fn splice_table_comment(
        &mut self,
        removed_position: Option<isize>,
        index: usize,
        comment: &str,
    ) {
        if let Some(pos) = removed_position
            && let Some(target_pos) = min_table_position(self.doc.as_table(), Some(pos))
            && let Some(target) = find_table_at_position_mut(self.doc.as_table_mut(), target_pos)
        {
            prepend_decor_comment(target.decor_mut(), comment);
            return;
        }
        if let Some(Item::ArrayOfTables(rules)) = self.doc.get_mut("rules")
            && let Some(next) = rules.get_mut(index)
        {
            prepend_decor_comment(next.decor_mut(), comment);
            return;
        }
        let trailing = self.doc.trailing().as_str().unwrap_or_default().to_owned();
        self.doc
            .set_trailing(join_comment_trailing(comment, &trailing));
    }
}

/// Returns comments in `prefix` separated from the following item by at least one blank line.
/// Comments directly above the removed item (no blank line) are treated as attached to it.
fn detached_comments(prefix: &str) -> Option<String> {
    if !prefix.contains('#') {
        return None;
    }
    let mut last_blank_end = None;
    let mut offset = 0;
    for line in prefix.split_inclusive('\n') {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if content.trim().is_empty() {
            last_blank_end = Some(offset + line.len());
        }
        offset += line.len();
    }
    let kept = trim_trailing_blank_lines(&prefix[..last_blank_end?]);
    (kept.contains('#') && !kept.is_empty()).then(|| kept.to_owned())
}

fn trim_trailing_blank_lines(s: &str) -> &str {
    let mut end = s.len();
    for line in s.split_inclusive('\n').rev() {
        let content = line.strip_suffix('\n').unwrap_or(line);
        let content = content.strip_suffix('\r').unwrap_or(content);
        if content.trim().is_empty() {
            end -= line.len();
        } else {
            break;
        }
    }
    &s[..end]
}

fn prepend_decor_comment(decor: &mut Decor, comment: &str) {
    let existing = decor
        .prefix()
        .and_then(RawString::as_str)
        .unwrap_or_default()
        .to_owned();
    decor.set_prefix(join_comment_prefix(comment, &existing));
}

fn join_comment_prefix(comment: &str, existing: &str) -> String {
    if existing.is_empty() {
        format!("{comment}\n")
    } else if existing.starts_with('\n') || existing.starts_with("\r\n") {
        format!("{comment}{existing}")
    } else {
        format!("{comment}\n{existing}")
    }
}

fn join_comment_trailing(comment: &str, existing: &str) -> String {
    if existing.is_empty() {
        comment.to_owned()
    } else if existing.starts_with('\n') || existing.starts_with("\r\n") {
        format!("{comment}{existing}")
    } else {
        format!("{comment}\n{existing}")
    }
}

fn min_table_position(table: &Table, min_exclusive: Option<isize>) -> Option<isize> {
    let mut best: Option<isize> = None;
    let mut consider = |candidate: Option<isize>| {
        if let Some(pos) = candidate
            && min_exclusive.is_none_or(|after| pos > after)
            && best.is_none_or(|current| pos < current)
        {
            best = Some(pos);
        }
    };
    for (_, item) in table.iter() {
        match item {
            Item::Table(sub) => {
                if !sub.is_implicit() {
                    consider(sub.position());
                }
                consider(min_table_position(sub, min_exclusive));
            }
            Item::ArrayOfTables(array) => {
                for sub in array.iter() {
                    consider(sub.position());
                    consider(min_table_position(sub, min_exclusive));
                }
            }
            _ => {}
        }
    }
    best
}

fn find_table_at_position_mut(table: &mut Table, target: isize) -> Option<&mut Table> {
    for (_, item) in table.iter_mut() {
        match item {
            Item::Table(sub) => {
                if !sub.is_implicit() && sub.position() == Some(target) {
                    return Some(sub);
                }
                if let Some(found) = find_table_at_position_mut(sub, target) {
                    return Some(found);
                }
            }
            Item::ArrayOfTables(array) => {
                for sub in array.iter_mut() {
                    if sub.position() == Some(target) {
                        return Some(sub);
                    }
                    if let Some(found) = find_table_at_position_mut(sub, target) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn next_value_key_after(table: &Table, key_name: &str) -> Option<String> {
    let mut saw_key = false;
    for (key, item) in table.iter() {
        if saw_key && item.is_value() {
            return Some(key.to_owned());
        }
        if key == key_name {
            saw_key = true;
        }
    }
    None
}
