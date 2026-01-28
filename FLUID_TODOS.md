# Fluid System TODOs - Priority List

This file tracks all TODOs related to the fluid system implementation.
See FLUID_REVIEW.md for detailed analysis.

## 🔴 HIGH PRIORITY (Gameplay Breaking)

### 1. Lava-Water Chemistry (CRITICAL)
**Files:** 
- `steel-core/src/behavior/blocks/liquid_block.rs` (lines 8-16, 55, 73)
- `steel-core/src/fluid/lava.rs` (spread_to_sides)

**Description:** 
- Lava adjacent to water should convert to obsidian (source) or cobblestone (flowing)
- Direction check: DOWN, SOUTH, NORTH, EAST, WEST (NOT UP)
- Must be immediate (no tick scheduling after conversion)
- Vanilla: LiquidBlock.shouldSpreadLiquid()

**Current Status:** Disabled due to deadlock issues

**Implementation Notes:**
- Must check in on_place() and handle_neighbor_changed()
- Must NOT schedule fluid tick after conversion
- Must check all 5 directions using get_fluid_state()
- Source lava → obsidian, Flowing lava → cobblestone

### 2. Bucket Stack Support (HIGH)
**Files:**
- `steel-core/src/behavior/items/bucket.rs` (line 6)

**Description:**
- Stacks of buckets (>1) don't work correctly
- add_item_or_drop() causes lock contention/deadlock
- Need to handle: give_empty_bucket(), give_filled_bucket()

**Current Status:** Disabled/simplified to avoid deadlocks

**Implementation Notes:**
- Option 1: Queue item additions for end of tick
- Option 2: Use existing ContainerLockGuard
- Option 3: Refactor inventory system

### 3. Visual Sync - Infinite Sources (HIGH)
**Files:**
- `steel-core/src/behavior/items/bucket.rs` (line 229)

**Description:**
- When taking water from infinite source, client sees air briefly
- Block updates not forced to interacting player
- Neighbors take 1-5 ticks to regenerate

**Current Status:** Timing issue, works but visually jarring

**Implementation Notes:**
- Need to send forced block update to player who used bucket
- May need new packet type or special handling

## 🟡 MEDIUM PRIORITY (Quality of Life)

### 4. Collision Shape Checks (canPassThroughWall)
**Files:**
- `steel-core/src/fluid/flowing.rs` (line 235)

**Description:**
- Current: Simplified check (replaceable/air only)
- Vanilla: Full VoxelShape collision detection
- Blocks fluids flowing through solid blocks incorrectly

**Implementation:**
- Use BlockState.getCollisionShape()
- Check Shapes.mergedFaceOccludes()
- Add OCCLUSION_CACHE for performance

### 5. Block Type Exclusions (canHoldAnyFluid)
**Files:**
- `steel-core/src/fluid/flowing.rs` (lines 258-287)

**Description:**
- Doors, signs, ladders shouldn't hold fluid
- Current: Simplified check only
- Vanilla: Specific block type checks + tags

**Blocks to Exclude:**
- BlockTags.DOORS
- BlockTags.ALL_SIGNS
- LADDER
- SUGAR_CANE
- BUBBLE_COLUMN
- NETHER_PORTAL
- END_PORTAL
- END_GATEWAY
- STRUCTURE_VOID

### 6. Sound Effects
**Files:**
- `steel-core/src/behavior/items/bucket.rs` (lines 7-8)
- `steel-core/src/fluid/lava.rs` (line 85, 186)
- `steel-core/src/fluid/water.rs` (line 147)

**Sounds Needed:**
- BUCKET_FILL (water)
- BUCKET_FILL_LAVA
- BUCKET_EMPTY (water)
- BUCKET_EMPTY_LAVA
- Level event 1501 (fizz - lava + water)

### 7. Particle Effects
**Files:**
- `steel-core/src/behavior/items/bucket.rs` (line 8)

**Particles Needed:**
- SPLASH (water bucket empty)
- LAVA (lava bucket empty)
- DRIPPING_WATER
- DRIPPING_LAVA
- Lava pop particles (animateTick)

