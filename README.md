# Fates

[![Crates.io](https://img.shields.io/crates/v/fates.svg)](https://crates.io/crates/fates)
[![license](https://img.shields.io/badge/license-MIT-blue.svg)](./LICENSE)
[![Crates.io](https://img.shields.io/crates/d/fates.svg)](https://crates.io/crates/fates)

This crate provides the Fate type, which can be used to create thread-safe reactive declarations. Fate instances that depend on other Fate values will automatically update their values when a dependency is altered. The crate also includes a macro for automatically creating and updating Fate instances.

**Disclaimer:** Extremely alpha, possibly broken, definitely changing.

## Examples

### Updating text:
```rust
fate! {
  [name, hello, goodbye]
  let name = "Alex".to_string();
  let hello = "Hello, ".to_string() + &name;
  let goodbye = "Goodbye, ".to_string() + &name;
}
assert_eq!(&hello.get(), "Hello, Alex");
assert_eq!(&goodbye.get(), "Goodbye, Alex");

fate! {[name] name = "Sam".to_string();}
assert_eq!(&hello.get(), "Hello, Sam");
assert_eq!(&goodbye.get(), "Goodbye, Sam");
```

### Math expressions:
```rust
use fates::{fate, Fate};

fate! {
  [a, b] // Which types should be Fate types
  let a = 5;
  let b = a * 3;
}
assert_eq!(b.get(), 15);
fate! {a = 7;}
assert_eq!(a.get(), 7);
assert_eq!(b.get(), 21);
```

### Copy types:
```rust
let a = 1;
let b = 10;
let c = 15;
fate! {
  [d, e] // Fate types
  let d = a + b; // 1 + 10
  let e = d * c; // 11 * 15
}
assert_eq!(e.get(), 11 * 15);
fate! {[d] d = a;}
assert_eq!(e.get(), 15);
```

### Accessing or mutating bound values by reference:
```rust
fate! {
  [a]
  let a = vec![1, 2, 3];
}
assert_eq!(a.get(), vec![1, 2, 3]);

a.by_ref_mut(|a| a.push(4));
assert_eq!(a.get(), vec![1, 2, 3, 4]);

let mut val = 2;
a.by_ref(|a| val = a[2]);
assert_eq!(val, 3);
```
If an expression is bound, by_ref / by_ref_mut will not run the supplied function. You can check for this:
```rust
let mut is_value = false;
a.by_ref_mut(|a| {
  is_value = true;
  a.push(5);
});
```

### Storing and manually updating a Fate instance:
```rust
struct TestStruct {
  fate: Fate<i32>,
}
fate! {
  [a]
  let a = 10;
}

let test_struct = TestStruct { fate: a.clone() };
assert_eq!(a.get(), 10);

fate! {[a] a = 15;}
assert_eq!(test_struct.fate.get(), 15);

// Alternatively:
a.bind_value(200);
assert_eq!(test_struct.fate.get(), 200);
```

## Inspirations

- The "destiny operator": https://paulstovell.com/reactive-programming/.
- Reactive declarations in [Svelte](https://svelte.dev/).
