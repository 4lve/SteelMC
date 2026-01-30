# Fluid Caching Implementation Plan

Based on vanilla FlowingFluid.java's caching mechanisms.

## Current Performance Issues

### 1. Multiple World Lookups in get_spread()
**Current code pattern (flowing.rs:565-588):**
```rust
for direction in [North, South, East, West] {
    let neighbor = direction.relative(&pos);
    if !can_pass_horizontally(world, &neighbor, fluid_id) {  // Query 1
        continue;
    }
    let new_fluid = get_new_liquid(world, neighbor, fluid_id, drop_off);  // Query 2-8 (neighbors)
    if is_hole(world, &neighbor, fluid_id) {  // Query 9-10 (below neighbor)
        // ...
    }
    let distance = get_slope_distance(world, neighbor, ...);  // Recursive queries!
}
```

**Problem:** Each neighbor position is queried multiple times during the spread calculation.

### 2. No Occlusion Caching in can_pass_through_wall()
**Current code (flowing.rs:179-225):**
- Gets collision shapes from both blocks
- Performs expensive face occlusion check
- No caching - recomputed every time

## Implementation Plan

## Phase 1: SpreadContext (Local Cache)

**File:** `steel-core/src/fluid/spread_context.rs` (new file)

**Structure:**
```rust
pub struct SpreadContext {
    /// Cache for block states by encoded relative position
    state_cache: FxHashMap<i16, BlockStateId>,
    /// Cache for hole check results by encoded relative position  
    hole_cache: FxHashMap<i16, bool>,
    /// Reference to world for cache misses
    world: &World,
}

impl SpreadContext {
    /// Encode relative x, z coordinates into a short key
    /// Uses vanilla's encoding: (dx + 128) << 8 | (dz + 128)
    fn encode_key(dx: i8, dz: i8) -> i16 {
        ((dx as i16 + 128) << 8) | (dz as i16 + 128)
    }
    
    /// Get cached block state or query world
    fn get_block_state(&mut self, pos: BlockPos) -> BlockStateId {
        let dx = (pos.0.x as i8);
        let dz = (pos.0.z as i8);
        let key = Self::encode_key(dx, dz);
        
        *self.state_cache.entry(key).or_insert_with(|| {
            self.world.get_block_state(&pos)
        })
    }
    
    /// Check if position is a hole (with caching)
    fn is_hole(&mut self, pos: BlockPos, fluid_id: u8) -> bool {
        let dx = (pos.0.x as i8);
        let dz = (pos.0.z as i8);
        let key = Self::encode_key(dx, dz);
        
        *self.hole_cache.entry(key).or_insert_with(|| {
            is_hole_internal(self.world, pos, fluid_id)
        })
    }
}
```

**Modified functions:**
- `get_spread()` - Create SpreadContext at start, pass to helpers
- `get_slope_distance()` - Accept SpreadContext instead of World
- `can_pass_horizontally()` - Use context.get_block_state()
- `is_hole()` - Use context.is_hole()

## Phase 2: OcclusionCache (Global LRU Cache)

**File:** `steel-core/src/fluid/occlusion_cache.rs` (new file)

**Structure:**
```rust
use rustc_hash::FxHashMap;
use std::cell::RefCell;

/// Key for occlusion cache: (from_state_id, to_state_id, direction)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct OcclusionKey {
    from_state: BlockStateId,
    to_state: BlockStateId,
    direction: Direction,
}

/// Occlusion cache entry with LRU tracking
struct CacheEntry {
    value: bool,
    last_access: u64,
}

thread_local! {
    static OCCLUSION_CACHE: RefCell<OcclusionCache> = RefCell::new(OcclusionCache::new());
}

pub struct OcclusionCache {
    cache: FxHashMap<OcclusionKey, CacheEntry>,
    access_counter: u64,
    max_size: usize,
}

impl OcclusionCache {
    const MAX_SIZE: usize = 200;
    
    fn new() -> Self {
        Self {
            cache: FxHashMap::default(),
            access_counter: 0,
            max_size: Self::MAX_SIZE,
        }
    }
    
    /// Check if fluid can pass through wall between two block states
    pub fn can_pass_through_cached(
        from_state: BlockStateId,
        to_state: BlockStateId,
        direction: Direction,
    ) -> Option<bool> {
        OCCLUSION_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let key = OcclusionKey { from_state, to_state, direction };
            
            if let Some(entry) = cache.cache.get_mut(&key) {
                entry.last_access = cache.access_counter;
                cache.access_counter += 1;
                return Some(entry.value);
            }
            None
        })
    }
    
    /// Store result in cache
    pub fn store_result(
        from_state: BlockStateId,
        to_state: BlockStateId,
        direction: Direction,
        result: bool,
    ) {
        OCCLUSION_CACHE.with(|cache| {
            let mut cache = cache.borrow_mut();
            let key = OcclusionKey { from_state, to_state, direction };
            
            // Evict oldest if at capacity
            if cache.cache.len() >= cache.max_size {
                cache.evict_oldest();
            }
            
            cache.cache.insert(key, CacheEntry {
                value: result,
                last_access: cache.access_counter,
            });
            cache.access_counter += 1;
        })
    }
    
    fn evict_oldest(&mut self) {
        if let Some(oldest_key) = self.cache
            .iter()
            .min_by_key(|(_, entry)| entry.last_access)
            .map(|(key, _)| *key) {
            self.cache.remove(&oldest_key);
        }
    }
}
```

**Integration in can_pass_through_wall():**
```rust
pub fn can_pass_through_wall(
    from_state: BlockStateId,
    to_state: BlockStateId,
    direction: Direction,
) -> bool {
    // Check cache first
    if let Some(cached) = OcclusionCache::can_pass_through_cached(from_state, to_state, direction) {
        return cached;
    }
    
    // Calculate result
    let result = calculate_can_pass(from_state, to_state, direction);
    
    // Store in cache (skip for dynamic shapes)
    if !is_dynamic_shape(from_state) && !is_dynamic_shape(to_state) {
        OcclusionCache::store_result(from_state, to_state, direction, result);
    }
    
    result
}
```

## Files to Create/Modify

### New Files:
1. `steel-core/src/fluid/spread_context.rs` - Local spread calculation cache
2. `steel-core/src/fluid/occlusion_cache.rs` - Global LRU occlusion cache

### Modified Files:
1. `steel-core/src/fluid/flowing.rs`:
   - Add SpreadContext usage in get_spread()
   - Modify get_slope_distance() signature
   - Integrate OcclusionCache in can_pass_through_wall()
   
2. `steel-core/src/fluid/mod.rs`:
   - Export new cache modules

3. `steel-core/src/fluid/water.rs` & `lava.rs`:
   - Update calls to use SpreadContext

## Implementation Order

1. **SpreadContext first** - Easier, immediate benefit for get_spread()
2. **OcclusionCache second** - More complex, but caches expensive shape operations

## Expected Performance Gains

- **SpreadContext**: Reduces world lookups from ~20-30 to ~6 per get_spread() call
- **OcclusionCache**: Eliminates redundant shape collision calculations
- **Combined**: Should significantly reduce CPU usage for fluid ticking

## Questions for User

1. Should I implement SpreadContext first, or both at once?
2. Is 200 entries for OCCLUSION_CACHE appropriate, or should we tune this?
3. Should the caches be behind a feature flag for benchmarking?
