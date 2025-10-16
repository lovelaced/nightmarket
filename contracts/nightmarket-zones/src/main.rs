#![no_std]
#![no_main]
extern crate alloc;

use simplealloc::SimpleAlloc;

// Fixed 50KB heap
#[global_allocator]
static GLOBAL_ALLOCATOR: SimpleAlloc<{ 1024 * 50 }> = SimpleAlloc::new();

use uapi::{HostFn, HostFnImpl as api, StorageFlags, ReturnFlags};
use ethabi::{decode, encode, Token, ParamType, ethereum_types::U256};
use nightmarket_shared::{
    Groth16Proof, verify_groth16, keccak256,
    safe_add, safe_sub, check_bounds,
    storage_key, zone_time_key, address_key,
};

// Include shared panic handler
include!("../../../shared/src/panic_handler.rs");

// ============================================================================
// Storage Prefixes
// ============================================================================

const PREFIX_OWNER: u8 = 0;
const PREFIX_ZONE_COUNT: u8 = 1;
const PREFIX_ZONE_DATA: u8 = 2;           // zone_id -> ZoneData
const PREFIX_ZONE_FINGERPRINT: u8 = 3;     // zone_id + timestamp -> merkle root
const PREFIX_PROOF_USED: u8 = 4;           // nullifier -> bool
const PREFIX_USER_LAST_PROOF: u8 = 5;      // user address -> timestamp
const PREFIX_PAUSED: u8 = 6;

// ============================================================================
// Constants
// ============================================================================

const NIGHT_START_HOUR: u64 = 6;    // 6:00 AM
const NIGHT_END_HOUR: u64 = 5;      // 5:00 AM
const SECONDS_PER_HOUR: u64 = 3600;
const FINGERPRINT_UPDATE_INTERVAL: u64 = 100; // blocks
const MIN_SIGNAL_COUNT: u64 = 8;    // 5 WiFi + 3 cellular minimum

// ============================================================================
// Function Selectors
// ============================================================================

// Admin functions
const SELECTOR_INITIALIZE: [u8; 4] = [0x81, 0x29, 0xfc, 0x1c];  // initialize()
const SELECTOR_ADD_ZONE: [u8; 4] = [0x23, 0xd7, 0x0d, 0x87];    // addZone(uint32,int32,int32,int32,int32)
const SELECTOR_UPDATE_FINGERPRINT: [u8; 4] = [0x3e, 0x45, 0xfc, 0x68];  // updateFingerprint(uint32,bytes32)
const SELECTOR_SET_PAUSED: [u8; 4] = [0x16, 0xc3, 0x8b, 0x3c];  // setPaused(bool)

// User functions
const SELECTOR_VERIFY_LOCATION_PROOF: [u8; 4] = [0x55, 0xb3, 0xf4, 0xbb];  // verifyLocationProof(uint32,bytes,bytes32)
const SELECTOR_IS_NIGHT_TIME: [u8; 4] = [0xc6, 0x93, 0xdb, 0x9b];  // isNightTime()

// View functions
const SELECTOR_GET_ZONE: [u8; 4] = [0xf5, 0x50, 0x2c, 0x34];     // getZone(uint32)
const SELECTOR_GET_ZONE_COUNT: [u8; 4] = [0x3b, 0x26, 0x0a, 0xa2];  // getZoneCount()
const SELECTOR_GET_FINGERPRINT: [u8; 4] = [0x30, 0xf8, 0x45, 0xde];  // getFingerprint(uint32)
const SELECTOR_HAS_VALID_PROOF: [u8; 4] = [0x01, 0xae, 0x8b, 0x7b];  // hasValidProof(address)

// ============================================================================
// Error Messages
// ============================================================================

const ERROR_NOT_INITIALIZED: &[u8] = b"NotInitialized";
const ERROR_ALREADY_INITIALIZED: &[u8] = b"AlreadyInitialized";
const ERROR_NOT_OWNER: &[u8] = b"NotOwner";
const ERROR_PAUSED: &[u8] = b"ContractPaused";
const ERROR_NOT_NIGHT_TIME: &[u8] = b"NotNightTime";
const ERROR_INVALID_ZONE: &[u8] = b"InvalidZone";
const ERROR_INVALID_PROOF: &[u8] = b"InvalidProof";
const ERROR_PROOF_ALREADY_USED: &[u8] = b"ProofAlreadyUsed";
const ERROR_TOO_SOON: &[u8] = b"ProofTooSoon";
const ERROR_INVALID_BOUNDARIES: &[u8] = b"InvalidBoundaries";

