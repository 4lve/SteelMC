use std::collections::HashMap;
use steel_utils::Identifier;
use steel_utils::text::TextComponent;

use crate::RegistryExt;

/// Represents the different types of dialogs defined in data packs.
#[derive(Debug)]
pub enum Dialog {
    DialogList(DialogList),
    ServerLinks(ServerLinks),
}

/// A dialog that displays a list of other dialogs.
#[derive(Debug)]
pub struct DialogList {
    pub key: Identifier,
    pub button_width: i32,
    pub columns: i32,
    pub dialogs: &'static str,
    pub exit_action: ExitAction,
    pub external_title: TextComponent,
    pub title: TextComponent,
}

/// A dialog that displays server links.
#[derive(Debug)]
pub struct ServerLinks {
    pub key: Identifier,
    pub button_width: i32,
    pub columns: i32,
    pub exit_action: ExitAction,
    pub external_title: TextComponent,
    pub title: TextComponent,
}

/// Represents an exit action with a label and width.
#[derive(Debug)]
pub struct ExitAction {
    pub label: TextComponent,
    pub width: i32,
}

pub type DialogRef = &'static Dialog;

pub struct DialogRegistry {
    dialogs_by_id: Vec<DialogRef>,
    dialogs_by_key: HashMap<Identifier, usize>,
    allows_registering: bool,
}

impl DialogRegistry {
    #[must_use]
    pub fn new() -> Self {
        Self {
            dialogs_by_id: Vec::new(),
            dialogs_by_key: HashMap::new(),
            allows_registering: true,
        }
    }

    pub fn register(&mut self, key: Identifier, dialog: DialogRef) -> usize {
        assert!(
            self.allows_registering,
            "Cannot register dialogs after the registry has been frozen"
        );

        let id = self.dialogs_by_id.len();
        self.dialogs_by_key.insert(key, id);
        self.dialogs_by_id.push(dialog);
        id
    }

    #[must_use]
    pub fn by_id(&self, id: usize) -> Option<DialogRef> {
        self.dialogs_by_id.get(id).copied()
    }

    #[must_use]
    pub fn get_id(&self, dialog: DialogRef) -> &usize {
        let key = match dialog {
            Dialog::DialogList(d) => &d.key,
            Dialog::ServerLinks(s) => &s.key,
        };
        self.dialogs_by_key.get(key).expect("Dialog not found")
    }

    #[must_use]
    pub fn by_key(&self, key: &Identifier) -> Option<DialogRef> {
        self.dialogs_by_key.get(key).and_then(|id| self.by_id(*id))
    }

    pub fn iter(&self) -> impl Iterator<Item = (usize, DialogRef)> + '_ {
        self.dialogs_by_id
            .iter()
            .enumerate()
            .map(|(id, &dialog)| (id, dialog))
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.dialogs_by_id.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.dialogs_by_id.is_empty()
    }
}

impl RegistryExt for DialogRegistry {
    fn freeze(&mut self) {
        self.allows_registering = false;
    }
}

impl Default for DialogRegistry {
    fn default() -> Self {
        Self::new()
    }
}
