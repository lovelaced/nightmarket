#!/usr/bin/env bash
set -euo pipefail

echo "Compiling Nightmarket ZK Circuits..."
echo

# Check circom is installed
if ! command -v circom &> /dev/null; then
    echo "Error: circom not found. Install with:"
    echo "  git clone https://github.com/iden3/circom.git"
    echo "  cd circom && cargo build --release && cargo install --path circom"
    exit 1
fi

# Create build directories
mkdir -p build
mkdir -p build/location_proof
mkdir -p build/mixer_withdrawal
mkdir -p build/reputation_threshold

# 1. Compile Location Proof Circuit
echo "Compiling location_proof.circom..."
circom location_proof.circom \
    --r1cs \
    --wasm \
    --sym \
    --output build/location_proof
echo "✓ Location proof circuit compiled"
echo "  - Constraints: $(grep -o 'n8 .*' build/location_proof/location_proof.r1cs | head -1 || echo 'unknown')"
echo

# 2. Compile Mixer Withdrawal Circuit
echo "Compiling mixer_withdrawal.circom..."
circom mixer_withdrawal.circom \
    --r1cs \
    --wasm \
    --sym \
    --output build/mixer_withdrawal
echo "✓ Mixer withdrawal circuit compiled"
echo "  - Constraints: $(grep -o 'n8 .*' build/mixer_withdrawal/mixer_withdrawal.r1cs | head -1 || echo 'unknown')"
echo

# 3. Compile Reputation Threshold Circuit
echo "Compiling reputation_threshold.circom..."
circom reputation_threshold.circom \
    --r1cs \
    --wasm \
    --sym \
    --output build/reputation_threshold
echo "✓ Reputation threshold circuit compiled"
echo "  - Constraints: $(grep -o 'n8 .*' build/reputation_threshold/reputation_threshold.r1cs | head -1 || echo 'unknown')"
echo

echo "All circuits compiled successfully!"
echo "Output directory: build/"
echo
echo "Next steps:"
echo "1. Run setup_circuits.sh to generate proving/verification keys"
echo "2. Run test_circuits.js to verify circuit correctness"
