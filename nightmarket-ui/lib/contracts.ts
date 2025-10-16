// Contract addresses (will be populated after deployment)
export const CONTRACTS = {
  ZONES: process.env.NEXT_PUBLIC_ZONES_CONTRACT || '',
  LISTINGS: process.env.NEXT_PUBLIC_LISTINGS_CONTRACT || '',
  MIXER: process.env.NEXT_PUBLIC_MIXER_CONTRACT || '',
  ESCROW: process.env.NEXT_PUBLIC_ESCROW_CONTRACT || '',
  REPUTATION: process.env.NEXT_PUBLIC_REPUTATION_CONTRACT || '',
};

// Minimal ABIs - just what we need for the UI
export const ZONES_ABI = [
  'function verifyLocationProof(uint32 zone_id, bytes proof, bytes32 nullifier)',
  'function isNightTime() view returns (bool)',
  'function hasValidProof(address user) view returns (bool)',
  'function getZoneCount() view returns (uint256)',
  'function getZone(uint32 zone_id) view returns (int32,int32,int32,int32)',
];

export const LISTINGS_ABI = [
  'function createListing(uint32 zone_id, bytes encrypted_data, uint256 price, bytes32 drop_zone_hash) returns (uint256)',
  'function cancelListing(uint256 listing_id)',
  // getListing returns raw 328 bytes (not ABI-encoded)
  // Must be called with staticCall and parsed manually
  'function getListing(uint256 listing_id) view',
  'function getListingsByZone(uint32 zone_id, uint256 offset, uint256 limit) view returns (uint256[])',
  'function getActiveCount() view returns (uint256)',
];

export const MIXER_ABI = [
  'function deposit(uint32 zone_id, bytes32 commitment) payable',
  'function withdraw(uint32 zone_id, bytes proof, bytes32 nullifier, address recipient)',
  'function getPoolBalance(uint32 zone_id, uint256 night_timestamp) view returns (uint256)',
  'function isNullifierUsed(bytes32 nullifier) view returns (bool)',
  'function getMinDeposit() view returns (uint256)',
];

export const ESCROW_ABI = [
  'function createTrade(uint256 listing_id, address seller, uint256 price) returns (uint256)',
  'function lockFunds(uint256 trade_id) payable',
  'function revealCoordinates(uint256 trade_id, uint8 stage, bytes coordinates)',
  'function submitHeartbeat(uint256 trade_id)',
  'function completeTrade(uint256 trade_id)',
  'function getTrade(uint256 trade_id) view returns (bytes)',
  'function getCoordinates(uint256 trade_id, uint8 stage) view returns (bytes)',
];

export const REPUTATION_ABI = [
  'function getScore(uint32 zone_id, bytes32 ephemeral_id) view returns (uint256)',
  'function getDecayedScore(uint32 zone_id, bytes32 ephemeral_id) view returns (uint256)',
  'function proveScoreThreshold(uint32 zone_id, bytes32 ephemeral_id, bytes proof, uint256 threshold)',
];
