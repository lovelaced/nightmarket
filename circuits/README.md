# Nightmarket ZK Circuits

Zero-knowledge proof circuits for privacy-preserving features in Nightmarket.

## Circuits

### 1. Location Proof (`location_proof.circom`)
**Purpose:** Prove presence in a geographic zone without revealing exact location.

**Private Inputs:**
- `location_lat`, `location_lon`: Exact coordinates (scaled by 1e6)
- `signal_hashes[8]`: WiFi/cellular signal identifiers
- `secret`: Random secret for nullifier derivation
- `zone_lat_min`, `zone_lat_max`, `zone_lon_min`, `zone_lon_max`: Zone boundaries

**Public Inputs:**
- `zone_id`: Geographic zone identifier (uint32)
- `timestamp`: Unix timestamp (uint64)

**Public Outputs:**
- `nullifier`: Prevents proof reuse

**Constraints:**
- Location coordinates within zone boundaries
- Signal fingerprint validity
- Nullifier = poseidon(secret, zone_id, timestamp)

---

### 2. Mixer Withdrawal (`mixer_withdrawal.circom`)
**Purpose:** Prove knowledge of deposit commitment without linking to deposit address.

**Private Inputs:**
- `secret`: Random secret from original deposit commitment
- `amount`: Withdrawal amount in wei

**Public Inputs:**
- `zone_id`: Zone identifier (uint32)

**Public Outputs:**
- `nullifier`: Prevents double-withdrawal

**Constraints:**
- Nullifier = poseidon(secret, zone_id)
- Amount > 0 (prevents zero-value attacks)
- Commitment = poseidon(secret, amount)

---

### 3. Reputation Threshold (`reputation_threshold.circom`)
**Purpose:** Prove reputation score >= threshold without revealing exact score.

**Private Inputs:**
- `score`: Actual reputation score (uint64)
- `secret`: User's reputation secret

**Public Inputs:**
- `zone_id`: Zone identifier (uint32)
- `ephemeral_id`: Nightly ephemeral identity hash
- `threshold`: Minimum required score (uint64)

**Constraints:**
- score >= threshold
- ephemeral_id = poseidon(secret, zone_id)
- Score commitment binds secret to score

---

## Setup

### Prerequisites

1. **Install Circom compiler:**
```bash
git clone https://github.com/iden3/circom.git
cd circom
cargo build --release
cargo install --path circom
```

2. **Install Node.js dependencies:**
```bash
npm install
```

### Compile Circuits

```bash
npm run compile
```

This generates:
- `.r1cs` files (constraint systems)
- `.wasm` files (witness generators)
- `.sym` files (symbol maps)

### Generate Proving/Verification Keys

```bash
npm run setup
```

This performs:
1. Downloads Powers of Tau ceremony file (~50MB, one-time)
2. Generates Groth16 proving keys for each circuit
3. Exports verification keys (JSON format)
4. Calculates VK hashes for contract integration
5. Generates Solidity verifiers (reference)

**Warning:** This can take several minutes depending on circuit complexity.

### Test Circuits

```bash
npm test
```

Runs test cases for all three circuits to verify correctness.

---

## Integration with Contracts

After running `setup_circuits.sh`, update the contract VK hashes:

**Zones Contract:**
```rust
// contracts/nightmarket-zones/src/main.rs
let vk_hash = [/* copy from build/location_proof/location_proof_vk_hash.txt */];
```

**Mixer Contract:**
```rust
// contracts/nightmarket-mixer/src/main.rs
let vk_hash = [/* copy from build/mixer_withdrawal/mixer_withdrawal_vk_hash.txt */];
```

**Reputation Contract:**
```rust
// contracts/nightmarket-reputation/src/main.rs
let vk_hash = [/* copy from build/reputation_threshold/reputation_threshold_vk_hash.txt */];
```

---

## Frontend Integration

The proving keys (`*_final.zkey`) and WASM witness generators (`*.wasm`) need to be accessible to the frontend:

1. Copy to `nightmarket-ui/public/circuits/`:
```bash
cp build/location_proof/location_proof_final.zkey ../nightmarket-ui/public/circuits/
cp build/location_proof/location_proof_js/location_proof.wasm ../nightmarket-ui/public/circuits/
# Repeat for other circuits
```

2. Use snarkjs in the frontend to generate proofs:
```typescript
import { groth16 } from 'snarkjs';

const { proof, publicSignals } = await groth16.fullProve(
    inputs,
    '/circuits/location_proof.wasm',
    '/circuits/location_proof_final.zkey'
);
```

---

## File Structure

```
circuits/
├── location_proof.circom           # Location proof circuit
├── mixer_withdrawal.circom         # Mixer withdrawal circuit
├── reputation_threshold.circom     # Reputation threshold circuit
├── compile_circuits.sh             # Compilation script
├── setup_circuits.sh               # Key generation script
├── test_circuits.js                # Test suite
├── package.json                    # Dependencies
└── build/                          # Generated artifacts
    ├── location_proof/
    │   ├── location_proof.r1cs
    │   ├── location_proof_final.zkey
    │   ├── location_proof_vkey.json
    │   ├── location_proof_vk_hash.txt
    │   └── location_proof_js/
    │       └── location_proof.wasm
    ├── mixer_withdrawal/
    └── reputation_threshold/
```

---

## Security Considerations

1. **Powers of Tau:** Using Hermez ceremony (trusted setup). For production, consider participating in a custom ceremony or using newer PLONK/STARKs.

2. **Verification Key Updates:** VK hashes are stored on-chain. Changing circuits requires contract upgrade.

3. **Circuit Audits:** These circuits should be professionally audited before mainnet deployment.

4. **Input Validation:** Always validate inputs before passing to circuits (frontend and contract side).

---

## Development

### Adding a New Circuit

1. Create `new_circuit.circom`
2. Add compilation step to `compile_circuits.sh`
3. Add setup step to `setup_circuits.sh`
4. Add test case to `test_circuits.js`
5. Update this README

### Debugging

- Use `circom --inspect` to see circuit structure
- Check `.sym` files for signal names and values
- Use `snarkjs wtns calculate` to compute witness for debugging
- Check constraint count with `snarkjs r1cs info <circuit>.r1cs`

---

## License

MIT
