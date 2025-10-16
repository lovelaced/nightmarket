#!/bin/bash
set -e

if [ -z "$1" ]; then
    echo "Usage: ./build_single.sh <contract_name>"
    echo "Available contracts:"
    echo "  - zones"
    echo "  - listings"
    echo "  - escrow"
    echo "  - mixer"
    echo "  - reputation"
    exit 1
fi

case "$1" in
    zones)
        PACKAGE="nightmarket-zones"
        BINARY="nightmarket_zones"
        ;;
    listings)
        PACKAGE="nightmarket-listings"
        BINARY="nightmarket_listings"
        ;;
    escrow)
        PACKAGE="nightmarket-escrow"
        BINARY="nightmarket_escrow"
        ;;
    mixer)
        PACKAGE="nightmarket-mixer"
        BINARY="nightmarket_mixer"
        ;;
    reputation)
        PACKAGE="nightmarket-reputation"
        BINARY="nightmarket_reputation"
        ;;
    *)
        echo "Unknown contract: $1"
        exit 1
        ;;
esac

echo "Building $PACKAGE..."
cargo build --release --bin "$BINARY"

echo "Linking $PACKAGE..."
mkdir -p build
polkatool link --strip \
    --output "build/${BINARY}.polkavm" \
    "target/riscv64emac-unknown-none-polkavm/release/${BINARY}"

echo "✓ $PACKAGE built successfully: build/${BINARY}.polkavm"
