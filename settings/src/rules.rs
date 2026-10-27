//! `config.toml` as the settings window edits it: rules and workspace displays, changed in
//! place so comments, order and anything else in the file survive. Every save is checked by
//! the agent's own parser first, and written atomically.

use std::path::PathBuf;

use spaceman::WorkspaceId;
use toml_edit::{ArrayOfTables, DocumentMut, Item, Table, value};

const EMPTY: &str = "# Spaceman configuration.\nversion = 1\n";

/// Where a rule sends an app's windows.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Place {
    Workspace(WorkspaceId),
    Float,
    /// Never tiled, moved or parked.
    Ignore,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Rule {
    /// Its position among the file's `[[rules]]`.
    pub index: usize,
    pub bundle_id: Option<String>,
    pub title: Option<String>,
    pub place: Place,
}

#[derive(Clone)]
pub struct Doc {
    path: Option<PathBuf>,
    doc: DocumentMut,
    /// Why the file couldn't be read, while it can't; saving is refused until it can.
    pub error: Option<String>,
}

impl Doc {
    pub fn load() -> Doc {
        let path = spaceman::config::config_path();
        let source = match path.as_ref().map(std::fs::read_to_string) {
            Some(Ok(source)) => source,
            Some(Err(error)) if error.kind() == std::io::ErrorKind::NotFound => EMPTY.into(),
            Some(Err(error)) => return Doc::broken(path, format!("Couldn't read config: {error}")),
            None => return Doc::broken(None, "HOME is unavailable".into()),
        };
        let mut doc = Doc::parse(&source);
        doc.path = path;
        doc
    }

    pub fn parse(source: &str) -> Doc {
        match source.parse::<DocumentMut>() {
            Ok(doc) => Doc {
                path: None,
                doc,
                error: spaceman::config::Config::parse(source).err(),
            },
            Err(error) => Doc::broken(None, error.to_string()),
        }
    }

    fn broken(path: Option<PathBuf>, error: String) -> Doc {
        Doc {
            path,
            doc: DocumentMut::new(),
            error: Some(error),
        }
    }

    pub fn rules(&self) -> Vec<Rule> {
        let Some(rules) = self.doc.get("rules").and_then(Item::as_array_of_tables) else {
            return Vec::new();
        };
        rules
            .iter()
            .enumerate()
            .filter_map(|(index, table)| {
                let place = match table.get("action").and_then(Item::as_str)? {
                    "float" => Place::Float,
                    "ignore" => Place::Ignore,
                    "route" => Place::Workspace(workspace(table.get("workspace")?)?),
                    _ => return None,
                };
                Some(Rule {
                    index,
                    bundle_id: string(table, "bundle_id"),
                    title: string(table, "title"),
                    place,
                })
            })
            .collect()
    }

    /// Sends rule `index`'s windows to `place`.
    pub fn place(&mut self, index: usize, place: Place) {
        if let Some(table) = self.rule_tables().get_mut(index) {
            set_place(table, place);
        }
    }

    pub fn remove(&mut self, index: usize) {
        let rules = self.rule_tables();
        if index < rules.len() {
            rules.remove(index);
        }
    }

    pub fn add(&mut self, bundle_id: &str, place: Place) {
        let mut table = Table::new();
        table.insert("bundle_id", value(bundle_id));
        set_place(&mut table, place);
        self.rule_tables().push(table);
    }

    /// The display a workspace asks for in `[[workspaces]]`, if it names one.
    pub fn monitor(&self, workspace: WorkspaceId) -> Option<String> {
        self.doc
            .get("workspaces")?
            .as_array_of_tables()?
            .iter()
            .find(|table| table.get("workspace").and_then(self::workspace) == Some(workspace))
            .and_then(|table| string(table, "monitor"))
    }

    /// Sets the display a workspace asks for; `None` removes its entry, back to the default.
    pub fn set_monitor(&mut self, workspace: WorkspaceId, monitor: Option<&str>) {
        let tables = self
            .doc
            .entry("workspaces")
            .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()));
        let Some(tables) = tables.as_array_of_tables_mut() else {
            return;
        };
        let found = tables
            .iter()
            .position(|table| table.get("workspace").and_then(self::workspace) == Some(workspace));
        match (found, monitor) {
            (Some(index), Some(monitor)) => {
                if let Some(table) = tables.get_mut(index) {
                    table.insert("monitor", value(monitor));
                }
            }
            (Some(index), None) => {
                tables.remove(index);
            }
            (None, Some(monitor)) => {
                let mut table = Table::new();
                table.insert("workspace", workspace_value(workspace));
                table.insert("monitor", value(monitor));
                tables.push(table);
            }
            (None, None) => {}
        }
        if tables.is_empty() {
            self.doc.remove("workspaces");
        }
    }

    fn rule_tables(&mut self) -> &mut ArrayOfTables {
        self.doc
            .entry("rules")
            .or_insert_with(|| Item::ArrayOfTables(ArrayOfTables::new()))
            .as_array_of_tables_mut()
            .expect("rules is an array of tables")
    }

    pub fn source(&self) -> String {
        self.doc.to_string()
    }

    /// Writes the file once Spaceman's own parser accepts it; the agent picks it up from there.
    pub fn save(&mut self) -> Result<(), String> {
        if let Some(error) = &self.error {
            return Err(format!("config.toml has an error to fix first: {error}"));
        }
        let source = self.source();
        spaceman::config::Config::parse(&source)?;
        let Some(path) = &self.path else {
            return Ok(());
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let mut file =
            atomic_write_file::AtomicWriteFile::open(path).map_err(|error| error.to_string())?;
        std::io::Write::write_all(&mut file, source.as_bytes())
            .map_err(|error| error.to_string())?;
        file.commit().map_err(|error| error.to_string())
    }
}

fn string(table: &Table, key: &str) -> Option<String> {
    table.get(key).and_then(Item::as_str).map(str::to_owned)
}

fn workspace(item: &Item) -> Option<WorkspaceId> {
    match item.as_value()? {
        toml_edit::Value::Integer(number) => {
            WorkspaceId::new(u8::try_from(*number.value()).ok()?).ok()
        }
        toml_edit::Value::String(name) => WorkspaceId::parse(name.value()).ok(),
        _ => None,
    }
}

fn workspace_value(workspace: WorkspaceId) -> Item {
    match workspace.number() {
        Some(number) => value(i64::from(number)),
        None => value(workspace.to_string()),
    }
}

fn set_place(table: &mut Table, place: Place) {
    match place {
        Place::Workspace(workspace) => {
            table.insert("action", value("route"));
            table.insert("workspace", workspace_value(workspace));
        }
        Place::Float | Place::Ignore => {
            let action = if place == Place::Float {
                "float"
            } else {
                "ignore"
            };
            table.insert("action", value(action));
            table.remove("workspace");
        }
    }
}

#[cfg(test)]
mod tests;
