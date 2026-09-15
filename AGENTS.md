This is a plugin for [Herdr](https://github.com/herdrdev/herdr).

## Code Style
- You MUST use meaningful function names.
- Write "How" in the code,"What" in the test code,"Why" in the commit log, and "Why not" in the code comments.

## CI
You MUST execute the following commands when you change any .rs files.

1. cargo fmt --all --check
2. cargo clippy --all-targets --all-features -- -D warnings
3. cargo test --all-features --locked

If you find any errors on above commands, you MUST fix it.
