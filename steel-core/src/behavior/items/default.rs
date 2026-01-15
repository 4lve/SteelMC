//! Default item behavior implementation.

use crate::behavior::ItemBehavior;
use crate::behavior::context::{InteractionResult, UseOnContext};

/// Default item behavior - does nothing special.
pub struct DefaultItemBehavior;

impl ItemBehavior for DefaultItemBehavior {
    fn use_on(&self, _context: &mut UseOnContext) -> InteractionResult {
        InteractionResult::Pass
    }
}
