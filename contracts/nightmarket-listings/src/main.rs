#![no_std]
#![no_main]
extern crate alloc;

use simplealloc::SimpleAlloc;
use alloc::vec::Vec;

#[global_allocator]
static GLOBAL_ALLOCATOR: SimpleAlloc<{ 1024 * 50 }> = SimpleAlloc::new();

use uapi::{HostFn, HostFnImpl as api, StorageFlags, ReturnFlags, CallFlags};
use ethabi::{decode, encode, Token, ParamType, ethereum_types::{U256, H160}};
use nightmarket_shared::{
    keccak256, hash_pair,
    safe_add, safe_sub,
    storage_key, address_u64_key, list_key, zone_time_key,
};

include!("../../../shared/src/panic_handler.rs");

// ============================================================================
// Storage Prefixes
// ============================================================================

const PREFIX_OWNER: u8 = 0;
const PREFIX_ZONES_CONTRACT: u8 = 1;
const PREFIX_LISTING_COUNT: u8 = 2;
const PREFIX_LISTING_DATA: u8 = 3;        // listing_id -> ListingData
const PREFIX_ZONE_LISTING_INDEX: u8 = 4; // zone_id -> listing_id[]
const PREFIX_SELLER_LISTINGS: u8 = 5;    // seller -> listing_id[]
const PREFIX_MERKLE_ROOT: u8 = 6;        // Zone-specific merkle root
const PREFIX_PAUSED: u8 = 7;

// List tracking
const PREFIX_ACTIVE_LIST: u8 = 20;
const PREFIX_EXPIRED_LIST: u8 = 21;
const PREFIX_ACTIVE_COUNT: u8 = 22;
const PREFIX_EXPIRED_COUNT: u8 = 23;

// ============================================================================
// Constants
// ============================================================================

const MAX_LISTING_SIZE: usize = 256;
const MAX_BATCH_SIZE: usize = 200;
const SUNRISE_HOUR: u64 = 6;     // 6:00 AM
const SECONDS_PER_HOUR: u64 = 3600;
const MAX_LISTING_LIFETIME: u64 = 86400; // 24 hours max

// ============================================================================
// Function Selectors
// ============================================================================

// Admin
const SELECTOR_INITIALIZE: [u8; 4] = [0x81, 0x29, 0xfc, 0x1c];
const SELECTOR_SET_ZONES_CONTRACT: [u8; 4] = [0x71, 0x1f, 0xab, 0x5f];
const SELECTOR_SET_PAUSED: [u8; 4] = [0x16, 0xc3, 0x8b, 0x3c];

// User functions
const SELECTOR_CREATE_LISTING: [u8; 4] = [0x77, 0xd2, 0x96, 0xaa];  // createListing(uint32,bytes,uint256,bytes32)
const SELECTOR_CANCEL_LISTING: [u8; 4] = [0x30, 0x5a, 0x67, 0xa8];  // cancelListing(uint256)
const SELECTOR_EXPIRE_LISTINGS: [u8; 4] = [0xd3, 0xd7, 0x7f, 0xec]; // expireListings(uint256[])

// View functions
const SELECTOR_GET_LISTING: [u8; 4] = [0x10, 0x7a, 0x27, 0x4a];      // getListing(uint256)
const SELECTOR_GET_LISTINGS_BY_ZONE: [u8; 4] = [0x91, 0x4c, 0x35, 0xdd]; // getListingsByZone(uint32,uint256,uint256)
const SELECTOR_GET_LISTINGS_BATCH: [u8; 4] = [0x9e, 0xea, 0x4a, 0x13]; // getListingsBatch(uint256[])
const SELECTOR_GET_ACTIVE_COUNT: [u8; 4] = [0x63, 0x33, 0x8b, 0x17];    // getActiveCount()
const SELECTOR_GET_LISTING_COUNT: [u8; 4] = [0x87, 0xed, 0x92, 0xd7];   // getListingCount()

// ============================================================================
// Error Messages
// ============================================================================

