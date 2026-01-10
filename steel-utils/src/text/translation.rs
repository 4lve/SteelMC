use std::borrow::Cow;

use serde::{Deserialize, Serialize};

use crate::{text::TextComponent, translations::TRANSLATIONS};

/// A translation with compile-time argument count checking
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct Translation<const ARGS: usize> {
    pub key: &'static str,
    pub format: &'static str,
}

#[allow(missing_docs)]
impl<const ARGS: usize> Translation<ARGS> {
    #[must_use]
    pub const fn new(key: &'static str, format: &'static str) -> Self {
        Self { key, format }
    }
}

impl Translation<0> {
    /// Creates a new `TranslatedMessage` with no arguments.
    #[must_use]
    pub fn msg(&self) -> TranslatedMessage {
        TranslatedMessage::new(self.key, None)
    }
}

impl<const ARGS: usize> Translation<ARGS> {
    /// Creates a new `TranslatedMessage` with the given arguments.
    #[must_use]
    pub fn message(self, args: [impl Into<TextComponent>; ARGS]) -> TranslatedMessage {
        TranslatedMessage::new(self.key, Some(Box::new(args.map(Into::into))))
    }
}

/// A constructed message
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[allow(missing_docs)]
pub struct TranslatedMessage {
    pub key: Cow<'static, str>,
    pub fallback: Option<Cow<'static, str>>,
    pub args: Option<Box<[TextComponent]>>,
}

/// An empty array of text components.
pub const EMPTY_ARGS: &[TextComponent] = &[];

impl TranslatedMessage {
    /// Creates a new `TranslatedMessage`.
    #[must_use]
    pub const fn new(key: &'static str, args: Option<Box<[TextComponent]>>) -> Self {
        Self {
            key: Cow::Borrowed(key),
            args,
            fallback: None,
        }
    }

    /// Get the translation key (for sending to client)
    #[must_use]
    pub fn key(&self) -> Cow<'static, str> {
        self.key.clone()
    }

    /// Get the arguments as a slice
    #[must_use]
    pub fn args(&self) -> &[TextComponent] {
        match &self.args {
            Some(args) => args,
            None => EMPTY_ARGS,
        }
    }

    /// Get key and arguments for client packet
    #[must_use]
    pub fn client_data(&self) -> (Cow<'static, str>, &[TextComponent]) {
        (self.key.clone(), self.args())
    }

    /// Format the message on the server-side
    ///
    /// # Panics
    /// - If the translation key is not found.
    #[must_use]
    pub fn format(&self) -> String {
        let mut result = (*TRANSLATIONS
            .get(self.key.as_ref())
            .expect("Translation key should exist"))
        .to_string();

        // Handle positional arguments
        for (i, arg) in self.args().iter().enumerate() {
            result = result.replace(&format!("%{}$s", i + 1), &arg.to_string());
        }

        // Handle sequential %s
        for arg in self.args() {
            result = result.replacen("%s", &arg.to_string(), 1);
        }

        result
    }
}

impl From<TranslatedMessage> for TextComponent {
    fn from(value: TranslatedMessage) -> Self {
        TextComponent::new().translate(value)
    }
}
