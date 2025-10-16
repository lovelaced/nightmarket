const ethers = require('ethers');
const fs = require('fs');

function getSelector(sig) {
  return ethers.id(sig).slice(0, 10);
}

function toBytes(hex) {
  const clean = hex.startsWith('0x') ? hex.slice(2) : hex;
  return '[0x' + clean.match(/.{2}/g).join(', 0x') + ']';
}

console.log('='.repeat(80));
console.log('NIGHTMARKET SELECTOR VERIFICATION');
console.log('='.repeat(80));
console.log('');

const contracts = {
  'ZONES': [
    { name: 'initialize()', sig: 'initialize()' },
    { name: 'addZone(uint32,int32,int32,int32,int32)', sig: 'addZone(uint32,int32,int32,int32,int32)' },
    { name: 'updateFingerprint(uint32,bytes32)', sig: 'updateFingerprint(uint32,bytes32)' },
    { name: 'setPaused(bool)', sig: 'setPaused(bool)' },
    { name: 'verifyLocationProof(uint32,bytes,bytes32)', sig: 'verifyLocationProof(uint32,bytes,bytes32)' },
    { name: 'isNightTime()', sig: 'isNightTime()' },
    { name: 'getZone(uint32)', sig: 'getZone(uint32)' },
    { name: 'getZoneCount()', sig: 'getZoneCount()' },
    { name: 'getFingerprint(uint32)', sig: 'getFingerprint(uint32)' },
    { name: 'hasValidProof(address)', sig: 'hasValidProof(address)' },
  ],

  'LISTINGS': [
    { name: 'initialize()', sig: 'initialize()' },
    { name: 'setZonesContract(address)', sig: 'setZonesContract(address)' },
    { name: 'setPaused(bool)', sig: 'setPaused(bool)' },
    { name: 'createListing(uint32,bytes,uint256,bytes32)', sig: 'createListing(uint32,bytes,uint256,bytes32)' },
    { name: 'cancelListing(uint256)', sig: 'cancelListing(uint256)' },
    { name: 'expireListings(uint256[])', sig: 'expireListings(uint256[])' },
    { name: 'getListing(uint256)', sig: 'getListing(uint256)' },
    { name: 'getListingsByZone(uint32,uint256,uint256)', sig: 'getListingsByZone(uint32,uint256,uint256)' },
    { name: 'getListingsBatch(uint256[])', sig: 'getListingsBatch(uint256[])' },
    { name: 'getActiveCount()', sig: 'getActiveCount()' },
    { name: 'getListingCount()', sig: 'getListingCount()' },
  ],

  'ESCROW': [
    { name: 'initialize()', sig: 'initialize()' },
    { name: 'setPaused(bool)', sig: 'setPaused(bool)' },
    { name: 'withdrawFees()', sig: 'withdrawFees()' },
    { name: 'createTrade(uint256,address,uint256)', sig: 'createTrade(uint256,address,uint256)' },
    { name: 'lockFunds(uint256)', sig: 'lockFunds(uint256)' },
    { name: 'cancelTrade(uint256)', sig: 'cancelTrade(uint256)' },
    { name: 'revealCoordinates(uint256,uint8,bytes)', sig: 'revealCoordinates(uint256,uint8,bytes)' },
    { name: 'submitHeartbeat(uint256)', sig: 'submitHeartbeat(uint256)' },
    { name: 'completeTrade(uint256)', sig: 'completeTrade(uint256)' },
    { name: 'disputeTrade(uint256)', sig: 'disputeTrade(uint256)' },
    { name: 'resolveDispute(uint256,bool)', sig: 'resolveDispute(uint256,bool)' },
    { name: 'getTrade(uint256)', sig: 'getTrade(uint256)' },
    { name: 'getCoordinates(uint256,uint8)', sig: 'getCoordinates(uint256,uint8)' },
    { name: 'getTradeState(uint256)', sig: 'getTradeState(uint256)' },
  ],

  'MIXER': [
    { name: 'initialize()', sig: 'initialize()' },
    { name: 'setPaused(bool)', sig: 'setPaused(bool)' },
    { name: 'withdrawFees()', sig: 'withdrawFees()' },
    { name: 'deposit(uint32,bytes32)', sig: 'deposit(uint32,bytes32)' },
    { name: 'withdraw(uint32,bytes,bytes32,address)', sig: 'withdraw(uint32,bytes,bytes32,address)' },
    { name: 'getPoolBalance(uint32,uint256)', sig: 'getPoolBalance(uint32,uint256)' },
    { name: 'isNullifierUsed(bytes32)', sig: 'isNullifierUsed(bytes32)' },
    { name: 'getMinDeposit()', sig: 'getMinDeposit()' },
  ],

  'REPUTATION': [
    { name: 'initialize()', sig: 'initialize()' },
    { name: 'setEscrowContract(address)', sig: 'setEscrowContract(address)' },
    { name: 'setPaused(bool)', sig: 'setPaused(bool)' },
    { name: 'updateScore(uint32,bytes32,int256)', sig: 'updateScore(uint32,bytes32,int256)' },
    { name: 'proveScoreThreshold(uint32,bytes32,bytes,uint256)', sig: 'proveScoreThreshold(uint32,bytes32,bytes,uint256)' },
    { name: 'getScore(uint32,bytes32)', sig: 'getScore(uint32,bytes32)' },
    { name: 'getDecayedScore(uint32,bytes32)', sig: 'getDecayedScore(uint32,bytes32)' },
  ],
};

let totalChecked = 0;
let mismatches = 0;

Object.entries(contracts).forEach(([contractName, functions]) => {
  console.log(`${contractName} CONTRACT:`);
  console.log('-'.repeat(80));

  functions.forEach(func => {
    const calculated = getSelector(func.sig);
    totalChecked++;

    console.log(`${func.name}`);
    console.log(`  Signature: ${func.sig}`);
    console.log(`  Selector:  ${calculated}`);
    console.log(`  Rust:      ${toBytes(calculated)}`);
    console.log('');
  });

  console.log('');
});

console.log('='.repeat(80));
console.log(`Total functions checked: ${totalChecked}`);
console.log(`Mismatches found: ${mismatches}`);
console.log('='.repeat(80));

console.log('');
console.log('To update contract selectors, copy the Rust format above.');
console.log('Example: const SELECTOR_INITIALIZE: [u8; 4] = [0x81, 0x29, 0xfc, 0x1c];');
