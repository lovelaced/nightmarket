#!/usr/bin/env node

/**
 * Extract verification key hashes and format them for Rust contracts
 */

import { readFileSync, existsSync } from 'fs';
import { createHash } from 'crypto';

const circuits = ['location_proof', 'mixer_withdrawal', 'reputation_threshold'];
const contracts = [
    'contracts/nightmarket-zones/src/main.rs',
    'contracts/nightmarket-mixer/src/main.rs',
    'contracts/nightmarket-reputation/src/main.rs'
];

console.log("Extracting Verification Key Hashes for Rust Contracts");
console.log("=".repeat(60));
console.log();

circuits.forEach((circuit, index) => {
    const vkeyPath = `build/${circuit}/${circuit}_vkey.json`;
    const contractPath = `../${contracts[index]}`;

    if (!existsSync(vkeyPath)) {
        console.log(`⚠️  ${circuit}: Verification key not found`);
        console.log(`   Run: npm run setup`);
        console.log();
        return;
    }

    // Read verification key
    const vkey = JSON.parse(readFileSync(vkeyPath, 'utf8'));

    // Calculate hash
    const vkeyStr = JSON.stringify(vkey);
    const hashBytes = createHash('sha256').update(vkeyStr).digest();

    // Format as Rust array
    const rustArray = Array.from(hashBytes)
        .map((b, i) => {
            const hex = '0x' + b.toString(16).padStart(2, '0');
            return (i % 8 === 0 ? '\n    ' : '') + hex;
        })
        .join(', ');

    console.log(`Circuit: ${circuit}`);
    console.log(`Contract: ${contracts[index]}`);
    console.log(`VK Hash (Rust format):`)
    console.log(`let vk_hash = [${rustArray}\n];`);
    console.log();
    console.log(`VK Hash (hex): 0x${hashBytes.toString('hex')}`);
    console.log();
    console.log("-".repeat(60));
    console.log();
});

console.log("Usage:");
console.log("1. Copy the 'let vk_hash = [...]' line");
console.log("2. Replace the placeholder in the corresponding contract:");
console.log("   let vk_hash = [0u8; 32];  // <-- Replace this line");
console.log("3. Rebuild contracts: ./build.sh");
console.log();
