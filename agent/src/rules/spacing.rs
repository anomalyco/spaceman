//! `[settings.gaps]` and `settings.single_window_max_width`, edited in place like the rest of
//! the file. A value at its default `0` leaves the file, and so does a table it empties.

use spaceman::spacing::{Spacing, SpacingKey};
use toml_edit::{Item, Table};

use super::Doc;

impl Doc {
    /// The spacing the file asks for; a missing or out-of-range key reads as `0`.
    pub fn spacing(&self) -> Spacing {
        let settings = self.doc.get("settings").and_then(Item::as_table_like);
        let gaps = settings
            .and_then(|table| table.get("gaps"))
            .and_then(Item::as_table_like);
        let mut spacing = Spacing::NONE;
        for key in SpacingKey::ALL {
            let item = match key.gap_name() {
                Some(name) => gaps.and_then(|table| table.get(name)),
                None => settings.and_then(|table| table.get(key.path())),
            };
            let checked = item
                .and_then(Item::as_integer)
                .and_then(|raw| key.check(raw).ok())
                .unwrap_or(0);
            key.set(&mut spacing, checked);
        }
        spacing
    }

    /// Writes every spacing key that differs from what the file holds, removing the ones at `0`.
    pub fn set_spacing(&mut self, spacing: Spacing) {
        let current = self.spacing();
        for key in SpacingKey::ALL {
            let value = key.get(&spacing);
            if key.get(&current) == value && self.has_spacing_key(key) == (value != 0) {
                continue;
            }
            let item = (value != 0).then(|| toml_edit::value(i64::from(value)));
            match key.gap_name() {
                Some(name) => self.set_gap(name, item),
                None => self.set_setting(key.path(), item),
            }
        }
    }

    fn has_spacing_key(&self, key: SpacingKey) -> bool {
        let settings = self.doc.get("settings").and_then(Item::as_table_like);
        match key.gap_name() {
            Some(name) => settings
                .and_then(|table| table.get("gaps"))
                .and_then(Item::as_table_like)
                .is_some_and(|gaps| gaps.contains_key(name)),
            None => settings.is_some_and(|table| table.contains_key(key.path())),
        }
    }

    /// Sets `name` in `[settings.gaps]`, or removes it (and the tables it empties) for `None`.
    fn set_gap(&mut self, name: &str, item: Option<Item>) {
        let Some(item) = item else {
            let Some(settings) = self.doc.get_mut("settings").and_then(Item::as_table_mut) else {
                return;
            };
            let emptied = settings
                .get_mut("gaps")
                .and_then(Item::as_table_mut)
                .is_some_and(|gaps| {
                    gaps.remove(name);
                    gaps.is_empty()
                });
            if emptied {
                self.remove_subtable("settings", "gaps");
            }
            if self
                .doc
                .get("settings")
                .and_then(Item::as_table_like)
                .is_some_and(toml_edit::TableLike::is_empty)
            {
                self.remove_table("settings");
            }
            return;
        };
        let settings = self.doc.entry("settings").or_insert_with(|| {
            // Holding only `[settings.gaps]`, the parent needs no header of its own.
            let mut table = Table::new();
            table.set_implicit(true);
            Item::Table(table)
        });
        if !settings.is_table() {
            *settings = Item::Table(Table::new());
        }
        let Some(settings) = settings.as_table_mut() else {
            return;
        };
        let position = settings.position();
        let gaps = settings
            .entry("gaps")
            .or_insert_with(|| Item::Table(Table::new()));
        if !gaps.is_table() {
            *gaps = Item::Table(Table::new());
        }
        if let Some(gaps) = gaps.as_table_mut() {
            // Sharing `[settings]`'s place draws it right after its parent, not at the file's end.
            if let (None, Some(position)) = (gaps.position(), position) {
                gaps.set_position(position);
            }
            super::set_table_item(gaps, name, item);
        }
    }
}
