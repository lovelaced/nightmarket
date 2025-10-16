#![no_std]
#![no_main]
extern crate alloc;

use simplealloc::SimpleAlloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: SimpleAlloc<{ 1024 * 50 }> = SimpleAlloc::new();

use uapi::{HostFn, HostFnImpl as api, StorageFlags, ReturnFlags};
use ethabi::{decode, encode, Token, ParamType, ethereum_types::U256};
use nightmarket_shared::{
    Groth16Proof, verify_groth16,
    safe_add, safe_sub, safe_percentage,
    storage_key, double_mapping_key,
};

include!("../../../shared/src/panic_handler.rs");

// ============================================================================
// Storage Prefixes
// ============================================================================

const PREFIX_OWNER: u8 = 0;
const PREFIX_SCORE: u8 = 1;               // zone_id + ephemeral_id -> score
const PREFIX_LAST_ACTIVITY: u8 = 2;       // zone_id + ephemeral_id -> timestamp
const PREFIX_ESCROW_CONTRACT: u8 = 3;
const PREFIX_PAUSED: u8 = 4;

// ============================================================================
// Constants
// ============================================================================

const SCORE_PER_TRADE: u64 = 10;
const SCORE_PER_NIGHT: u64 = 1;
const DECAY_PERCENTAGE: u64 = 1000;       // 10% decay per week (10% = 1000 basis points)
const WEEK_IN_SECONDS: u64 = 604800;      // 7 days

// ============================================================================
// Function Selectors
// ============================================================================

// Admin
const SELECTOR_INITIALIZE: [u8; 4] = [0x81, 0x29, 0xfc, 0x1c];
const SELECTOR_SET_ESCROW_CONTRACT: [u8; 4] = [0xf4, 0x23, 0x75, 0xb5];
const SELECTOR_SET_PAUSED: [u8; 4] = [0x16, 0xc3, 0x8b, 0x3c];

// User functions
const SELECTOR_UPDATE_SCORE: [u8; 4] = [0x5e, 0x72, 0x7d, 0x76]; // updateScore(uint32,bytes32,int256)
const SELECTOR_PROVE_SCORE_THRESHOLD: [u8; 4] = [0x79, 0x7c, 0xb6, 0x97]; // proveScoreThreshold(uint32,bytes32,bytes,uint256)

// View functions
const SELECTOR_GET_SCORE: [u8; 4] = [0xac, 0x6e, 0xdd, 0x86];    // getScore(uint32,bytes32)
const SELECTOR_GET_DECAYED_SCORE: [u8; 4] = [0xe2, 0x16, 0x6f, 0xed]; // getDecayedScore(uint32,bytes32)

// ============================================================================
// Error Messages
// ============================================================================

const ERROR_NOT_OWNER: &[u8] = b"NotOwner";
const ERROR_PAUSED: &[u8] = b"ContractPaused";
const ERROR_NOT_ESCROW: &[u8] = b"NotEscrowContract";
const ERROR_INVALID_PROOF: &[u8] = b"InvalidProof";
const ERROR_SCORE_TOO_LOW: &[u8] = b"ScoreBelowThreshold";

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

    let paused_key = storage_key(PREFIX_PAUSED, b"");
    api::set_storage(StorageFlags::empty(), &paused_key, &[0u8; 1]);

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
        SELECTOR_SET_ESCROW_CONTRACT => handle_set_escrow_contract(),
        SELECTOR_SET_PAUSED => handle_set_paused(),
        SELECTOR_UPDATE_SCORE => handle_update_score(),
        SELECTOR_PROVE_SCORE_THRESHOLD => handle_prove_score_threshold(),
        SELECTOR_GET_SCORE => handle_get_score(),
        SELECTOR_GET_DECAYED_SCORE => handle_get_decayed_score(),
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

fn handle_set_escrow_contract() {
    require_owner();

    // setEscrowContract(address escrow_contract)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Address], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let escrow_addr = match &tokens[0] {
        Token::Address(a) => {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&a.0);
            addr
        }
        _ => revert(b"InvalidAddress"),
    };

    let escrow_key = storage_key(PREFIX_ESCROW_CONTRACT, b"");
    api::set_storage(StorageFlags::empty(), &escrow_key, &escrow_addr);

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