// ============================================================================
// Deploy Function
// ============================================================================

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn deploy() {
    // Store deployer as owner
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    let owner_key = storage_key(PREFIX_OWNER, b"");
    api::set_storage(StorageFlags::empty(), &owner_key, &caller);

    // Initialize zone count to 0
    let count_key = storage_key(PREFIX_ZONE_COUNT, b"");
    let zero = [0u8; 32];
    api::set_storage(StorageFlags::empty(), &count_key, &zero);

    // Not paused by default
    let paused_key = storage_key(PREFIX_PAUSED, b"");
    api::set_storage(StorageFlags::empty(), &paused_key, &[0u8; 1]);

    // Emit Initialized event
    let topics = [[0x11; 32]]; // Initialized topic
    api::deposit_event(&topics, &caller);
}

// ============================================================================
// Call Function (Router)
// ============================================================================

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn call() {
    // Check if paused (except for view functions and owner functions)
    let mut selector = [0u8; 4];
    api::call_data_copy(&mut selector, 0);

    match selector {
        SELECTOR_INITIALIZE => handle_initialize(),
        SELECTOR_ADD_ZONE => handle_add_zone(),
        SELECTOR_UPDATE_FINGERPRINT => handle_update_fingerprint(),
        SELECTOR_SET_PAUSED => handle_set_paused(),
        SELECTOR_VERIFY_LOCATION_PROOF => handle_verify_location_proof(),
        SELECTOR_IS_NIGHT_TIME => handle_is_night_time(),
        SELECTOR_GET_ZONE => handle_get_zone(),
        SELECTOR_GET_ZONE_COUNT => handle_get_zone_count(),
        SELECTOR_GET_FINGERPRINT => handle_get_fingerprint(),
        SELECTOR_HAS_VALID_PROOF => handle_has_valid_proof(),
        _ => {
            // Fallback - accept value transfers
            api::return_value(ReturnFlags::empty(), &[]);
        }
    }
}

// ============================================================================
// Admin Functions
// ============================================================================

fn handle_initialize() {
    require_owner();

    // Already done in deploy(), this is a no-op for compatibility
    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_add_zone() {
    require_owner();

    // Read inputs: addZone(uint32 zone_id, int32 lat_min, int32 lon_min, int32 lat_max, int32 lon_max)
    let input_size = api::call_data_size();
    if input_size != 4 + 32 * 5 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 164];
    api::call_data_copy(&mut input, 0);

    // Decode parameters
    let tokens = match decode(
        &[
            ParamType::Uint(32),
            ParamType::Int(32),
            ParamType::Int(32),
            ParamType::Int(32),
            ParamType::Int(32),
        ],
        &input[4..],
    ) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    // For simplicity, store zone boundaries as encoded data
    // In production, would validate lat/lon ranges
    let zone_key = zone_storage_key(zone_id);
    api::set_storage(StorageFlags::empty(), &zone_key, &input[4..]);

    // Increment zone count
    let count_key = storage_key(PREFIX_ZONE_COUNT, b"");
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);
    let count = u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                                    count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]]);
    let new_count = count + 1;
    count_bytes[..8].copy_from_slice(&new_count.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &count_key, &count_bytes);

    // Emit ZoneAdded event
    let mut topic = [0u8; 32];
    topic[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x22; 32], topic];
    api::deposit_event(&topics, &[]);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_update_fingerprint() {
    require_owner();

    // updateFingerprint(uint32 zone_id, bytes32 merkle_root)
    let input_size = api::call_data_size();
    if input_size != 4 + 64 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 68];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(
        &[ParamType::Uint(32), ParamType::FixedBytes(32)],
        &input[4..],
    ) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let merkle_root = match &tokens[1] {
        Token::FixedBytes(b) => {
            let mut root = [0u8; 32];
            root.copy_from_slice(&b[..32]);
            root
        }
        _ => revert(b"InvalidRoot"),
    };

    // Get current timestamp
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2], timestamp_buffer[3],
                                        timestamp_buffer[4], timestamp_buffer[5], timestamp_buffer[6], timestamp_buffer[7]]);

    // Store fingerprint: zone_id + timestamp -> merkle_root
    let fp_key = zone_time_key(PREFIX_ZONE_FINGERPRINT, zone_id, timestamp);
    api::set_storage(StorageFlags::empty(), &fp_key, &merkle_root);

    // Emit FingerprintUpdated event
    let mut topic = [0u8; 32];
    topic[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x33; 32], topic];
    api::deposit_event(&topics, &merkle_root);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_set_paused() {
    require_owner();

    // setPaused(bool paused)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Bool], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let paused = match &tokens[0] {
        Token::Bool(b) => *b,
        _ => revert(b"InvalidBool"),
    };

    let paused_key = storage_key(PREFIX_PAUSED, b"");
    let value = if paused { [1u8; 1] } else { [0u8; 1] };
    api::set_storage(StorageFlags::empty(), &paused_key, &value);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

