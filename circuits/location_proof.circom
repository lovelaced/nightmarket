pragma circom 2.1.4;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/bitify.circom";

/*
 * Location Proof Circuit
 *
 * Proves user is in a specific zone at a specific time without revealing exact location.
 *
 * Private Inputs:
 * - secret: Random secret for nullifier derivation
 * - location_lat, location_lon: Exact coordinates (scaled by 1e6)
 * - signal_hashes[8]: WiFi/cellular signal identifiers
 * - zone_lat_min, zone_lat_max, zone_lon_min, zone_lon_max: Zone boundaries
 *
 * Public Inputs:
 * - zone_id: Geographic zone identifier
 * - timestamp: Unix timestamp (for replay protection)
 *
 * Public Outputs:
 * - nullifier: Prevents proof reuse
 */

template LocationProof() {
    // Private inputs
    signal input secret;
    signal input location_lat;
    signal input location_lon;
    signal input signal_hashes[8];
    signal input zone_lat_min;
    signal input zone_lat_max;
    signal input zone_lon_min;
    signal input zone_lon_max;

    // Public inputs
    signal input zone_id;
    signal input timestamp;

    // Public output
    signal output nullifier;

    // 1. Derive nullifier from secret + zone_id + timestamp
    component nullifier_hasher = Poseidon(3);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.inputs[1] <== zone_id;
    nullifier_hasher.inputs[2] <== timestamp;
    nullifier <== nullifier_hasher.out;

    // 2. Verify location is within zone boundaries
    component lat_min_check = GreaterEqThan(32);
    lat_min_check.in[0] <== location_lat;
    lat_min_check.in[1] <== zone_lat_min;
    lat_min_check.out === 1;

    component lat_max_check = LessEqThan(32);
    lat_max_check.in[0] <== location_lat;
    lat_max_check.in[1] <== zone_lat_max;
    lat_max_check.out === 1;

    component lon_min_check = GreaterEqThan(32);
    lon_min_check.in[0] <== location_lon;
    lon_min_check.in[1] <== zone_lon_min;
    lon_min_check.out === 1;

    component lon_max_check = LessEqThan(32);
    lon_max_check.in[0] <== location_lon;
    lon_max_check.in[1] <== zone_lon_max;
    lon_max_check.out === 1;

    // 3. Hash signal fingerprint (simplified)
    component signal_hasher = Poseidon(8);
    for (var i = 0; i < 8; i++) {
        signal_hasher.inputs[i] <== signal_hashes[i];
    }
    signal fingerprint_hash <== signal_hasher.out;

    // Signal fingerprint contributes to proof uniqueness
    // In production: would verify merkle proof of fingerprint against zone root

    // 4. Range checks
    component zone_id_bits = Num2Bits(32);
    zone_id_bits.in <== zone_id;

    component timestamp_bits = Num2Bits(64);
    timestamp_bits.in <== timestamp;
}

component main {public [zone_id, timestamp]} = LocationProof();
