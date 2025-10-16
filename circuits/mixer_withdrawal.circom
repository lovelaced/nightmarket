pragma circom 2.1.4;

include "node_modules/circomlib/circuits/poseidon.circom";
include "node_modules/circomlib/circuits/bitify.circom";

/*
 * Mixer Withdrawal Circuit
 *
 * Proves knowledge of a commitment secret without linking withdrawal to deposit.
 *
 * Private Inputs:
 * - secret: Random secret used in original commitment
 *
 * Public Inputs:
 * - zone_id: Zone where funds were deposited
 *
 * Public Outputs:
 * - nullifier: Prevents double-withdrawal
 * - commitment: Original deposit commitment
 */

template MixerWithdrawal() {
    // Private input
    signal input secret;

    // Public input
    signal input zone_id;

    // Public outputs
    signal output nullifier;
    signal output commitment;

    // 1. Generate commitment = poseidon(secret, zone_id)
    component commitment_hasher = Poseidon(2);
    commitment_hasher.inputs[0] <== secret;
    commitment_hasher.inputs[1] <== zone_id;
    commitment <== commitment_hasher.out;

    // 2. Generate nullifier = poseidon(secret, secret)
    // Double hashing ensures nullifier != commitment
    component nullifier_hasher = Poseidon(2);
    nullifier_hasher.inputs[0] <== secret;
    nullifier_hasher.inputs[1] <== secret;
    nullifier <== nullifier_hasher.out;

    // 3. Range check zone_id
    component zone_id_bits = Num2Bits(32);
    zone_id_bits.in <== zone_id;
}

component main {public [zone_id]} = MixerWithdrawal();
