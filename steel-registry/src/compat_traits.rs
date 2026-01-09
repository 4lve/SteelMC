use steel_utils::{BlockPos, BlockStateId, types::UpdateFlags};

pub trait RegistryPlayer {}

pub trait RegistryWorld {
    /// Gets the block state at the given position.
    fn get_block_state(&self, pos: &BlockPos) -> BlockStateId;

    /// Sets a block at the given position.
    /// Returns `true` if the block was successfully set, `false` otherwise.
    fn set_block(&self, pos: BlockPos, block_state: BlockStateId, flags: UpdateFlags) -> bool;

    /// Returns whether the block position is within valid world bounds.
    fn is_in_valid_bounds(&self, block_pos: &BlockPos) -> bool;
}

pub trait RegistryServer {}
