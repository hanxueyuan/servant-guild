#!/bin/bash
# Simple syntax verification script for Rust code

# Check coordinator.rs syntax
echo "Checking coordinator.rs syntax..."
cargo check --package zeroclaw --lib 2>&1 | grep -E "error|warning" | head -50

# Check other servant files
echo "Checking servant modules..."
for file in src/servants/*.rs; do
    echo "Checking $file..."
    rustc --crate-type lib --edition 2021 "$file" 2>&1 | head -20
done

echo "Syntax verification complete"
