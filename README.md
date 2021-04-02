# Fates

[![Crates.io](https://img.shields.io/crates/v/fates.svg)](https://crates.io/crates/fates)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/d/fates.svg)](https://crates.io/crates/fates)

This crate provides the Fate type, which can be used to create threadsafe reactive declarations. Fate instances that depend on other Fate values will automatically update their values when a dependency is altered. The crate also includes a macro for automatically creating and updating Fate instances.

## Examples

Basic usage:
```rust
use fates::{fate, Fate};

fate! {
  let a = 5;
  let b = a * 3;
}
assert_eq!(b.get(), 15);
fate! {a = 7;}
assert_eq!(a.get(), 7);
assert_eq!(b.get(), 21);
```

You can also capture Copy types in a fate macro:
```rust
let a = 1;
let b = 10;
let c = 15;
fate! {
  [a, b, c] // Non-Fate types
  let d = a + b; // 1 + 10
  let e = d * c; // 11 * 15
}
assert_eq!(e.get(), 11 * 15);
fate! {[a] d = a;}
assert_eq!(e.get(), 15);
```

Finally, if you need to store a Fate instance:
```rust
struct TestStruct {
  fate: Fate<i32>,
}
fate! {
  let a = 10;
}
let test_struct = TestStruct { fate: a.clone() };
assert_eq!(a.get(), 10);
fate! {a = 15;}
assert_eq!(test_struct.fate.get(), 15);
```

## Inspirations

- The "destiny operator": https://paulstovell.com/reactive-programming/.
- Reactive declarations in [Svelte](https://svelte.dev/).