### 8. Dimension-Based Lava Speed
**Files:**
- `steel-core/src/fluid/lava.rs` (tick_delay)

**Description:**
- Nether: 10 tick delay (fast)
- Overworld/End: 30 tick delay (slow)
- Vanilla: isFastLava() dimension check

**Current:** Hardcoded 30 ticks

### 9. Spread Delay Randomization
**Files:**
- `steel-core/src/fluid/lava.rs`

**Description:**
- 25% chance of 4x slower flow
- Adds visual variety
- Vanilla: getSpreadDelay() with randomization

## 🟢 LOW PRIORITY (Polish)

### 10. Waterlogging Support
**Files:**
- `steel-core/src/behavior/items/bucket.rs` (line 9)

**Description:**
- Stairs, slabs, fences can be waterlogged
- Water flows into waterloggable block → waterlogs it
- Bucket takes water from waterlogged block

### 11. Entity Interactions
**Files:**
- `steel-core/src/fluid/water.rs`
- `steel-core/src/fluid/lava.rs`

**Water:**
- Extinguish burning entities
- Push force

**Lava:**
- Fire damage
- Set entities on fire
- Slow movement

### 12. Performance Optimizations
**Files:**
- `steel-core/src/fluid/flowing.rs`

**Optimizations:**
- SpreadContext caching (Short2ObjectMap)
- OCCLUSION_CACHE for wall pass-through
- FluidState registry (IdMapper)

### 13. Advanced Features

#### 13a. Blue Ice + Soul Soil = Basalt
**Files:**
- `steel-core/src/fluid/lava.rs` (randomTick)

**Condition:**
- Lava above soul soil
- Adjacent to blue ice
- Converts to basalt

#### 13b. Cauldron Filling
- Rain fills cauldrons
- Lava in cauldrons (nether)

#### 13c. Bubble Columns
- Water above soul sand/magma
- Entity pushing/pulling

## 📋 CODE ORGANIZATION TODOs

### File Structure
- `steel-core/src/fluid/mod.rs`: Consider submodule organization
- `steel-core/src/fluid/flowing.rs`: Split into smaller files:
  - fluid_state.rs
  - fluid_trait.rs
  - spread_logic.rs
  - collision.rs

### Documentation
- Add module-level docs for each public function
- Document algorithm choices
- Add examples

### Testing
- Add unit tests in tests/ directory
- Test spread algorithms
- Test source conversion
- Test edge cases

## ✅ COMPLETED

- ✅ Basic spread mechanics
- ✅ Source conversion (2+ sources)
- ✅ Game rule support
- ✅ Bucket basic mechanics
- ✅ Fluid state encoding
- ✅ Tick scheduling
- ✅ Slope finding
- ✅ SteelExtractor integration

## 🔧 ARCHITECTURAL DECISIONS

### FluidId Type
**Current:** u16 matching vanilla IDs
**Status:** ✅ Good, matches vanilla

### FluidState Struct
**Current:** Simple struct { fluid, level, falling }
**Status:** ✅ Good, more efficient than vanilla StateHolder

### Tick-Based System
**Current:** Schedule ticks with delays
**Status:** ✅ Good, matches vanilla approach

## 🎯 ESTIMATED EFFORT

| Task | Complexity | Estimated Time |
|------|-----------|----------------|
| Lava-water chemistry | High | 2-3 days |
| Bucket stack fix | High | 1-2 days |
| Visual sync | Medium | 1 day |
| Collision shapes | Medium | 2-3 days |
| Block exclusions | Low | 1 day |
| Sound effects | Low | 1 day |
| Particles | Low | 1 day |
| Dimension speed | Low | 2 hours |
| Randomization | Low | 1 hour |
| Waterlogging | Medium | 2-3 days |
| Entity effects | Medium | 2-3 days |
| Performance | Medium | 2-3 days |
| Advanced features | Low | 1-2 days |

**Total:** ~15-20 days of work for full parity
