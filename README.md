# bpe-rs
bpe-rs is an implementation of Philip Gage's Byte Pair Encoding in Rust, primarily used for binary file compression and decompression.

## Capabilities
- [X] Decode
- [X] Encode

## Usage
### Decoding
```rust
let encoded = fs::read("encoded_file.bin")?;
let decoded = bpe::decode(&encoded, bpe::DEFAULT_STACK_SIZE);
```

### Encoding
```rust
let decoded = fs::read("decoded_file.txt")?;
let encoded = bpe::encode(&decoded);
```
