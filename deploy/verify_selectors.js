// Verify all function selectors match their keccak256 hashes
const { keccak256, toUtf8Bytes } = require('ethers');

// Helper to compute selector
function computeSelector(signature) {
    const hash = keccak256(toUtf8Bytes(signature));
    return hash.slice(0, 10); // 0x + 8 hex chars = 4 bytes
}

// Helper to format expected bytes
function formatBytes(hexString) {
    // Remove 0x prefix and convert to byte array format
    const hex = hexString.slice(2);
    const bytes = [];
    for (let i = 0; i < hex.length; i += 2) {
        bytes.push('0x' + hex[i] + hex[i+1]);
    }
    return '[' + bytes.join(', ') + ']';
}

console.log('Verifying Nightmarket Contract Function Selectors\n');
console.log('='.repeat(80));

let allCorrect = true;

// NightmarketZones
console.log('\n📍 NightmarketZones Contract\n');
const zonesSelectors = {
    'initialize()': [0x8a, 0xd2, 0xb9, 0xfb],
    'addZone(uint32,int32,int32,int32,int32)': [0x7c, 0x9a, 0x24, 0x1e],
    'updateFingerprint(uint32,bytes32)': [0x42, 0x15, 0x88, 0xd7],
    'setPaused(bool)': [0x16, 0xc3, 0x8b, 0x3c],
    'verifyLocationProof(uint32,bytes,bytes32[])': [0x9e, 0x3c, 0x72, 0x4f],
    'isNightTime()': [0xa1, 0x2d, 0x43, 0x9e],
    'getZone(uint32)': [0x1d, 0x8a, 0x5e, 0x3c],
    'getZoneCount()': [0x45, 0x1a, 0x7c, 0x8d],
    'getFingerprint(uint32)': [0x7e, 0x2b, 0x9c, 0x4a],
    'hasValidProof(address)': [0x93, 0x6a, 0x5e, 0x1b],
};

for (const [sig, expected] of Object.entries(zonesSelectors)) {
    const computed = computeSelector(sig);
    const expectedHex = '0x' + expected.map(b => b.toString(16).padStart(2, '0')).join('');
    const match = computed === expectedHex;

    console.log(`  ${match ? '✓' : '✗'} ${sig}`);
    console.log(`     Expected: ${expectedHex}`);
    console.log(`     Computed: ${computed}`);

    if (!match) {
        console.log(`     MISMATCH! Should be: ${formatBytes(computed)}`);
        allCorrect = false;
    }
    console.log();
}

// NightmarketListings
console.log('='.repeat(80));
console.log('\n📝 NightmarketListings Contract\n');
const listingsSelectors = {
    'initialize()': [0x8a, 0xd2, 0xb9, 0xfb],
    'setZonesContract(address)': [0x4c, 0x2a, 0x9e, 0x31],
    'setPaused(bool)': [0x16, 0xc3, 0x8b, 0x3c],
    'createListing(uint32,bytes,uint256,bytes32)': [0x2d, 0x4e, 0x7a, 0x9c],
    'cancelListing(uint256)': [0x7a, 0xc2, 0xff, 0x3e],
    'expireListings(uint256[])': [0x91, 0x3b, 0x5c, 0x7d],
    'getListing(uint256)': [0x1f, 0x84, 0x2a, 0x5c],
    'getListingsByZone(uint32,uint256,uint256)': [0x3c, 0x91, 0x6e, 0x8a],
    'getListingsBatch(uint256[])': [0x5e, 0x2c, 0x9f, 0x1b],
    'getActiveCount()': [0x72, 0xa1, 0x4d, 0x9e],
    'getListingCount()': [0x8d, 0x3f, 0x7c, 0x2a],
};

for (const [sig, expected] of Object.entries(listingsSelectors)) {
    const computed = computeSelector(sig);
    const expectedHex = '0x' + expected.map(b => b.toString(16).padStart(2, '0')).join('');
    const match = computed === expectedHex;

    console.log(`  ${match ? '✓' : '✗'} ${sig}`);

    if (!match) {
        console.log(`     Expected: ${expectedHex}`);
        console.log(`     Computed: ${computed}`);
        console.log(`     MISMATCH! Should be: ${formatBytes(computed)}`);
        allCorrect = false;
    }
}
console.log();

