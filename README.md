# Nightmarket

An anonymous decentralized marketplace operating exclusively during night hours (2:00-5:00 AM local time). Built on PolkaVM with zero-knowledge proofs, privacy-preserving transactions, and dead drop coordination.

## Overview

Nightmarket enables anonymous peer-to-peer commerce through:

- **Time-restricted operations**: Only accessible during 2:00-5:00 AM
- **Location-based zones**: Geographic areas with ZK location proofs
- **Privacy-preserving transactions**: Mixer pools anonymize fund flows
- **Dead drop protocol**: Staged coordinate revelation for physical exchanges
- **Anonymous reputation**: ZK proofs of reputation without revealing identity
- **Ephemeral identities**: New unlinkable identities each night per zone

## Architecture

The system consists of 5 smart contracts:

1. **NightmarketZones**: Zone management, signal fingerprints, time validation, location proofs
2. **NightmarketListings**: Encrypted listings with sparse merkle tree and auto-expiry
3. **NightmarketEscrow**: Multi-party escrow, staged reveals, dispute resolution
4. **NightmarketMixer**: Deposit pooling, anonymous withdrawals, nullifier tracking
5. **NightmarketReputation**: Score management, ZK reputation proofs, weekly decay

## Building

### Prerequisites

- Rust nightly (2024-11-19)
- `polkatool` (for linking PolkaVM binaries)

### Build all contracts

```bash
./build.sh
```

### Build a single contract

```bash
./build_single.sh zones     # or listings, escrow, mixer, reputation
```

Output binaries are placed in `build/`.

## Project Structure

```
nightmarket/
├── contracts/
│   ├── nightmarket-zones/
│   ├── nightmarket-listings/
│   ├── nightmarket-escrow/
│   ├── nightmarket-mixer/
│   └── nightmarket-reputation/
├── shared/
│   └── src/
│       ├── crypto.rs        # ZK proofs, merkle trees
│       ├── bounds.rs        # Safe arithmetic
│       └── storage.rs       # Storage helpers
├── build/                   # Compiled contracts
└── deployments/            # Deployment artifacts

## Development Status

🚧 **Phase 1 Complete**: Project setup and infrastructure
- [x] Workspace structure
- [x] Build system configuration
- [x] Shared crypto, bounds, storage modules

⏳ **Phase 2 In Progress**: Core contracts implementation

## Design

See [DESIGN.md](./DESIGN.md) for the complete protocol specification.

## License

MIT
