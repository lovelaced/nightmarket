#!/bin/bash
set -e

echo "Building Nightmarket Contracts..."
echo

# Array of contract names
contracts=(
    "nightmarket-zones:nightmarket_zones"
    "nightmarket-listings:nightmarket_listings"
    "nightmarket-escrow:nightmarket_escrow"
    "nightmarket-mixer:nightmarket_mixer"
    "nightmarket-reputation:nightmarket_reputation"
)

# Create output directory
mkdir -p build

# Build each contract
for contract_pair in "${contracts[@]}"; do
    IFS=':' read -r contract_name binary_name <<< "$contract_pair"

    echo "Building $contract_name..."
    cargo build --release --bin "$binary_name"

    echo "Linking $contract_name..."
    polkatool link --strip \
        --output "build/${binary_name}.polkavm" \
        "target/riscv64emac-unknown-none-polkavm/release/${binary_name}"

    echo "✓ $contract_name built successfully"
    echo
done

echo "All contracts built successfully!"
echo "Output directory: build/"
ls -lh build/
