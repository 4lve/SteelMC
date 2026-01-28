//! FLUID SYSTEM - VANILLA PARITY REVIEW
//!
//! This file documents the current state of the fluid system implementation
//! compared to vanilla Minecraft, and lists all TODOs for future improvements.

/*
================================================================================
                    VANILLA vs STEELMC FLUID SYSTEM
================================================================================

## ✅ CORRECTLY IMPLEMENTED

### Core Mechanics
- ✅ Basic spread algorithm (getNewLiquid, getSpread)
- ✅ Source conversion (2+ sources + solid below)
- ✅ Game rule support (waterSourceConversion, lavaSourceConversion)
- ✅ Falling fluid (level 8 encoding)
- ✅ Tick delays (Water=5, Lava=30)
- ✅ Drop-off (Water=1, Lava=2)
- ✅ Slope finding (Water=4, Lava=2)
- ✅ Fluid IDs match vanilla (u16: Empty=0, Flowing_Water=1, Water=2, Flowing_Lava=3, Lava=4)

### Bucket Mechanics
- ✅ Basic place/pickup
- ✅ Source detection (level 0)
- ✅ Empty bucket fills from source
- ✅ Filled bucket places fluid
- ✅ Game mode checks (creative doesn't consume)

### Fluid State
- ✅ FluidState struct with level, falling, fluid type
- ✅ Proper encoding/decoding (level 0=source, 1-7=flowing, 8=falling)
- ✅ Block state <-> fluid state conversion

## ❌ MISSING / NOT IMPLEMENTED

### Critical Missing Features

#### 1. Lava-Water Chemistry (OBSIDIAN/COBBLESTONE)
**Vanilla:** LiquidBlock.shouldSpreadLiquid()
- Lava adjacent to water → immediate conversion
- Source lava + water → obsidian
- Flowing lava + water → cobblestone
- Direction: DOWN, SOUTH, NORTH, EAST, WEST (NOT UP!)
- NO fluid tick scheduled after conversion

**Current:** Basic spread interaction only (lava flowing INTO water)
**Status:** ❌ Not implemented (caused deadlocks, disabled)
**TODOs:** See liquid_block.rs lines 8-16, 55, 73

#### 2. Collision Shape Checks (canPassThroughWall)
**Vanilla:** FlowingFluid.canPassThroughWall()
- Uses VoxelShape collision detection
- Shapes.block() = full block (can't pass)
- Shapes.empty() = empty (can pass)
- Shapes.mergedFaceOccludes() for face occlusion
- OCCLUSION_CACHE for performance

**Current:** Simplified check (replaceable/air only)
**TODO:** flowing.rs line 235

#### 3. Block Type Exclusions (canHoldAnyFluid)
**Vanilla:** FlowingFluid.canHoldAnyFluid()
- Returns false for: doors, signs, ladders, sugar cane, bubble columns
- Portals (nether, end), end gateway, structure void
- Checks BlockTags.DOORS, BlockTags.ALL_SIGNS

**Current:** Simplified (collision/replaceable check only)
**TODOs:** flowing.rs lines 258-287

#### 4. Waterlogging Support
**Vanilla:** Blocks like stairs, slabs can be waterlogged
- Water flows into waterloggable block → waterlogs it
- Bucket takes water from waterlogged block → removes water, keeps block

**Current:** Not implemented
**TODO:** bucket.rs line 9

#### 5. Dimension-Based Lava Speed
**Vanilla:** LavaFluid.isFastLava()
- Nether: 10 tick delay (fast)
- Overworld/End: 30 tick delay (slow)
- isUltrawarm dimension check

**Current:** Hardcoded 30 ticks
**TODO:** Add dimension check

#### 6. Spread Delay Randomization
**Vanilla:** LavaFluid.getSpreadDelay()
- 25% chance of 4x slower flow (random)
- Adds visual variety

**Current:** Fixed delay
**TODO:** Add randomization

### Audio/Visual Effects

#### 7. Sound Effects
**Vanilla:**
- Bucket fill: BUCKET_FILL (water), BUCKET_FILL_LAVA (lava)
- Bucket empty: BUCKET_EMPTY (water), BUCKET_EMPTY_LAVA (lava)
- Fizz sound: Level event 1501 (lava + water)

**Current:** Silent
**TODOs:** bucket.rs lines 7-8, flowing.rs lines 85, 186

#### 8. Particle Effects
**Vanilla:**
- Water dripping: DRIPPING_WATER
- Lava dripping: DRIPPING_LAVA
- Splash particles on place
- Lava pop particles (animateTick)

**Current:** No particles
**TODO:** bucket.rs line 8

#### 9. Underwater Effects
**Vanilla:**
- Fog color change
- Visibility reduction
- Bubble particles

**Current:** Not implemented
**TODO:** Client-side rendering

### Entity Interactions

#### 10. entityInside() - Entity Effects
**Vanilla WaterFluid:**
- Extinguishes burning entities
- Applies water push force

**Vanilla LavaFluid:**
- Deals fire damage
- Sets entities on fire
- Slows movement

**Current:** Not implemented
**TODO:** Add entity effects

#### 11. Entity Extinguishing
**Vanilla:** Entity.extinguish() when touching water
- Removes fire status
- Plays sound

**Current:** Not implemented

### Performance Optimizations

#### 12. SpreadContext Caching
**Vanilla:** FlowingFluid.SpreadContext
- Caches block states during spread calculation
- Avoids repeated world lookups
- Short2ObjectMap for performance

**Current:** No caching
**TODO:** flowing.rs - add caching

#### 13. OCCLUSION_CACHE
**Vanilla:** FlowingFluid.OCCLUSION_CACHE
- Caches wall pass-through results
- Keyed by block shape combinations

**Current:** No caching
**TODO:** flowing.rs line 235

#### 14. FluidState Registry
**Vanilla:** IdMapper<FluidState> FLUID_STATE_REGISTRY
- Maps fluid states to IDs for serialization
- Efficient network transmission

**Current:** Direct conversion
**TODO:** Add registry

### Advanced Features

#### 15. Blue Ice + Soul Soil = Basalt
**Vanilla:** LavaFluid.randomTick()
- Lava above soul soil
- Adjacent to blue ice
- Converts to basalt

**Current:** Not implemented
**TODO:** Add to LavaFluid

#### 16. Cauldron Filling
**Vanilla:** 
- Rain fills cauldrons with water
- Lava fills cauldrons in nether

**Current:** Not implemented
**TODO:** Add cauldron support

#### 17. Bubble Columns
**Vanilla:**
- Water above soul sand/magma block
- Creates bubble column effect
- Entity pushing/pulling

**Current:** Not implemented

## ⚠️ PARTIALLY IMPLEMENTED

### Bucket Stack Support
**Current:** Disabled (deadlock issues)
- Stacks of buckets don't work correctly
- add_item_or_drop causes lock contention
**TODO:** bucket.rs line 6

### Visual Sync Issues
**Current:** Infinite water sources don't update visually immediately
- Client sees air briefly before regeneration
- Block updates not forced to interacting player
**TODO:** bucket.rs line 229

### Fire Spread from Lava
**Current:** Disabled (randomTick in LiquidBlockBehavior)
- Was causing performance issues
- Needs careful implementation
**TODO:** liquid_block.rs

## 📝 ARCHITECTURAL NOTES

### Good Design Decisions ✅
1. Separate FluidBehaviour trait for different fluids
2. FluidState as simple struct (not StateHolder like vanilla)
3. SteelExtractor integration for fluid IDs
4. Tick-based scheduling system

### Areas for Improvement ⚠️
1. **FluidId constants naming** - Should match vanilla (WATER vs Water)
2. **File organization** - flowing.rs is too large (800+ lines)
3. **Documentation** - Many private methods lack docs
4. **Unit tests** - No test coverage

### Rust-Specific Considerations
1. **Lock contention** - add_item_or_drop deadlocks
2. **Memory layout** - FluidState is well-optimized (3 fields)
3. **Trait design** - FluidBehaviour works well

## 🎯 PRIORITY ROADMAP

### High Priority (Gameplay Critical)
1. Lava-water interaction (obsidian/cobblestone) ❌
2. Bucket stack support without deadlocks ❌
3. Visual sync for infinite sources ⚠️

### Medium Priority (Quality of Life)
4. Sound effects ❌
5. Particle effects ❌
6. Waterlogging ❌
7. Dimension-based lava speed ❌

### Low Priority (Polish)
8. Performance optimizations (caching) ❌
9. Advanced features (basalt, bubble columns) ❌
10. Entity interactions ❌

## 📊 PARITY SCORE

| Category | Score | Notes |
|----------|-------|-------|
| Core Mechanics | 85% | Good foundation |
| Bucket System | 60% | Works, stacks broken |
| Audio/Visual | 10% | Silent, no particles |
| Entity Effects | 0% | Not implemented |
| Performance | 40% | No caching |
| Advanced | 20% | Basic features only |

**Overall: ~50% Vanilla Parity**

================================================================================
*/

// This is a documentation-only file. No code should be added here.
// TODOs in the codebase reference this review.
