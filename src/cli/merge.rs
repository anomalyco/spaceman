//! Writing a CLI edit back into `config.toml` without reformatting the file.
//!
//! The CLI edits a typed copy of the config and renders it fresh, which drops every comment and
//! reorders keys. [`merge_into`] instead applies the difference between the file and that rendering
//! to the file's own syntax tree: keys and entries the edit did not change keep their text,
//! comments, and order; changed values keep the comments around them; removed keys and entries go
//! with the comments attached to them; new ones are added beside their siblings.

use toml_edit::{Array, ArrayOfTables, DocumentMut, InlineTable, Item, Table, TableLike, Value};

/// The `source` file with `rendered`'s content, or `None` when either fails to parse.
pub(crate) fn merge_into(source: &str, rendered: &str) -> Option<String> {
    let mut doc = source.parse::<DocumentMut>().ok()?;
    let target = rendered.parse::<DocumentMut>().ok()?;
    merge_table(doc.as_table_mut(), target.as_table());
    Some(doc.to_string())
}

fn merge_table(old: &mut Table, new: &Table) {
    let stale: Vec<String> = old
        .iter()
        .filter(|(key, _)| new.get(key).is_none_or(Item::is_none))
        .map(|(key, _)| key.to_owned())
        .collect();
    for key in stale {
        old.remove(&key);
    }
    for (key, item) in new.iter() {
        if item.is_none() {
            continue;
        }
        match old.get_mut(key) {
            Some(existing) => merge_item(existing, item),
            None => {
                if item.is_value() && old.is_implicit() && !old.is_dotted() {
                    old.set_implicit(false);
                }
                old.insert(key, detach(item));
            }
        }
    }
}

fn merge_item(old: &mut Item, new: &Item) {
    if same_item(old, new) {
        return;
    }
    match (old, new) {
        (Item::Table(old), Item::Table(new)) => merge_table(old, new),
        (Item::Table(old), Item::Value(Value::InlineTable(new))) => {
            merge_table(old, &new.clone().into_table());
        }
        (Item::ArrayOfTables(old), Item::ArrayOfTables(new)) => merge_tables(old, new),
        (Item::Value(old), Item::Value(new)) => merge_value(old, new),
        (Item::Value(old), Item::Table(new)) if old.is_inline_table() => {
            merge_value(old, &Value::InlineTable(new.clone().into_inline_table()));
        }
        (Item::Value(old), Item::ArrayOfTables(new)) if old.is_array() => {
            merge_value(old, &Value::Array(new.clone().into_array()));
        }
        (old, new) => *old = detach(new),
    }
}

fn merge_tables(old: &mut ArrayOfTables, new: &ArrayOfTables) {
    let olds: Vec<&Table> = old.iter().collect();
    let news: Vec<&Table> = new.iter().collect();
    let plan = align(olds.len(), news.len(), |i, j| same_tables(olds[i], news[j]));
    let mut merged = ArrayOfTables::new();
    for (from, j) in plan {
        merged.push(match from {
            Some(i) => {
                let mut table = olds[i].clone();
                merge_table(&mut table, news[j]);
                table
            }
            None => detach_table(news[j]),
        });
    }
    *old = merged;
}

fn merge_value(old: &mut Value, new: &Value) {
    if same_value(old, new) {
        return;
    }
    match (old, new) {
        (Value::InlineTable(old), Value::InlineTable(new)) => merge_inline(old, new),
        (Value::Array(old), Value::Array(new)) => merge_array(old, new),
        (old, new) => {
            let decor = old.decor().clone();
            *old = new.clone();
            *old.decor_mut() = decor;
        }
    }
}

fn merge_inline(old: &mut InlineTable, new: &InlineTable) {
    let stale: Vec<String> = old
        .iter()
        .filter(|(key, _)| new.get(key).is_none())
        .map(|(key, _)| key.to_owned())
        .collect();
    for key in stale {
        old.remove(&key);
    }
    for (key, value) in new.iter() {
        match old.get_mut(key) {
            Some(existing) => merge_value(existing, value),
            None => {
                old.insert(key, value.clone());
            }
        }
    }
}

fn merge_array(old: &mut Array, new: &Array) {
    let olds: Vec<&Value> = old.iter().collect();
    let news: Vec<&Value> = new.iter().collect();
    let plan = align(olds.len(), news.len(), |i, j| same_value(olds[i], news[j]));
    let mut merged = Array::new();
    for (from, j) in plan {
        merged.push_formatted(match from {
            Some(i) => {
                let mut value = olds[i].clone();
                merge_value(&mut value, news[j]);
                value
            }
            None => news[j].clone(),
        });
    }
    merged.set_trailing(old.trailing().clone());
    merged.set_trailing_comma(old.trailing_comma());
    *merged.decor_mut() = old.decor().clone();
    *old = merged;
}

/// For each new element in order, the old element it continues, if any. Equal lengths pair
/// elements by position (an edit in place); otherwise a longest common subsequence of equal
/// elements keeps the untouched entries and their comments around an insertion or removal.
fn align(
    old_len: usize,
    new_len: usize,
    same: impl Fn(usize, usize) -> bool,
) -> Vec<(Option<usize>, usize)> {
    if old_len == new_len {
        return (0..new_len).map(|j| (Some(j), j)).collect();
    }
    let width = new_len + 1;
    let mut longest = vec![0_usize; (old_len + 1) * width];
    for i in (0..old_len).rev() {
        for j in (0..new_len).rev() {
            longest[i * width + j] = if same(i, j) {
                longest[(i + 1) * width + j + 1] + 1
            } else {
                longest[(i + 1) * width + j].max(longest[i * width + j + 1])
            };
        }
    }
    let (mut i, mut j) = (0, 0);
    let mut plan = Vec::with_capacity(new_len);
    while j < new_len {
        if i < old_len && same(i, j) {
            plan.push((Some(i), j));
            i += 1;
            j += 1;
        } else if i < old_len && longest[(i + 1) * width + j] >= longest[i * width + j + 1] {
            i += 1;
        } else {
            plan.push((None, j));
            j += 1;
        }
    }
    plan
}