fn handle_update_score() {
    require_not_paused();
    // Only escrow contract can update scores
    require_escrow();

    // updateScore(uint32 zone_id, bytes32 ephemeral_id, int256 score_delta)
    let mut input = [0u8; 100];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32), ParamType::FixedBytes(32), ParamType::Int(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let ephemeral_id = match &tokens[1] {
        Token::FixedBytes(b) => {
            let mut id = [0u8; 32];
            id.copy_from_slice(&b[..32]);
            id
        }
        _ => revert(b"InvalidId"),
    };

    let score_delta = match &tokens[2] {
        Token::Int(v) => {
            // Simplified: treat as u64 for now
            v.as_u64() as i64
        }
        _ => 0i64,
    };

    // Get current score
    let score_key = get_score_key(zone_id, &ephemeral_id);
    let mut score_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &score_key, &mut &mut score_bytes[..]);
    let current_score = u64::from_le_bytes([score_bytes[0], score_bytes[1], score_bytes[2], score_bytes[3],
                                             score_bytes[4], score_bytes[5], score_bytes[6], score_bytes[7]]);

    // Apply delta
    let new_score = if score_delta >= 0 {
        match safe_add(current_score, score_delta as u64) {
            Ok(s) => s,
            Err(_) => current_score,
        }
    } else {
        match safe_sub(current_score, (-score_delta) as u64) {
            Ok(s) => s,
            Err(_) => 0,
        }
    };

    // Store new score
    score_bytes[..8].copy_from_slice(&new_score.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &score_key, &score_bytes);

    // Update last activity timestamp
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                        timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                        timestamp_buffer[6], timestamp_buffer[7]]);

    let activity_key = get_activity_key(zone_id, &ephemeral_id);
    let mut activity_bytes = [0u8; 32];
    activity_bytes[..8].copy_from_slice(&timestamp.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &activity_key, &activity_bytes);

    // Emit ScoreUpdated event
    let mut topic1 = [0u8; 32];
    topic1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x22; 32], topic1, ephemeral_id];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&new_score.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_prove_score_threshold() {
    require_not_paused();

    // proveScoreThreshold(uint32 zone_id, bytes32 ephemeral_id, bytes proof, uint256 threshold)
    let mut input = [0u8; 512];
    api::call_data_copy(&mut input, 0);

    // Simplified: zone_id(4) + ephemeral_id(32) + proof(256) + threshold(32)
    let zone_id = u32::from_le_bytes([input[4], input[5], input[6], input[7]]);

    let mut ephemeral_id = [0u8; 32];
    ephemeral_id.copy_from_slice(&input[8..40]);

    // Parse proof
    let proof = match Groth16Proof::from_bytes(&input[40..296]) {
        Ok(p) => p,
        Err(e) => revert(e.as_bytes()),
    };

    let threshold = u64::from_le_bytes([input[296], input[297], input[298], input[299],
                                         input[300], input[301], input[302], input[303]]);

    // Get current decayed score
    let score = get_decayed_score_internal(zone_id, &ephemeral_id);

    // Verify ZK proof that score >= threshold
    // Public inputs: [zone_id, ephemeral_id_hash, threshold, score_commitment]
    let mut pub_input_1 = [0u8; 32];
    pub_input_1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let mut pub_input_2 = [0u8; 32];
    pub_input_2[..8].copy_from_slice(&threshold.to_le_bytes());
    let public_inputs = [pub_input_1, ephemeral_id, pub_input_2];
    // Reputation Threshold circuit verification key hash
    let vk_hash = [0x8c, 0xa7, 0x53, 0xb9, 0x62, 0x80, 0xfc, 0xca, 0x98, 0xf4, 0xfa, 0x3f, 0xdd, 0xde, 0x5a, 0xce, 0xda, 0x90, 0xaf, 0x07, 0xb1, 0x85, 0x6b, 0x89, 0x9d, 0xe7, 0xd3, 0xa3, 0x7d, 0x02, 0x5d, 0xd5];

    if let Err(e) = verify_groth16(&proof, &public_inputs, &vk_hash) {
        revert(e.as_bytes());
    }

    // For simplified Phase 1, also check score directly
    if score < threshold {
        revert(ERROR_SCORE_TOO_LOW);
    }

    // Emit ProofVerified event
    let mut topic1 = [0u8; 32];
    topic1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x33; 32], topic1, ephemeral_id];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&threshold.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

// ============================================================================
// View Functions
// ============================================================================

