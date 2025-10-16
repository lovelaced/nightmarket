pragma circom 2.1.4;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/comparators.circom";
include "node_modules/circomlib/circuits/bitify.circom";

/*
 * Reputation Threshold Proof Circuit
 *
 * Proves reputation score >= threshold without revealing exact score.
 *
 * Private Inputs:
 * - secret: User's reputation secret
 * - score: Actual reputation score
 *
 * Public Inputs:
 * - zone_id: Zone identifier
 * - ephemeral_id: Nightly ephemeral identity hash
 * - threshold: Minimum required score
 */

template ReputationThreshold() {
    // Private inputs
    signal input secret;
    signal input score;

    // Public inputs
    signal input zone_id;
    signal input ephemeral_id;
    signal input threshold;

    // 1. Verify ephemeral_id = poseidon(secret, zone_id)
    component id_hasher = Poseidon(2);
    id_hasher.inputs[0] <== secret;
    id_hasher.inputs[1] <== zone_id;
    id_hasher.out === ephemeral_id;

    // 2. Verify score >= threshold
    component score_check = GreaterEqThan(64);
    score_check.in[0] <== score;
    score_check.in[1] <== threshold;
    score_check.out === 1;

    // 3. Range checks
    component score_bits = Num2Bits(64);
    score_bits.in <== score;

    component threshold_bits = Num2Bits(64);
    threshold_bits.in <== threshold;

    component zone_bits = Num2Bits(32);
    zone_bits.in <== zone_id;
}

component main {public [zone_id, ephemeral_id, threshold]} = ReputationThreshold();
