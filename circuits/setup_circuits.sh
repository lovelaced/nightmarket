#!/usr/bin/env bash
set -euo pipefail

echo "Setting up Nightmarket ZK Circuits (Powers of Tau + Groth16 keys)..."
echo

# Check if circuits are compiled
if [ ! -d "build/location_proof" ] || [ ! -d "build/mixer_withdrawal" ] || [ ! -d "build/reputation_threshold" ]; then
    echo "Error: Circuits not compiled. Run ./compile_circuits.sh first"
    exit 1
fi

# Create keys directory
mkdir -p build/keys

# Check if we have a powers of tau file, if not download one
PTAU_FILE="build/keys/powersOfTau28_hez_final_14.ptau"
if [ ! -f "$PTAU_FILE" ]; then
    echo "Downloading Powers of Tau ceremony file (2^14 constraints)..."
    echo "This is a one-time ~50MB download from the Hermez trusted setup ceremony"
    curl -L -o "$PTAU_FILE" \
        "https://hermez.s3-eu-west-1.amazonaws.com/powersOfTau28_hez_final_14.ptau"
    echo "✓ Powers of Tau file downloaded"
    echo
fi

# Function to setup a circuit
setup_circuit() {
    CIRCUIT_NAME=$1
    CIRCUIT_DIR="build/$CIRCUIT_NAME"

    echo "Setting up $CIRCUIT_NAME..."

    # Generate zkey (Groth16 proving key)
    echo "  Generating proving key..."
    npx snarkjs groth16 setup \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}.r1cs" \
        "$PTAU_FILE" \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_0000.zkey" \
        > /dev/null

    # Contribute to phase 2 ceremony (adds randomness)
    echo "  Adding randomness (phase 2)..."
    npx snarkjs zkey contribute \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_0000.zkey" \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_final.zkey" \
        --name="Nightmarket contribution" \
        -e="$(openssl rand -hex 32)" \
        > /dev/null

    # Export verification key
    echo "  Exporting verification key..."
    npx snarkjs zkey export verificationkey \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_final.zkey" \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_vkey.json"

    # Generate Solidity verifier (for reference, PolkaVM uses different format)
    echo "  Generating Solidity verifier..."
    npx snarkjs zkey export solidityverifier \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_final.zkey" \
        "$CIRCUIT_DIR/${CIRCUIT_NAME}_verifier.sol" \
        > /dev/null

    # Calculate verification key hash for contract
    echo "  Calculating VK hash..."
    node -e "
        const fs = require('fs');
        const crypto = require('crypto');
        const vkey = JSON.parse(fs.readFileSync('$CIRCUIT_DIR/${CIRCUIT_NAME}_vkey.json'));
        const vkeyStr = JSON.stringify(vkey);
        const hash = crypto.createHash('sha256').update(vkeyStr).digest('hex');
        fs.writeFileSync('$CIRCUIT_DIR/${CIRCUIT_NAME}_vk_hash.txt', '0x' + hash);
        console.log('  VK Hash: 0x' + hash);
    "

    # Clean up intermediate files
    rm -f "$CIRCUIT_DIR/${CIRCUIT_NAME}_0000.zkey"

    echo "✓ $CIRCUIT_NAME setup complete"
    echo
}

# Setup each circuit
setup_circuit "location_proof"
setup_circuit "mixer_withdrawal"
setup_circuit "reputation_threshold"

echo
echo "All circuits setup successfully!"
echo
echo "Generated files:"
echo "  - build/*/[circuit]_final.zkey    - Proving keys"
echo "  - build/*/[circuit]_vkey.json     - Verification keys"
echo "  - build/*/[circuit]_vk_hash.txt   - VK hashes for contracts"
echo "  - build/*/[circuit]_verifier.sol  - Solidity verifiers (reference)"
echo
echo "Next steps:"
echo "1. Update contracts with VK hashes from build/*/[circuit]_vk_hash.txt"
echo "2. Test circuits with test_circuits.js"
echo "3. Integrate proving keys into frontend"
