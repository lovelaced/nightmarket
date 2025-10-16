#!/bin/bash

# Build script for Nightmarket ZK circuits
# Based on fragments circuit build system

set -e

echo "🔧 Building Nightmarket ZK circuits..."

# Add cargo bin to PATH (for circom if installed via cargo)
export PATH=$HOME/.cargo/bin:$PATH

# Create build directory
mkdir -p build

# Install circomlib if not present
if [ ! -d "node_modules/circomlib" ]; then
    echo "📦 Installing circomlib..."
    npm install
fi

# Function to build a circuit
build_circuit() {
    CIRCUIT_NAME=$1
    echo ""
    echo "⚡ Building $CIRCUIT_NAME circuit..."

    # Compile circuit
    echo "  → Compiling circuit..."
    circom $CIRCUIT_NAME.circom --r1cs --wasm --sym -o build/

    # Setup ceremony (for production, use a real ceremony)
    echo "  → Running trusted setup..."
    snarkjs powersoftau new bn128 14 build/pot14_0000.ptau -v > /dev/null 2>&1
    echo "random_entropy_$CIRCUIT_NAME" | snarkjs powersoftau contribute build/pot14_0000.ptau build/pot14_0001.ptau --name="First contribution" -e="entropy" > /dev/null 2>&1
    snarkjs powersoftau prepare phase2 build/pot14_0001.ptau build/pot14_final.ptau -v > /dev/null 2>&1

    # Generate zkey
    echo "  → Generating proving key..."
    snarkjs groth16 setup build/$CIRCUIT_NAME.r1cs build/pot14_final.ptau build/${CIRCUIT_NAME}_0000.zkey > /dev/null 2>&1
    echo "zkey_entropy_$CIRCUIT_NAME" | snarkjs zkey contribute build/${CIRCUIT_NAME}_0000.zkey build/${CIRCUIT_NAME}_0001.zkey --name="1st Contributor" -e="entropy" > /dev/null 2>&1

    # Export verification key
    echo "  → Exporting verification key..."
    snarkjs zkey export verificationkey build/${CIRCUIT_NAME}_0001.zkey build/${CIRCUIT_NAME}_verification_key.json

    # Rename final zkey
    mv build/${CIRCUIT_NAME}_0001.zkey build/${CIRCUIT_NAME}.zkey

    # Calculate VK hash
    echo "  → Calculating VK hash..."
    node -e "
        const fs = require('fs');
        const crypto = require('crypto');
        const vkey = JSON.parse(fs.readFileSync('build/${CIRCUIT_NAME}_verification_key.json'));
        const vkeyStr = JSON.stringify(vkey);
        const hash = crypto.createHash('sha256').update(vkeyStr).digest('hex');
        fs.writeFileSync('build/${CIRCUIT_NAME}_vk_hash.txt', '0x' + hash);
        console.log('  VK Hash: 0x' + hash);
    "

    # Clean up intermediate files
    rm -f build/${CIRCUIT_NAME}_0000.zkey
    rm -f build/pot14_0000.ptau build/pot14_0001.ptau build/pot14_final.ptau

    echo "  ✅ $CIRCUIT_NAME circuit built successfully!"
}

# Build all circuits
build_circuit "location_proof"
build_circuit "mixer_withdrawal"
build_circuit "reputation_threshold"

echo ""
echo "✨ All circuits built successfully!"
echo ""
echo "📁 Build artifacts:"
echo "  - WASM files: build/*_js/"
echo "  - R1CS files: build/*.r1cs"
echo "  - ZKey files: build/*.zkey"
echo "  - Verification keys: build/*_verification_key.json"
echo "  - VK hashes: build/*_vk_hash.txt"
echo ""
echo "Next step: node extract_vk_hashes.js to get Rust-formatted VK hashes"
