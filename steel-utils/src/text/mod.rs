//! This module contains everything related to text components.
use crate::translations_registry::TRANSLATIONS;
use text_components::{
    TextComponent,
    content::{Content, Resolvable},
    custom::CustomData,
    resolving::TextResolutor,
};

/// A [`TextResolutor`] for the console
pub struct DisplayResolutor;
impl TextResolutor for DisplayResolutor {
    fn resolve_content(&self, resolvable: &Resolvable) -> TextComponent {
        TextComponent {
            content: Content::Resolvable(resolvable.clone()),
            ..Default::default()
        }
    }

    fn resolve_custom(&self, _data: &CustomData) -> Option<TextComponent> {
        None
    }

    fn translate(&self, key: &str) -> Option<String> {
        TRANSLATIONS.get(key).map(ToString::to_string)
    }
}