/// A copy of `item` whose tables carry no position from the document they came from, so
/// they print beside their siblings in the document they join.
fn detach(item: &Item) -> Item {
    match item {
        Item::Table(table) => Item::Table(detach_table(table)),
        Item::ArrayOfTables(tables) => {
            let mut out = ArrayOfTables::new();
            for table in tables.iter() {
                out.push(detach_table(table));
            }
            Item::ArrayOfTables(out)
        }
        other => other.clone(),
    }
}

fn detach_table(table: &Table) -> Table {
    let mut out = Table::new();
    out.set_implicit(table.is_implicit());
    out.set_dotted(table.is_dotted());
    for (key, item) in table.iter() {
        out.insert(key, detach(item));
    }
    out
}

fn same_item(a: &Item, b: &Item) -> bool {
    match (a, b) {
        (Item::None, Item::None) => true,
        (Item::Value(a), Item::Value(b)) => same_value(a, b),
        _ => match (a.as_table_like(), b.as_table_like()) {
            (Some(a), Some(b)) => same_tables(a, b),
            _ => match (table_entries(a), table_entries(b)) {
                (Some(a), Some(b)) => {
                    a.len() == b.len() && a.iter().zip(&b).all(|(a, b)| same_tables(*a, *b))
                }
                _ => false,
            },
        },
    }
}

fn table_entries(item: &Item) -> Option<Vec<&dyn TableLike>> {
    match item {
        Item::ArrayOfTables(tables) => {
            Some(tables.iter().map(|table| table as &dyn TableLike).collect())
        }
        Item::Value(Value::Array(values)) => values
            .iter()
            .map(|value| value.as_inline_table().map(|table| table as &dyn TableLike))
            .collect(),
        _ => None,
    }
}

fn same_tables(a: &dyn TableLike, b: &dyn TableLike) -> bool {
    a.len() == b.len()
        && a.iter()
            .all(|(key, a)| b.get(key).is_some_and(|b| same_item(a, b)))
}

fn same_value(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::String(a), Value::String(b)) => a.value() == b.value(),
        (Value::Integer(a), Value::Integer(b)) => a.value() == b.value(),
        (Value::Float(a), Value::Float(b)) => a.value().to_bits() == b.value().to_bits(),
        (Value::Boolean(a), Value::Boolean(b)) => a.value() == b.value(),
        (Value::Datetime(a), Value::Datetime(b)) => a.value() == b.value(),
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b.iter()).all(|(a, b)| same_value(a, b))
        }
        (Value::InlineTable(a), Value::InlineTable(b)) => same_tables(a, b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::merge_into;

    #[test]
    fn keeps_comments_and_order_around_a_removed_key() {
        let source = "# Top.\nversion = 1\n\n[settings]\n# Columns.\nlayout = \"columns\"\nhud_position = \"top\"\n\n# Disabled:\n# [[rules]]\n";
        let rendered = "version = 1\n\n[settings]\nlayout = \"columns\"\n";
        assert_eq!(
            merge_into(source, rendered).unwrap(),
            "# Top.\nversion = 1\n\n[settings]\n# Columns.\nlayout = \"columns\"\n\n# Disabled:\n# [[rules]]\n"
        );
    }

    #[test]
    fn changes_a_value_in_place_and_leaves_an_unchanged_file_alone() {
        let source = "version = 1\n[settings]\nlayout = 'dwindle' # mine\nhud = false\n";
        let rendered = "version = 1\n\n[settings]\nhud = false\nlayout = \"columns\"\n";
        assert_eq!(
            merge_into(source, rendered).unwrap(),
            "version = 1\n[settings]\nlayout = \"columns\" # mine\nhud = false\n"
        );
        let same = "version = 1\n\n[settings]\nhud = false\nlayout = 'dwindle'\n";
        assert_eq!(merge_into(source, same).unwrap(), source);
    }

    #[test]
    fn removes_and_adds_array_entries_without_touching_the_others() {
        let source = "version = 1\n\n# One.\n[[rules]]\nbundle_id = \"a\"\naction = \"float\"\n\n# Two.\n[[rules]]\nbundle_id = \"b\"\naction = \"float\"\n\n# Trailing note.\n";
        let removed = "version = 1\n\n[[rules]]\nbundle_id = \"b\"\naction = \"float\"\n";
        assert_eq!(
            merge_into(source, removed).unwrap(),
            "version = 1\n\n# Two.\n[[rules]]\nbundle_id = \"b\"\naction = \"float\"\n\n# Trailing note.\n"
        );
        let added = "version = 1\n\n[[rules]]\nbundle_id = \"a\"\naction = \"float\"\n\n[[rules]]\nbundle_id = \"b\"\naction = \"float\"\n\n[[rules]]\nbundle_id = \"c\"\naction = \"float\"\n";
        let merged = merge_into(source, added).unwrap();
        assert!(
            merged.contains("# One.\n[[rules]]\nbundle_id = \"a\""),
            "{merged}"
        );
        assert!(
            merged.contains("# Two.\n[[rules]]\nbundle_id = \"b\""),
            "{merged}"
        );
        assert!(merged.contains("# Trailing note."), "{merged}");
        assert!(
            merged.find("bundle_id = \"b\"") < merged.find("bundle_id = \"c\""),
            "{merged}"
        );
    }
}