const ERROR_NOT_OWNER: &[u8] = b"NotOwner";
const ERROR_PAUSED: &[u8] = b"ContractPaused";
const ERROR_INVALID_LISTING: &[u8] = b"InvalidListing";
const ERROR_NOT_SELLER: &[u8] = b"NotSeller";
const ERROR_LISTING_TOO_LARGE: &[u8] = b"ListingTooLarge";
const ERROR_BATCH_TOO_LARGE: &[u8] = b"BatchTooLarge";
const ERROR_ZONES_CONTRACT_NOT_SET: &[u8] = b"ZonesContractNotSet";
const ERROR_NO_LOCATION_PROOF: &[u8] = b"NoLocationProof";
const ERROR_LISTING_EXPIRED: &[u8] = b"ListingExpired";
const ERROR_INVALID_ZONE: &[u8] = b"InvalidZone";

// ============================================================================
// Deploy Function
// ============================================================================

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn deploy() {
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    let owner_key = storage_key(PREFIX_OWNER, b"");
    api::set_storage(StorageFlags::empty(), &owner_key, &caller);

    let count_key = storage_key(PREFIX_LISTING_COUNT, b"");
    let zero = [0u8; 32];
    api::set_storage(StorageFlags::empty(), &count_key, &zero);

    let active_count_key = storage_key(PREFIX_ACTIVE_COUNT, b"");
    api::set_storage(StorageFlags::empty(), &active_count_key, &zero);

    let paused_key = storage_key(PREFIX_PAUSED, b"");
    api::set_storage(StorageFlags::empty(), &paused_key, &[0u8; 1]);

    // Emit Initialized event
    let topics = [[0x11; 32]];
    api::deposit_event(&topics, &caller);
}

// ============================================================================
// Call Function (Router)
// ============================================================================

#[no_mangle]
#[polkavm_derive::polkavm_export]
pub extern "C" fn call() {
    let mut selector = [0u8; 4];
    api::call_data_copy(&mut selector, 0);

    match selector {
        SELECTOR_INITIALIZE => handle_initialize(),
        SELECTOR_SET_ZONES_CONTRACT => handle_set_zones_contract(),
        SELECTOR_SET_PAUSED => handle_set_paused(),
        SELECTOR_CREATE_LISTING => handle_create_listing(),
        SELECTOR_CANCEL_LISTING => handle_cancel_listing(),
        SELECTOR_EXPIRE_LISTINGS => handle_expire_listings(),
        SELECTOR_GET_LISTING => handle_get_listing(),
        SELECTOR_GET_LISTINGS_BY_ZONE => handle_get_listings_by_zone(),
        SELECTOR_GET_LISTINGS_BATCH => handle_get_listings_batch(),
        SELECTOR_GET_ACTIVE_COUNT => handle_get_active_count(),
        SELECTOR_GET_LISTING_COUNT => handle_get_listing_count(),
        _ => {
            api::return_value(ReturnFlags::empty(), &[]);
        }
    }
}

// ============================================================================
// Admin Functions
// ============================================================================