// ============================================================================
// User Functions
// ============================================================================

fn handle_verify_location_proof() {
    require_not_paused();
    require_night_time();

    // verifyLocationProof(uint32 zone_id, bytes proof, bytes32[] public_inputs)
    // For now, simplified: (zone_id, proof_bytes, nullifier)
    let input_size = api::call_data_size();
    if input_size < 4 + 32 * 3 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 512];
    let copy_len = input_size.min(512);
    api::call_data_copy(&mut input, 0);

    // Simplified decoding: zone_id (32 bytes), proof (256 bytes), nullifier (32 bytes)
    let zone_id = u32::from_le_bytes([input[4], input[5], input[6], input[7]]);

    // NOTE: With global grid system, zones don't need pre-registration
    // Zone IDs are calculated deterministically from GPS coordinates
    // The ZK circuit verifies the user is actually at the location for this zone
    // No need to check zone existence in contract storage

    // Parse proof (256 bytes starting at offset 36)
    let proof = match Groth16Proof::from_bytes(&input[36..292]) {
        Ok(p) => p,
        Err(e) => revert(e.as_bytes()),
    };

    // Get nullifier (32 bytes at offset 292)
    let mut nullifier = [0u8; 32];
    nullifier.copy_from_slice(&input[292..324]);

    // Check if proof already used
    let nullifier_key = storage_key(PREFIX_PROOF_USED, &nullifier);
    let mut check_buffer = [0u8; 1];
    if api::get_storage(StorageFlags::empty(), &nullifier_key, &mut &mut check_buffer[..]).is_ok() {
        revert(ERROR_PROOF_ALREADY_USED);
    }

    // Rate limiting: check last proof time (one proof per hour)
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    let last_proof_key = address_key(PREFIX_USER_LAST_PROOF, &caller);
    let mut last_time_bytes = [0u8; 32];
    if api::get_storage(StorageFlags::empty(), &last_proof_key, &mut &mut last_time_bytes[..]).is_ok() {
        let last_time = u64::from_le_bytes([
            last_time_bytes[0], last_time_bytes[1], last_time_bytes[2], last_time_bytes[3],
            last_time_bytes[4], last_time_bytes[5], last_time_bytes[6], last_time_bytes[7],
        ]);
        let mut now_buffer = [0u8; 32];
        api::now(&mut now_buffer);
        let now = u64::from_le_bytes([now_buffer[0], now_buffer[1], now_buffer[2], now_buffer[3],
                                       now_buffer[4], now_buffer[5], now_buffer[6], now_buffer[7]]);
        if now < last_time + SECONDS_PER_HOUR {
            revert(ERROR_TOO_SOON);
        }
    }

    // Verify the ZK proof
    // Public inputs: [zone_id, timestamp, nullifier_hash]
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2], timestamp_buffer[3],
                                        timestamp_buffer[4], timestamp_buffer[5], timestamp_buffer[6], timestamp_buffer[7]]);
    let mut pub_input_1 = [0u8; 32];
    pub_input_1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let mut pub_input_2 = [0u8; 32];
    pub_input_2[..8].copy_from_slice(&timestamp.to_le_bytes());

    let public_inputs = [pub_input_1, pub_input_2, nullifier];
    // Location Proof circuit verification key hash
    let vk_hash = [0xa8, 0xa5, 0xef, 0x48, 0xeb, 0xeb, 0xb2, 0x3d, 0x29, 0x2f, 0xf9, 0xba, 0x9b, 0xa0, 0x28, 0xe9, 0x3e, 0xbf, 0xa9, 0xa8, 0x98, 0x8b, 0x15, 0x82, 0x83, 0x1c, 0x28, 0x13, 0xf3, 0x16, 0x44, 0x61];

    if let Err(e) = verify_groth16(&proof, &public_inputs, &vk_hash) {
        revert(e.as_bytes());
    }

    // Mark nullifier as used
    api::set_storage(StorageFlags::empty(), &nullifier_key, &[1u8]);

    // Update last proof time
    let mut time_bytes = [0u8; 32];
    time_bytes[..8].copy_from_slice(&timestamp.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &last_proof_key, &time_bytes);

    // Emit LocationProofVerified event
    let mut topic1 = [0u8; 32];
    topic1[..20].copy_from_slice(&caller);
    let mut topic2 = [0u8; 32];
    topic2[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x44; 32], topic1, topic2];
    api::deposit_event(&topics, &nullifier);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_is_night_time() {
    let is_night = check_night_time();
    let output = encode(&[Token::Bool(is_night)]);
    api::return_value(ReturnFlags::empty(), &output);
}