// NightmarketMixer
console.log('='.repeat(80));
console.log('\n🔀 NightmarketMixer Contract\n');
const mixerSelectors = {
    'initialize()': [0x8a, 0xd2, 0xb9, 0xfb],
    'setPaused(bool)': [0x16, 0xc3, 0x8b, 0x3c],
    'withdrawFees()': [0x4a, 0x7c, 0x2e, 0x91],
    'deposit(uint32,bytes32)': [0xd0, 0xe3, 0x0d, 0xb0],
    'withdraw(uint32,bytes,bytes32,address)': [0x3c, 0xcf, 0xd6, 0x0b],
    'getPoolBalance(uint32,uint256)': [0x5c, 0x9a, 0x1e, 0x72],
    'isNullifierUsed(bytes32)': [0x7e, 0x2d, 0x4a, 0x93],
    'getMinDeposit()': [0x9f, 0x3b, 0x7c, 0x1a],
};

for (const [sig, expected] of Object.entries(mixerSelectors)) {
    const computed = computeSelector(sig);
    const expectedHex = '0x' + expected.map(b => b.toString(16).padStart(2, '0')).join('');
    const match = computed === expectedHex;

    console.log(`  ${match ? '✓' : '✗'} ${sig}`);

    if (!match) {
        console.log(`     Expected: ${expectedHex}`);
        console.log(`     Computed: ${computed}`);
        console.log(`     MISMATCH! Should be: ${formatBytes(computed)}`);
        allCorrect = false;
    }
}
console.log();

// NightmarketEscrow
console.log('='.repeat(80));
console.log('\n🤝 NightmarketEscrow Contract\n');
const escrowSelectors = {
    'initialize()': [0x8a, 0xd2, 0xb9, 0xfb],
    'setPaused(bool)': [0x16, 0xc3, 0x8b, 0x3c],
    'createTrade(uint256,address,uint256)': [0x3a, 0x8e, 0x7c, 0x21],
    'lockFunds(uint256)': [0x5d, 0x2a, 0x9f, 0x83],
    'revealCoordinates(uint256,uint8,bytes)': [0x7e, 0x1c, 0x4a, 0x92],
    'submitHeartbeat(uint256)': [0x9c, 0x3f, 0x7e, 0x1a],
    'completeTrade(uint256)': [0xa2, 0x5b, 0x8d, 0x4f],
    'disputeTrade(uint256)': [0xb4, 0x7d, 0x9a, 0x2c],
    'resolveDispute(uint256,bool)': [0xc8, 0x4e, 0x1f, 0x93],
    'getTrade(uint256)': [0xd1, 0x9c, 0x3a, 0x7e],
    'getCoordinates(uint256,uint8)': [0xe3, 0x2f, 0x8b, 0x5a],
    'getTradeState(uint256)': [0xf2, 0x7a, 0x4d, 0x1c],
};

for (const [sig, expected] of Object.entries(escrowSelectors)) {
    const computed = computeSelector(sig);
    const expectedHex = '0x' + expected.map(b => b.toString(16).padStart(2, '0')).join('');
    const match = computed === expectedHex;

    console.log(`  ${match ? '✓' : '✗'} ${sig}`);

    if (!match) {
        console.log(`     Expected: ${expectedHex}`);
        console.log(`     Computed: ${computed}`);
        console.log(`     MISMATCH! Should be: ${formatBytes(computed)}`);
        allCorrect = false;
    }
}
console.log();

// NightmarketReputation
console.log('='.repeat(80));
console.log('\n⭐ NightmarketReputation Contract\n');
const reputationSelectors = {
    'initialize()': [0x8a, 0xd2, 0xb9, 0xfb],
    'setEscrowContract(address)': [0x3c, 0x9e, 0x2a, 0x74],
    'setPaused(bool)': [0x16, 0xc3, 0x8b, 0x3c],
    'updateScore(uint32,bytes32,int256)': [0x4d, 0x7a, 0x91, 0x3e],
    'proveScoreThreshold(uint32,bytes32,bytes,uint256)': [0x5e, 0x8c, 0x2f, 0xa1],
    'getScore(uint32,bytes32)': [0x6f, 0x9a, 0x4d, 0x82],
    'getDecayedScore(uint32,bytes32)': [0x7a, 0x2c, 0x8e, 0x93],
};

for (const [sig, expected] of Object.entries(reputationSelectors)) {
    const computed = computeSelector(sig);
    const expectedHex = '0x' + expected.map(b => b.toString(16).padStart(2, '0')).join('');
    const match = computed === expectedHex;

    console.log(`  ${match ? '✓' : '✗'} ${sig}`);

    if (!match) {
        console.log(`     Expected: ${expectedHex}`);
        console.log(`     Computed: ${computed}`);
        console.log(`     MISMATCH! Should be: ${formatBytes(computed)}`);
        allCorrect = false;
    }
}
console.log();

console.log('='.repeat(80));
if (allCorrect) {
    console.log('\n✅ All function selectors are CORRECT!\n');
    process.exit(0);
} else {
    console.log('\n❌ Some selectors are INCORRECT and need to be fixed.\n');
    process.exit(1);
}
