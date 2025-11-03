pub use crate::blocks::properties::NoteBlockInstrument;

#[derive(Debug, Clone, Copy)]
pub enum PushReaction {
    Normal,
    Destroy,
    Block,
    Ignore,
    PushOnly,
}

#[derive(Debug)]
pub struct BlockBehaviourProperties {
    pub has_collision: bool,
    pub can_occlude: bool,
    pub explosion_resistance: f32,
    pub is_randomly_ticking: bool,
    pub force_solid_off: bool,
    pub force_solid_on: bool,
    pub push_reaction: PushReaction,
    pub friction: f32,
    pub speed_factor: f32,
    pub jump_factor: f32,
    pub dynamic_shape: bool,
    pub destroy_time: f32,
    pub ignited_by_lava: bool,
    pub liquid: bool,
    pub is_air: bool,
    pub requires_correct_tool_for_drops: bool,
    pub instrument: NoteBlockInstrument,
    pub replaceable: bool,
}

impl BlockBehaviourProperties {
    /// Starts building a new set of block properties.
    pub const fn new() -> Self {
        Self {
            has_collision: true,
            can_occlude: true,
            explosion_resistance: 0.0,
            is_randomly_ticking: false,
            force_solid_off: false,
            force_solid_on: false,
            push_reaction: PushReaction::Normal,
            friction: 0.6,
            speed_factor: 1.0,
            jump_factor: 1.0,
            dynamic_shape: false,
            destroy_time: 0.0,
            ignited_by_lava: false,
            liquid: false,
            is_air: false,
            requires_correct_tool_for_drops: false,
            instrument: NoteBlockInstrument::Harp,
            replaceable: false,
        }
    }

    pub const fn has_collision(mut self, has_collision: bool) -> Self {
        self.has_collision = has_collision;
        self
    }

    pub const fn can_occlude(mut self, can_occlude: bool) -> Self {
        self.can_occlude = can_occlude;
        self
    }

    pub const fn explosion_resistance(mut self, resistance: f32) -> Self {
        self.explosion_resistance = resistance;
        self
    }

    pub const fn is_randomly_ticking(mut self, ticking: bool) -> Self {
        self.is_randomly_ticking = ticking;
        self
    }

    pub const fn force_solid_off(mut self, force: bool) -> Self {
        self.force_solid_off = force;
        self
    }

    pub const fn force_solid_on(mut self, force: bool) -> Self {
        self.force_solid_on = force;
        self
    }

    pub const fn push_reaction(mut self, reaction: PushReaction) -> Self {
        self.push_reaction = reaction;
        self
    }

    pub const fn friction(mut self, friction: f32) -> Self {
        self.friction = friction;
        self
    }

    pub const fn speed_factor(mut self, factor: f32) -> Self {
        self.speed_factor = factor;
        self
    }

    pub const fn jump_factor(mut self, factor: f32) -> Self {
        self.jump_factor = factor;
        self
    }

    pub const fn dynamic_shape(mut self, dynamic: bool) -> Self {
        self.dynamic_shape = dynamic;
        self
    }

    pub const fn destroy_time(mut self, time: f32) -> Self {
        self.destroy_time = time;
        self
    }

    pub const fn ignited_by_lava(mut self, ignited: bool) -> Self {
        self.ignited_by_lava = ignited;
        self
    }

    pub const fn liquid(mut self, liquid: bool) -> Self {
        self.liquid = liquid;
        self
    }

    pub const fn is_air(mut self, is_air: bool) -> Self {
        self.is_air = is_air;
        self
    }

    pub const fn requires_correct_tool_for_drops(mut self, requires: bool) -> Self {
        self.requires_correct_tool_for_drops = requires;
        self
    }

    pub const fn instrument(mut self, instrument: NoteBlockInstrument) -> Self {
        self.instrument = instrument;
        self
    }

    pub const fn replaceable(mut self, replaceable: bool) -> Self {
        self.replaceable = replaceable;
        self
    }
}

impl Default for BlockBehaviourProperties {
    fn default() -> Self {
        Self::new()
    }
}