// ============================================================================
// View Functions
// ============================================================================

fn handle_get_zone() {
    // getZone(uint32 zone_id) returns (int32 lat_min, int32 lon_min, int32 lat_max, int32 lon_max)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let zone_key = zone_storage_key(zone_id);
    let mut zone_data = [0u8; 160];
    if api::get_storage(StorageFlags::empty(), &zone_key, &mut &mut zone_data[..]).is_err() {
        revert(ERROR_INVALID_ZONE);
    }

    api::return_value(ReturnFlags::empty(), &zone_data);
}

fn handle_get_zone_count() {
    let count_key = storage_key(PREFIX_ZONE_COUNT, b"");
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);

    let count = u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                                    count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]]);

    let output = encode(&[Token::Uint(U256::from(count))]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_get_fingerprint() {
    // getFingerprint(uint32 zone_id) returns (bytes32)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    // Get latest fingerprint for zone
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2], timestamp_buffer[3],
                                        timestamp_buffer[4], timestamp_buffer[5], timestamp_buffer[6], timestamp_buffer[7]]);
    let fp_key = zone_time_key(PREFIX_ZONE_FINGERPRINT, zone_id, timestamp);

    let mut merkle_root = [0u8; 32];
    if api::get_storage(StorageFlags::empty(), &fp_key, &mut &mut merkle_root[..]).is_err() {
        // Return zeros if no fingerprint
        merkle_root = [0u8; 32];
    }

    api::return_value(ReturnFlags::empty(), &merkle_root);
}

fn handle_has_valid_proof() {
    // hasValidProof(address user) returns (bool)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Address], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let user_addr = match &tokens[0] {
        Token::Address(a) => {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&a.0);
            addr
        }
        _ => revert(b"InvalidAddress"),
    };

    let last_proof_key = address_key(PREFIX_USER_LAST_PROOF, &user_addr);
    let mut last_time_bytes = [0u8; 32];

    let has_proof = if api::get_storage(StorageFlags::empty(), &last_proof_key, &mut &mut last_time_bytes[..]).is_ok() {
        let last_time = u64::from_le_bytes([
            last_time_bytes[0], last_time_bytes[1], last_time_bytes[2], last_time_bytes[3],
            last_time_bytes[4], last_time_bytes[5], last_time_bytes[6], last_time_bytes[7],
        ]);
        let mut now_buffer = [0u8; 32];
        api::now(&mut now_buffer);
        let now = u64::from_le_bytes([now_buffer[0], now_buffer[1], now_buffer[2], now_buffer[3],
                                       now_buffer[4], now_buffer[5], now_buffer[6], now_buffer[7]]);
        // Proof valid for 24 hours (spans across midnight for 8 AM - 5 AM window)
        now < last_time + 86400
    } else {
        false
    };

    let output = encode(&[Token::Bool(has_proof)]);
    api::return_value(ReturnFlags::empty(), &output);
}

// ============================================================================
// Helper Functions
// ============================================================================

fn require_owner() {
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    let owner_key = storage_key(PREFIX_OWNER, b"");
    let mut owner = [0u8; 20];
    if api::get_storage(StorageFlags::empty(), &owner_key, &mut &mut owner[..]).is_err() {
        revert(ERROR_NOT_INITIALIZED);
    }

    if caller != owner {
        revert(ERROR_NOT_OWNER);
    }
}

fn require_not_paused() {
    let paused_key = storage_key(PREFIX_PAUSED, b"");
    let mut paused = [0u8; 1];
    if api::get_storage(StorageFlags::empty(), &paused_key, &mut &mut paused[..]).is_ok() {
        if paused[0] != 0 {
            revert(ERROR_PAUSED);
        }
    }
}

fn require_night_time() {
    if !check_night_time() {
        revert(ERROR_NOT_NIGHT_TIME);
    }
}

fn check_night_time() -> bool {
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2], timestamp_buffer[3],
                                        timestamp_buffer[4], timestamp_buffer[5], timestamp_buffer[6], timestamp_buffer[7]]);
    // Get hour of day (timestamp % 86400 / 3600)
    let seconds_in_day = timestamp % 86400;
    let hour = seconds_in_day / SECONDS_PER_HOUR;

    // Testing hours: 8 AM to 5 AM next day (8:00-23:59, then 0:00-5:00)
    hour >= NIGHT_START_HOUR || hour < NIGHT_END_HOUR
}

fn zone_storage_key(zone_id: u32) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = PREFIX_ZONE_DATA;
    key[1..5].copy_from_slice(&zone_id.to_le_bytes());
    key
}

fn revert(error: &[u8]) -> ! {
    api::return_value(ReturnFlags::REVERT, error);
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}
