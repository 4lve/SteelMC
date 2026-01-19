# Adding a New Block (Basic Setup)

> ⚠️ This is only the very basic setup and **does not provide any functionality yet**.

---

## 1. Register the Block Module

Add your block module to:

```
steel-core/src/behavior/blocks/mod.rs
```

It should look like this:

```rust
mod iron_bars_block;
pub use iron_bars_block::IronBarsBlock;
```

---

## 2. Verify the Struct Name

Double-check that your **struct name is correct**, as it must match everywhere it is referenced.

---

## 3. Add the Struct to the Generated Blocks

Now we need to add the struct to the generated block list.
This happens in:

```
steel-core/build/blocks.rs
```

If you want to understand what is happening internally, the function
`generate_registrations` is interesting to read — but **it is not required** to get your block working.

---

## 4. Focus on the Build Function

We will now focus on the `build` function in the opened file.

⚠️ **Important:**
Only **add** new code.
Do **not remove or modify existing code**, as this could break blocks from other contributors.

---

## 5. Create a Mutable Vector

First, create a mutable vector with a descriptive name:

```rust
let mut iron_bar_blocks = Vec::new();
```

---

## 6. Extend the Match Statement

Add your block struct name to the `match` statement.
Again: **only add your line**, do not remove others.

```rust
for block in blocks {
    let const_ident = to_const_ident(&block.name);
    match block.class.as_str() {
        ...
        "IronBarsBlock" => iron_bar_blocks.push(const_ident),
        _ => {}
    }
}
```

---

## 7. Define the Block Type

Now define the block type identifier:

```rust
let iron_bar_type = Ident::new("IronBarsBlock", Span::call_site());
```

---

## 8. Generate Registrations

Next, generate the registrations:

```rust
let iron_bar_registrations =
    generate_registrations(iron_bar_blocks.iter(), &iron_bar_type);
```

---

## 9. Add the Registrations to the Output

⚠️ **Be very careful here!**

* The `#` before the registration name is **required**
* It prevents name collisions with Rust keywords
* Do **not** add a trailing comma — this code is generated into another file

Example:

```rust
let output = quote! {
    //! Generated block behavior assignments.

    use steel_registry::vanilla_blocks;
    use crate::behavior::BlockBehaviorRegistry;
    use crate::behavior::blocks::{
        CraftingTableBlock,
        CropBlock,
        EndPortalFrameBlock,
        FarmlandBlock,
        FenceBlock,
        RotatedPillarBlock,
        BarBlock
    };

    pub fn register_block_behaviors(registry: &mut BlockBehaviorRegistry) {
        ...
        #iron_bar_registrations
    }
};
```

---

## 10. Compile the Project

Now compile the project and let Rust (and the build system) do its magic.

After compilation, your block should appear in:

```
steel-core/src/behavior/generated/blocks.rs
```

### Troubleshooting

If your block is missing:

1. Delete the `generated` folder
2. Run:

   ```
   cargo clean
   ```
3. Compile again

This usually resolves the issue.

---

# Adding Behavior to the Block

At this point, the block **does nothing**.
To add behavior, you must implement the required methods in `BlockBehaviour` in your block file (e.g. `iron_bars_block.rs`).

👉 A good approach is to look at **existing blocks** with similar behavior and use them as references.

---

## Working with Block States

### Getting a Neighbor Block State

```rust
let west_pos = Direction::West.relative(pos);
let west_state = world.get_block_state(&west_pos);
```

A `BlockState` contains **all information** about that specific block.

---

### Modifying Block State Properties

Set a value:

```rust
state.set_value(&BlockStateProperties::WEST, true);
```

Get a value works the same way, just in reverse.

---

## Checking Neighbor Blocks or Tags

To check whether a neighboring block belongs to a specific block group (e.g. walls or bars):

```rust
let walls_tag = Identifier::vanilla_static("walls");
if REGISTRY.blocks.is_in_tag(neighbor_block, &walls_tag) {
    return true;
}
```

---

That’s it — you now have the **basic structure** in place and can start implementing real behavior 🚀

## Other useful resources
Currently no available