fn handle_initialize() {
    require_owner();
    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_set_zones_contract() {
    require_owner();

    // setZonesContract(address zones_contract)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Address], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zones_addr = match &tokens[0] {
        Token::Address(a) => {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&a.0);
            addr
        }
        _ => revert(b"InvalidAddress"),
    };

    let zones_key = storage_key(PREFIX_ZONES_CONTRACT, b"");
    api::set_storage(StorageFlags::empty(), &zones_key, &zones_addr);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_set_paused() {
    require_owner();

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

fn handle_create_listing() {
    require_not_paused();

    // CRITICAL FIX: Enforce night-time restriction
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                        timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                        timestamp_buffer[6], timestamp_buffer[7]]);
    let seconds_in_day = timestamp % 86400;
    let hour = seconds_in_day / 3600;
    const NIGHT_START_HOUR: u64 = 6;
    const NIGHT_END_HOUR: u64 = 5;
    if !(hour >= NIGHT_START_HOUR || hour < NIGHT_END_HOUR) {
        revert(b"NotNightTime");
    }

    // createListing(uint32 zone_id, bytes encrypted_data, uint256 price, bytes32 drop_zone_hash)
    let input_size = api::call_data_size();
    if input_size < 4 + 32 * 4 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 1024];
    let copy_len = input_size.min(1024) as usize;
    api::call_data_copy(&mut input[..copy_len], 0);

    // Proper ABI decoding
    let tokens = match decode(
        &[ParamType::Uint(32), ParamType::Bytes, ParamType::Uint(256), ParamType::FixedBytes(32)],
        &input[4..copy_len]
    ) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let mut encrypted_data = [0u8; 256];
    match &tokens[1] {
        Token::Bytes(b) => {
            if b.len() != 256 {
                revert(b"InvalidEncryptedDataLength");
            }
            encrypted_data.copy_from_slice(&b[..256]);
        }
        _ => revert(b"InvalidEncryptedData"),
    };

    // CRITICAL FIX: Validate data appears encrypted (entropy check)
    let mut zero_count = 0u32;
    for i in 0..256 {
        if encrypted_data[i] == 0 {
            zero_count += 1;
        }
    }
    // More than 50% zeros suggests not encrypted
    if zero_count > 128 {
        revert(b"DataNotEncrypted");
    }

    let price = match &tokens[2] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidPrice"),
    };

    // CRITICAL FIX: Validate price
    if price == 0 {
        revert(b"PriceCannotBeZero");
    }

    let drop_zone_hash = match &tokens[3] {
        Token::FixedBytes(b) => {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&b[..32]);
            hash
        }
        _ => revert(b"InvalidDropZoneHash"),
    };

    // CRITICAL FIX: Validate drop zone hash is not all zeros
    if drop_zone_hash.iter().all(|&b| b == 0) {
        revert(b"InvalidDropZoneHash");
    }

    // Verify seller has valid location proof (call zones contract)
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    // Get zones contract address from storage
    let zones_key = storage_key(PREFIX_ZONES_CONTRACT, b"");
    let mut zones_addr = [0u8; 20];
    if api::get_storage(StorageFlags::empty(), &zones_key, &mut &mut zones_addr[..]).is_err() {
        revert(ERROR_ZONES_CONTRACT_NOT_SET);
    }

    // Prepare call: hasValidProof(address) -> returns bool
    // Selector: 0x01ae8b7b
    const HAS_VALID_PROOF_SELECTOR: [u8; 4] = [0x01, 0xae, 0x8b, 0x7b];
    let proof_check_input = encode(&[Token::Address(caller.into())]);
    let mut call_data = [0u8; 36];
    call_data[0..4].copy_from_slice(&HAS_VALID_PROOF_SELECTOR);
    call_data[4..36].copy_from_slice(&proof_check_input[..32]);

    // Make the cross-contract call
    let zero_value = [0u8; 32];
    match api::call(
        CallFlags::READ_ONLY,  // Read-only, no state changes
        &zones_addr,
        u64::MAX,              // ref_time limit (use all available)
        u64::MAX,              // proof_size limit
        &[u8::MAX; 32],       // deposit limit
        &zero_value,           // No value transfer
        &call_data,
        None,                  // Don't need output buffer, will use return_data API
    ) {
        Ok(()) => {
            // Get return data (bool encoded as 32 bytes)
            let return_size = api::return_data_size();
            if return_size < 32 {
                revert(b"InvalidReturnData");
            }
            let mut has_proof = [0u8; 32];
            api::return_data_copy(&mut &mut has_proof[..], 0);

            // Check if result is false (last byte is 0 in ABI-encoded bool)
            if has_proof[31] == 0 {
                revert(ERROR_NO_LOCATION_PROOF);
            }
        },
        Err(_) => revert(b"ZonesCallFailed"),
    }

    // Get current timestamp for expiry calculation
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                        timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                        timestamp_buffer[6], timestamp_buffer[7]]);

    // Calculate expiry (next sunrise at 6 AM)
    let seconds_in_day = timestamp % 86400;
    let seconds_until_sunrise = if seconds_in_day < SUNRISE_HOUR * SECONDS_PER_HOUR {
        SUNRISE_HOUR * SECONDS_PER_HOUR - seconds_in_day
    } else {
        86400 - seconds_in_day + SUNRISE_HOUR * SECONDS_PER_HOUR
    };
    let expiry_timestamp = timestamp + seconds_until_sunrise;

    // Generate listing ID
    let listing_id = get_next_listing_id();

    // Store listing data: seller(20) + zone_id(4) + encrypted(256) + price(8) + drop_hash(32) + expiry(8) = 328 bytes
    let mut listing_data = [0u8; 328];
    listing_data[0..20].copy_from_slice(&caller);
    listing_data[20..24].copy_from_slice(&zone_id.to_le_bytes());
    listing_data[24..280].copy_from_slice(&encrypted_data);
    listing_data[280..288].copy_from_slice(&price.to_le_bytes());
    listing_data[288..320].copy_from_slice(&drop_zone_hash);
    listing_data[320..328].copy_from_slice(&expiry_timestamp.to_le_bytes());

    let listing_key = listing_storage_key(listing_id);
    api::set_storage(StorageFlags::empty(), &listing_key, &listing_data);

    // Add to active list
    add_to_active_list(listing_id);

    // Emit ListingCreated event
    let mut topic1 = [0u8; 32];
    topic1[..8].copy_from_slice(&listing_id.to_le_bytes());
    let mut topic2 = [0u8; 32];
    topic2[..20].copy_from_slice(&caller);
    let mut topic3 = [0u8; 32];
    topic3[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x22; 32], topic1, topic2, topic3];

    let mut event_data = [0u8; 40];
    event_data[..8].copy_from_slice(&price.to_le_bytes());
    event_data[8..40].copy_from_slice(&drop_zone_hash);
    api::deposit_event(&topics, &event_data);

    // Return listing ID
    let output = encode(&[Token::Uint(U256::from(listing_id))]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_cancel_listing() {
    require_not_paused();

    // cancelListing(uint256 listing_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let listing_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidListingId"),
    };

    // Get listing data
    let listing_key = listing_storage_key(listing_id);
    let mut listing_data = [0u8; 328];
    if api::get_storage(StorageFlags::empty(), &listing_key, &mut &mut listing_data[..]).is_err() {
        revert(ERROR_INVALID_LISTING);
    }

    // Verify caller is seller
    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    let seller = &listing_data[0..20];
    if caller.as_slice() != seller {
        revert(ERROR_NOT_SELLER);
    }

    // Clear listing (set to empty to get gas refund)
    api::set_storage(StorageFlags::empty(), &listing_key, &[]);

    // Remove from active list (for simplicity, just mark as expired)
    remove_from_active_list(listing_id);

    // Emit ListingCancelled event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&listing_id.to_le_bytes());
    let topics = [[0x33; 32], topic];
    api::deposit_event(&topics, &[]);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_expire_listings() {
    // expireListings(uint256[] listing_ids)
    let input_size = api::call_data_size();
    if input_size < 4 + 32 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 512];
    let copy_len = input_size.min(512);
    api::call_data_copy(&mut input, 0);

    // Simplified: expect array of listing IDs
    // For now, support up to 10 listings per batch
    let max_listings = ((copy_len - 4) / 32).min(10);

    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let now = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                   timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                   timestamp_buffer[6], timestamp_buffer[7]]);

    let mut expired_count = 0u32;

    for i in 0..max_listings {
        let offset = 4 + i * 32;
        let listing_id = u64::from_le_bytes([input[offset as usize], input[offset as usize+1], input[offset as usize+2], input[offset as usize+3],
                                              input[offset as usize+4], input[offset as usize+5], input[offset as usize+6], input[offset as usize+7]]);

        if listing_id == 0 {
            continue;
        }

        // Get listing
        let listing_key = listing_storage_key(listing_id);
        let mut listing_data = [0u8; 328];
        if api::get_storage(StorageFlags::empty(), &listing_key, &mut &mut listing_data[..]).is_err() {
            continue; // Skip invalid listings
        }

        // Check if expired
        let expiry = u64::from_le_bytes([listing_data[320], listing_data[321], listing_data[322], listing_data[323],
                                          listing_data[324], listing_data[325], listing_data[326], listing_data[327]]);

        if now >= expiry {
            // Clear listing (gas refund)
            api::set_storage(StorageFlags::empty(), &listing_key, &[]);
            remove_from_active_list(listing_id);
            expired_count += 1;
        }
    }

    // Return expired count
    let output = encode(&[Token::Uint(U256::from(expired_count))]);
    api::return_value(ReturnFlags::empty(), &output);
}