fn handle_get_score() {
    // getScore(uint32 zone_id, bytes32 ephemeral_id)
    let mut input = [0u8; 68];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32), ParamType::FixedBytes(32)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let ephemeral_id = match &tokens[1] {
        Token::FixedBytes(b) => {
            let mut id = [0u8; 32];
            id.copy_from_slice(&b[..32]);
            id
        }
        _ => revert(b"InvalidId"),
    };

    let score_key = get_score_key(zone_id, &ephemeral_id);
    let mut score_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &score_key, &mut &mut score_bytes[..]);
    let score = u64::from_le_bytes([score_bytes[0], score_bytes[1], score_bytes[2], score_bytes[3],
                                     score_bytes[4], score_bytes[5], score_bytes[6], score_bytes[7]]);

    let output = encode(&[Token::Uint(U256::from(score))]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_get_decayed_score() {
    // getDecayedScore(uint32 zone_id, bytes32 ephemeral_id)
    let mut input = [0u8; 68];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32), ParamType::FixedBytes(32)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let ephemeral_id = match &tokens[1] {
        Token::FixedBytes(b) => {
            let mut id = [0u8; 32];
            id.copy_from_slice(&b[..32]);
            id
        }
        _ => revert(b"InvalidId"),
    };

    let decayed_score = get_decayed_score_internal(zone_id, &ephemeral_id);

    let output = encode(&[Token::Uint(U256::from(decayed_score))]);
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

fn require_escrow() {
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    let escrow_key = storage_key(PREFIX_ESCROW_CONTRACT, b"");
    let mut escrow = [0u8; 20];
    if api::get_storage(StorageFlags::empty(), &escrow_key, &mut &mut escrow[..]).is_err() {
        revert(b"EscrowNotSet");
    }

    if caller != escrow {
        revert(ERROR_NOT_ESCROW);
    }
}

fn get_score_key(zone_id: u32, ephemeral_id: &[u8; 32]) -> [u8; 32] {
    let mut zone_bytes = [0u8; 32];
    zone_bytes[..4].copy_from_slice(&zone_id.to_le_bytes());
    double_mapping_key(PREFIX_SCORE, &zone_bytes, ephemeral_id)
}

fn get_activity_key(zone_id: u32, ephemeral_id: &[u8; 32]) -> [u8; 32] {
    let mut zone_bytes = [0u8; 32];
    zone_bytes[..4].copy_from_slice(&zone_id.to_le_bytes());
    double_mapping_key(PREFIX_LAST_ACTIVITY, &zone_bytes, ephemeral_id)
}

fn get_decayed_score_internal(zone_id: u32, ephemeral_id: &[u8; 32]) -> u64 {
    // Get base score
    let score_key = get_score_key(zone_id, ephemeral_id);
    let mut score_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &score_key, &mut &mut score_bytes[..]);
    let base_score = u64::from_le_bytes([score_bytes[0], score_bytes[1], score_bytes[2], score_bytes[3],
                                          score_bytes[4], score_bytes[5], score_bytes[6], score_bytes[7]]);

    if base_score == 0 {
        return 0;
    }

    // Get last activity timestamp
    let activity_key = get_activity_key(zone_id, ephemeral_id);
    let mut activity_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &activity_key, &mut &mut activity_bytes[..]);
    let last_activity = u64::from_le_bytes([activity_bytes[0], activity_bytes[1], activity_bytes[2], activity_bytes[3],
                                             activity_bytes[4], activity_bytes[5], activity_bytes[6], activity_bytes[7]]);

    if last_activity == 0 {
        return base_score;
    }

    // Calculate decay based on time elapsed
    let mut now_buffer = [0u8; 32];
    api::now(&mut now_buffer);
    let now = u64::from_le_bytes([now_buffer[0], now_buffer[1], now_buffer[2], now_buffer[3],
                                   now_buffer[4], now_buffer[5], now_buffer[6], now_buffer[7]]);

    let time_elapsed = match safe_sub(now, last_activity) {
        Ok(t) => t,
        Err(_) => return base_score,
    };

    // Calculate number of weeks
    let weeks_elapsed = time_elapsed / WEEK_IN_SECONDS;

    if weeks_elapsed == 0 {
        return base_score;
    }

    // Apply decay: score * (0.9 ^ weeks)
    // Simplified: subtract 10% per week
    let mut decayed_score = base_score;
    for _ in 0..weeks_elapsed.min(10) {
        decayed_score = match safe_percentage(decayed_score, 10000 - DECAY_PERCENTAGE) {
            Ok(s) => s,
            Err(_) => 0,
        };
    }

    decayed_score
}

fn revert(error: &[u8]) -> ! {
    api::return_value(ReturnFlags::REVERT, error);
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}
