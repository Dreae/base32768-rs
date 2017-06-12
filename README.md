# Base32768-rs
Base32768 is a binary encoding for encoding arbitrary binary data to UTF-16 text.

# Usage
```rust
// Encoding
let data = [72u8, 101u8, 108u8, 108u8, 111u8];
let encoded = base32768::encode(&data).unwrap(); // "䩲腻㐿"

// Decoding
let mut decoded = Vec::<u8>::new();
base32768::decode(&encoded, &mut decoded).unwrap();
println!("{}", String::from_utf8(decoded).unwrap()); // Prints "Hello"
```

# Prior Art
This module is a pretty vanilla port of the original
[JavaScript module](https://github.com/qntm/base32768) to
Rust.