// ============================================================================
// View Functions
// ============================================================================

fn handle_get_listing() {
    // getListing(uint256 listing_id) returns (address,uint32,bytes,uint256,bytes32,uint256)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let listing_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidListingId"),
    };

    let listing_key = listing_storage_key(listing_id);
    let mut listing_data = [0u8; 328];
    if api::get_storage(StorageFlags::empty(), &listing_key, &mut &mut listing_data[..]).is_err() {
        revert(ERROR_INVALID_LISTING);
    }

    // Check not expired
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let now = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                   timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                   timestamp_buffer[6], timestamp_buffer[7]]);

    let expiry = u64::from_le_bytes([listing_data[320], listing_data[321], listing_data[322], listing_data[323],
                                      listing_data[324], listing_data[325], listing_data[326], listing_data[327]]);

    if now >= expiry {
        revert(ERROR_LISTING_EXPIRED);
    }

    // Return listing data
    api::return_value(ReturnFlags::empty(), &listing_data);
}

fn handle_get_listings_by_zone() {
    // getListingsByZone(uint32 zone_id, uint256 offset, uint256 limit) returns (uint256[])
    let mut input = [0u8; 100];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32), ParamType::Uint(256), ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let offset = match &tokens[1] {
        Token::Uint(v) => v.as_u64(),
        _ => 0,
    };

    let limit = match &tokens[2] {
        Token::Uint(v) => v.as_u64().min(100),
        _ => 100,
    };

    // Filter active listings by zone_id
    let active_count = get_active_count();
    let mut result_ids = Vec::new();
    let mut found = 0u64;
    let mut scanned = 0u64;

    // Iterate through active list and filter by zone
    for i in 0..active_count {
        let key = list_key(PREFIX_ACTIVE_LIST, i);
        let mut id_bytes = [0u8; 8];
        if api::get_storage(StorageFlags::empty(), &key, &mut &mut id_bytes[..]).is_ok() {
            let listing_id = u64::from_le_bytes(id_bytes);

            // Load listing to check zone_id
            let listing_key = listing_storage_key(listing_id);
            let mut listing_data = [0u8; 328];
            if api::get_storage(StorageFlags::empty(), &listing_key, &mut &mut listing_data[..]).is_ok() {
                // Zone ID is at bytes 20-24
                let listing_zone_id = u32::from_le_bytes([listing_data[20], listing_data[21],
                                                           listing_data[22], listing_data[23]]);

                if listing_zone_id == zone_id {
                    // Apply offset and limit
                    if scanned >= offset && found < limit {
                        result_ids.push(Token::Uint(U256::from(listing_id)));
                        found += 1;
                    }
                    scanned += 1;

                    if found >= limit {
                        break;
                    }
                }
            }
        }
    }

    let output = encode(&[Token::Array(result_ids)]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_get_listings_batch() {
    // getListingsBatch(uint256[] ids) returns (bytes[])
    let input_size = api::call_data_size();
    if input_size < 4 + 32 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 512];
    let copy_len = input_size.min(512);
    api::call_data_copy(&mut input, 0);

    // Simplified: read up to 10 listing IDs
    let max_items = ((copy_len - 4) / 32).min(10);

    let mut results = Vec::new();
    for i in 0..max_items {
        let offset = 4 + i * 32;
        let listing_id = u64::from_le_bytes([input[offset as usize], input[offset as usize +1], input[offset as usize +2], input[offset as usize +3],
                                              input[offset as usize +4], input[offset as usize +5], input[offset as usize +6], input[offset as usize +7]]);

        if listing_id == 0 {
            continue;
        }

        let listing_key = listing_storage_key(listing_id);
        let mut listing_data = [0u8; 328];
        if api::get_storage(StorageFlags::empty(), &listing_key, &mut &mut listing_data[..]).is_ok() {
            results.push(Token::Bytes(listing_data.to_vec()));
        }
    }

    let output = encode(&[Token::Array(results)]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_get_active_count() {
    let count = get_active_count();
    let output = encode(&[Token::Uint(U256::from(count))]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_get_listing_count() {
    let count_key = storage_key(PREFIX_LISTING_COUNT, b"");
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);
    let count = u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                                     count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]]);

    let output = encode(&[Token::Uint(U256::from(count))]);
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
        revert(b"NotInitialized");
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

fn get_next_listing_id() -> u64 {
    let count_key = storage_key(PREFIX_LISTING_COUNT, b"");
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);
    let count = u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                                     count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]]);
    let new_count = count + 1;
    let mut new_count_bytes = [0u8; 32];
    new_count_bytes[..8].copy_from_slice(&new_count.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &count_key, &new_count_bytes);
    new_count
}

