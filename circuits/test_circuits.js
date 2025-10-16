#!/usr/bin/env node

import { readFileSync } from 'fs';
import { groth16 } from 'snarkjs';

/**
 * Test all Nightmarket ZK circuits with sample inputs
 */

async function testLocationProof() {
    console.log("Testing Location Proof Circuit...");

    const input = {
        // Private inputs
        location_lat: 40749000,      // 40.749° N (Times Square, NYC * 1e6)
        location_lon: -73987000,     // -73.987° W
        signal_hashes: [
            123456n, 234567n, 345678n, 456789n,
            567890n, 678901n, 789012n, 890123n
        ],
        secret: 999888777666n,

        // Zone boundaries (private)
        zone_lat_min: 40740000,      // 40.740° N
        zone_lat_max: 40760000,      // 40.760° N
        zone_lon_min: -74000000,     // -74.000° W
        zone_lon_max: -73970000,     // -73.970° W

        // Public inputs
        zone_id: 1n,
        timestamp: 1697500000n
    };

    try {
        const { proof, publicSignals } = await groth16.fullProve(
            input,
            "build/location_proof/location_proof_js/location_proof.wasm",
            "build/location_proof/location_proof_final.zkey"
        );

        const vkey = JSON.parse(readFileSync("build/location_proof/location_proof_vkey.json"));
        const verified = await groth16.verify(vkey, publicSignals, proof);

        console.log("  ✓ Proof generated");
        console.log("  ✓ Public signals:", publicSignals);
        console.log("  ✓ Verified:", verified);

        if (!verified) throw new Error("Verification failed!");

        console.log("  ✓ Location proof circuit works!\n");
        return true;
    } catch (error) {
        console.error("  ✗ Location proof test failed:", error.message);
        return false;
    }
}

async function testMixerWithdrawal() {
    console.log("Testing Mixer Withdrawal Circuit...");

    const input = {
        // Private inputs
        secret: 123456789012345n,
        amount: 10000000000000000n,  // 0.01 ETH in wei

        // Public inputs
        zone_id: 1n
    };

    try {
        const { proof, publicSignals } = await groth16.fullProve(
            input,
            "build/mixer_withdrawal/mixer_withdrawal_js/mixer_withdrawal.wasm",
            "build/mixer_withdrawal/mixer_withdrawal_final.zkey"
        );

        const vkey = JSON.parse(readFileSync("build/mixer_withdrawal/mixer_withdrawal_vkey.json"));
        const verified = await groth16.verify(vkey, publicSignals, proof);

        console.log("  ✓ Proof generated");
        console.log("  ✓ Public signals:", publicSignals);
        console.log("  ✓ Verified:", verified);

        if (!verified) throw new Error("Verification failed!");

        console.log("  ✓ Mixer withdrawal circuit works!\n");
        return true;
    } catch (error) {
        console.error("  ✗ Mixer withdrawal test failed:", error.message);
        return false;
    }
}

async function testReputationThreshold() {
    console.log("Testing Reputation Threshold Circuit...");

    const input = {
        // Private inputs
        score: 150n,
        secret: 987654321098765n,

        // Public inputs
        zone_id: 1n,
        ephemeral_id: 12345678901234567890n,  // Would be hash(secret, zone_id) in practice
        threshold: 100n
    };

    // First compute correct ephemeral_id
    // In practice: ephemeral_id = poseidon(secret, zone_id)
    // For testing, we'll let the circuit compute it

    try {
        const { proof, publicSignals } = await groth16.fullProve(
            input,
            "build/reputation_threshold/reputation_threshold_js/reputation_threshold.wasm",
            "build/reputation_threshold/reputation_threshold_final.zkey"
        );

        const vkey = JSON.parse(readFileSync("build/reputation_threshold/reputation_threshold_vkey.json"));
        const verified = await groth16.verify(vkey, publicSignals, proof);

        console.log("  ✓ Proof generated");
        console.log("  ✓ Public signals:", publicSignals);
        console.log("  ✓ Verified:", verified);

        if (!verified) throw new Error("Verification failed!");

        console.log("  ✓ Reputation threshold circuit works!\n");
        return true;
    } catch (error) {
        console.error("  ✗ Reputation threshold test failed:", error.message);
        return false;
    }
}

async function main() {
    console.log("=".repeat(60));
    console.log("Nightmarket ZK Circuit Test Suite");
    console.log("=".repeat(60));
    console.log();

    const results = await Promise.all([
        testLocationProof(),
        testMixerWithdrawal(),
        testReputationThreshold()
    ]);

    console.log("=".repeat(60));
    const passed = results.filter(r => r).length;
    const total = results.length;

    if (passed === total) {
        console.log(`✓ All ${total} circuits passed!`);
        console.log("=".repeat(60));
        process.exit(0);
    } else {
        console.log(`✗ ${total - passed}/${total} circuits failed`);
        console.log("=".repeat(60));
        process.exit(1);
    }
}

main().catch(console.error);