fn get_active_count() -> u64 {
    let count_key = storage_key(PREFIX_ACTIVE_COUNT, b"");
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);
    u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                        count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]])
}

fn add_to_active_list(listing_id: u64) {
    let count = get_active_count();
    let key = list_key(PREFIX_ACTIVE_LIST, count);
    let id_bytes = listing_id.to_le_bytes();
    api::set_storage(StorageFlags::empty(), &key, &id_bytes);

    // Increment count
    let count_key = storage_key(PREFIX_ACTIVE_COUNT, b"");
    let mut new_count_bytes = [0u8; 32];
    new_count_bytes[..8].copy_from_slice(&(count + 1).to_le_bytes());
    api::set_storage(StorageFlags::empty(), &count_key, &new_count_bytes);
}

fn remove_from_active_list(listing_id: u64) {
    // Swap-and-pop removal to maintain list integrity
    let count = get_active_count();
    if count == 0 {
        return;
    }

    // Find the index of the listing_id in the active list
    let mut found_index: Option<u64> = None;
    for i in 0..count {
        let key = list_key(PREFIX_ACTIVE_LIST, i);
        let mut id_bytes = [0u8; 8];
        if api::get_storage(StorageFlags::empty(), &key, &mut &mut id_bytes[..]).is_ok() {
            let id = u64::from_le_bytes(id_bytes);
            if id == listing_id {
                found_index = Some(i);
                break;
            }
        }
    }

    // If found, swap with last element and pop
    if let Some(index) = found_index {
        let last_index = count - 1;

        if index != last_index {
            // Get last element
            let last_key = list_key(PREFIX_ACTIVE_LIST, last_index);
            let mut last_id_bytes = [0u8; 8];
            if api::get_storage(StorageFlags::empty(), &last_key, &mut &mut last_id_bytes[..]).is_ok() {
                // Swap: write last element to found position
                let found_key = list_key(PREFIX_ACTIVE_LIST, index);
                api::set_storage(StorageFlags::empty(), &found_key, &last_id_bytes);
            }
        }

        // Clear last position (gas refund)
        let last_key = list_key(PREFIX_ACTIVE_LIST, last_index);
        api::set_storage(StorageFlags::empty(), &last_key, &[]);

        // Decrement count
        let count_key = storage_key(PREFIX_ACTIVE_COUNT, b"");
        let mut new_count_bytes = [0u8; 32];
        new_count_bytes[..8].copy_from_slice(&last_index.to_le_bytes());
        api::set_storage(StorageFlags::empty(), &count_key, &new_count_bytes);
    }
}

fn listing_storage_key(listing_id: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = PREFIX_LISTING_DATA;
    key[1..9].copy_from_slice(&listing_id.to_le_bytes());
    key
}

fn revert(error: &[u8]) -> ! {
    api::return_value(ReturnFlags::REVERT, error);
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}
