#![no_std]
#![no_main]
extern crate alloc;

use simplealloc::SimpleAlloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: SimpleAlloc<{ 1024 * 50 }> = SimpleAlloc::new();

use uapi::{HostFn, HostFnImpl as api, StorageFlags, ReturnFlags, CallFlags};
use ethabi::{decode, encode, Token, ParamType, ethereum_types::U256};
use nightmarket_shared::{
    Groth16Proof, verify_groth16, derive_nullifier, keccak256,
    safe_add, safe_sub, safe_percentage,
    storage_key, double_mapping_key, zone_time_key,
};

include!("../../../shared/src/panic_handler.rs");

// ============================================================================
// Storage Prefixes
// ============================================================================

const PREFIX_OWNER: u8 = 0;
const PREFIX_POOL_BALANCE: u8 = 1;        // zone_id + night_timestamp -> balance
const PREFIX_NULLIFIER: u8 = 2;           // nullifier -> bool
const PREFIX_DEPOSIT_COMMITMENT: u8 = 3;  // commitment -> deposit_data
const PREFIX_WITHDRAWAL_DELAY: u8 = 4;    // address -> random_delay_timestamp
const PREFIX_PAUSED: u8 = 5;
const PREFIX_MIN_DEPOSIT: u8 = 6;
const PREFIX_DEPOSIT_COUNT: u8 = 7;       // zone_id + night -> deposit_count
const PREFIX_ACCUMULATED_FEES: u8 = 8;    // Total accumulated fees

// ============================================================================
// Constants
// ============================================================================

const MIN_DEPOSIT_WEI: u64 = 10_000_000_000_000_000; // 0.01 ETH
const MIN_DELAY_SECONDS: u64 = 600;       // 10 minutes
const MAX_DELAY_SECONDS: u64 = 1800;      // 30 minutes
const NIGHT_DURATION: u64 = 10800;        // 3 hours (2 AM - 5 AM)
const FEE_BASIS_POINTS: u64 = 100;        // 1% fee

// ============================================================================
// Function Selectors
// ============================================================================

// Admin
const SELECTOR_INITIALIZE: [u8; 4] = [0x81, 0x29, 0xfc, 0x1c];  // initialize()
const SELECTOR_SET_PAUSED: [u8; 4] = [0x16, 0xc3, 0x8b, 0x3c];  // setPaused(bool)
const SELECTOR_WITHDRAW_FEES: [u8; 4] = [0x47, 0x63, 0x43, 0xee];  // withdrawFees()

// User functions
const SELECTOR_DEPOSIT: [u8; 4] = [0x65, 0x01, 0xf9, 0xc7];  // deposit(uint32,bytes32)
const SELECTOR_WITHDRAW: [u8; 4] = [0x91, 0xf5, 0x19, 0x0e];  // withdraw(uint32,bytes,bytes32,address)

// View functions
const SELECTOR_GET_POOL_BALANCE: [u8; 4] = [0x33, 0x1b, 0x8c, 0x2b];  // getPoolBalance(uint32,uint256)
const SELECTOR_IS_NULLIFIER_USED: [u8; 4] = [0x22, 0xdc, 0x7b, 0x4c];  // isNullifierUsed(bytes32)
const SELECTOR_GET_MIN_DEPOSIT: [u8; 4] = [0x0e, 0xaa, 0xd3, 0xf1];  // getMinDeposit()

// ============================================================================
// Error Messages
// ============================================================================

const ERROR_NOT_OWNER: &[u8] = b"NotOwner";
const ERROR_PAUSED: &[u8] = b"ContractPaused";
const ERROR_INSUFFICIENT_VALUE: &[u8] = b"InsufficientValue";
const ERROR_NULLIFIER_USED: &[u8] = b"NullifierAlreadyUsed";
const ERROR_INVALID_PROOF: &[u8] = b"InvalidProof";
const ERROR_WITHDRAWAL_TOO_SOON: &[u8] = b"WithdrawalTooSoon";
const ERROR_INSUFFICIENT_POOL: &[u8] = b"InsufficientPoolBalance";
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

    let paused_key = storage_key(PREFIX_PAUSED, b"");
    api::set_storage(StorageFlags::empty(), &paused_key, &[0u8; 1]);

    // Set minimum deposit
    let min_deposit_key = storage_key(PREFIX_MIN_DEPOSIT, b"");
    let mut min_bytes = [0u8; 32];
    min_bytes[..8].copy_from_slice(&MIN_DEPOSIT_WEI.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &min_deposit_key, &min_bytes);

    // Initialize accumulated fees to zero
    let fees_key = storage_key(PREFIX_ACCUMULATED_FEES, b"");
    let zero = [0u8; 32];
    api::set_storage(StorageFlags::empty(), &fees_key, &zero);

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
        SELECTOR_SET_PAUSED => handle_set_paused(),
        SELECTOR_WITHDRAW_FEES => handle_withdraw_fees(),
        SELECTOR_DEPOSIT => handle_deposit(),
        SELECTOR_WITHDRAW => handle_withdraw(),
        SELECTOR_GET_POOL_BALANCE => handle_get_pool_balance(),
        SELECTOR_IS_NULLIFIER_USED => handle_is_nullifier_used(),
        SELECTOR_GET_MIN_DEPOSIT => handle_get_min_deposit(),
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

fn handle_withdraw_fees() {
    require_owner();

    // Get accumulated fees
    let fees_key = storage_key(PREFIX_ACCUMULATED_FEES, b"");
    let mut fees_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &fees_key, &mut &mut fees_bytes[..]);
    let total_fees = u64::from_le_bytes([fees_bytes[0], fees_bytes[1], fees_bytes[2], fees_bytes[3],
                                          fees_bytes[4], fees_bytes[5], fees_bytes[6], fees_bytes[7]]);

    if total_fees == 0 {
        revert(b"NoFeesToWithdraw");
    }

    // Reset accumulated fees to zero
    let zero = [0u8; 32];
    api::set_storage(StorageFlags::empty(), &fees_key, &zero);

    // Transfer fees to owner (caller is already verified as owner by require_owner())
    let mut owner = [0u8; 20];
    api::caller(&mut owner);

    let mut fee_value = [0u8; 32];
    fee_value[..8].copy_from_slice(&total_fees.to_le_bytes());

    match api::call(
        CallFlags::empty(),
        &owner,
        u64::MAX,              // ref_time limit
        u64::MAX,              // proof_size limit
        &[u8::MAX; 32],       // deposit limit
        &fee_value,
        &[],
        None,
    ) {
        Ok(()) => { /* Transfer successful */ },
        Err(_) => revert(b"TransferFailed"),
    }

    // Emit FeesWithdrawn event
    let topics = [[0x99; 32]];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&total_fees.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    let output = encode(&[Token::Uint(U256::from(total_fees))]);
    api::return_value(ReturnFlags::empty(), &output);
}

// ============================================================================
// User Functions
// ============================================================================

fn handle_deposit() {
    require_not_paused();

    // deposit(uint32 zone_id, bytes32 commitment)
    let input_size = api::call_data_size();
    if input_size != 4 + 64 {
        revert(b"InvalidInput");
    }

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

    let commitment = match &tokens[1] {
        Token::FixedBytes(b) => {
            let mut c = [0u8; 32];
            c.copy_from_slice(&b[..32]);
            c
        }
        _ => revert(b"InvalidCommitment"),
    };

    // Check value transferred
    let mut value_buffer = [0u8; 32];
    api::value_transferred(&mut value_buffer);
    let value = u64::from_le_bytes([value_buffer[0], value_buffer[1], value_buffer[2], value_buffer[3],
                                     value_buffer[4], value_buffer[5], value_buffer[6], value_buffer[7]]);

    if value < MIN_DEPOSIT_WEI {
        revert(ERROR_INSUFFICIENT_VALUE);
    }

    // Get current night timestamp (rounded to start of night)
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                        timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                        timestamp_buffer[6], timestamp_buffer[7]]);
    let night_timestamp = get_night_start(timestamp);

    // Add to pool balance for this zone+night
    let pool_key = zone_time_key(PREFIX_POOL_BALANCE, zone_id, night_timestamp);
    let mut pool_balance = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &pool_key, &mut &mut pool_balance[..]);
    let current_balance = u64::from_le_bytes([pool_balance[0], pool_balance[1], pool_balance[2], pool_balance[3],
                                               pool_balance[4], pool_balance[5], pool_balance[6], pool_balance[7]]);

    let new_balance = match safe_add(current_balance, value) {
        Ok(b) => b,
        Err(e) => revert(e.as_bytes()),
    };

    pool_balance[..8].copy_from_slice(&new_balance.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &pool_key, &pool_balance);

    // Store commitment
    let commitment_key = storage_key(PREFIX_DEPOSIT_COMMITMENT, &commitment);
    let mut deposit_data = [0u8; 44]; // zone_id(4) + night(8) + value(8) + timestamp(8) + depositor(20) - actually 48
    deposit_data[0..4].copy_from_slice(&zone_id.to_le_bytes());
    deposit_data[4..12].copy_from_slice(&night_timestamp.to_le_bytes());
    deposit_data[12..20].copy_from_slice(&value.to_le_bytes());
    deposit_data[20..28].copy_from_slice(&timestamp.to_le_bytes());

    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    let mut full_deposit_data = [0u8; 48];
    full_deposit_data[..28].copy_from_slice(&deposit_data[..28]);
    full_deposit_data[28..48].copy_from_slice(&caller);

    api::set_storage(StorageFlags::empty(), &commitment_key, &full_deposit_data);

    // Increment deposit count
    let count_key = zone_time_key(PREFIX_DEPOSIT_COUNT, zone_id, night_timestamp);
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);
    let count = u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                                     count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]]);
    count_bytes[..8].copy_from_slice(&(count + 1).to_le_bytes());
    api::set_storage(StorageFlags::empty(), &count_key, &count_bytes);

    // Emit Deposit event
    let mut topic1 = [0u8; 32];
    topic1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let topics = [[0x22; 32], topic1, commitment];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&value.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_withdraw() {
    require_not_paused();

    // withdraw(uint32 zone_id, bytes proof, bytes32 nullifier, address recipient)
    let input_size = api::call_data_size();
    if input_size < 4 + 32 * 3 + 256 {
        revert(b"InvalidInput");
    }

    let mut input = [0u8; 512];
    let copy_len = input_size.min(512);
    api::call_data_copy(&mut input, 0);

    // Simplified: zone_id(4) + proof(256) + nullifier(32) + recipient(20)
    let zone_id = u32::from_le_bytes([input[4], input[5], input[6], input[7]]);

    // Parse proof
    let proof = match Groth16Proof::from_bytes(&input[8..264]) {
        Ok(p) => p,
        Err(e) => revert(e.as_bytes()),
    };

    // Get nullifier
    let mut nullifier = [0u8; 32];
    nullifier.copy_from_slice(&input[264..296]);

    // Get recipient
    let mut recipient = [0u8; 20];
    recipient.copy_from_slice(&input[296..316]);

    // Check nullifier not used
    let nullifier_key = storage_key(PREFIX_NULLIFIER, &nullifier);
    let mut check_buffer = [0u8; 1];
    if api::get_storage(StorageFlags::empty(), &nullifier_key, &mut &mut check_buffer[..]).is_ok() {
        revert(ERROR_NULLIFIER_USED);
    }

    // Verify ZK proof
    // Public inputs: [zone_id, nullifier]
    let mut pub_input_1 = [0u8; 32];
    pub_input_1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let public_inputs = [pub_input_1, nullifier];
    // Mixer Withdrawal circuit verification key hash
    let vk_hash = [0xd0, 0xd1, 0x99, 0x14, 0xb4, 0x07, 0xd3, 0xaa, 0xc5, 0xac, 0x5b, 0xc5, 0x2e, 0x9c, 0xc9, 0xa2, 0x7c, 0x99, 0x74, 0xf7, 0x01, 0x9c, 0x86, 0x28, 0x3d, 0xea, 0x66, 0xb8, 0xac, 0x5d, 0x3b, 0x7f];

    if let Err(e) = verify_groth16(&proof, &public_inputs, &vk_hash) {
        revert(e.as_bytes());
    }

    // Check random delay
    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    let delay_key = storage_key(PREFIX_WITHDRAWAL_DELAY, &caller);
    let mut delay_buffer = [0u8; 32];
    if api::get_storage(StorageFlags::empty(), &delay_key, &mut &mut delay_buffer[..]).is_ok() {
        let delay_until = u64::from_le_bytes([delay_buffer[0], delay_buffer[1], delay_buffer[2], delay_buffer[3],
                                               delay_buffer[4], delay_buffer[5], delay_buffer[6], delay_buffer[7]]);

        let mut now_buffer = [0u8; 32];
        api::now(&mut now_buffer);
        let now = u64::from_le_bytes([now_buffer[0], now_buffer[1], now_buffer[2], now_buffer[3],
                                       now_buffer[4], now_buffer[5], now_buffer[6], now_buffer[7]]);

        if now < delay_until {
            revert(ERROR_WITHDRAWAL_TOO_SOON);
        }
    }

    // For Phase 1, use fixed withdrawal amount (in production, would be proven via ZK)
    // Assume withdrawal is for MIN_DEPOSIT_WEI
    let withdrawal_amount = MIN_DEPOSIT_WEI;

    // Calculate fee
    let fee = match safe_percentage(withdrawal_amount, FEE_BASIS_POINTS) {
        Ok(f) => f,
        Err(e) => revert(e.as_bytes()),
    };

    let amount_after_fee = match safe_sub(withdrawal_amount, fee) {
        Ok(a) => a,
        Err(e) => revert(e.as_bytes()),
    };

    // Get current night
    let mut now_buffer = [0u8; 32];
    api::now(&mut now_buffer);
    let timestamp = u64::from_le_bytes([now_buffer[0], now_buffer[1], now_buffer[2], now_buffer[3],
                                        now_buffer[4], now_buffer[5], now_buffer[6], now_buffer[7]]);
    let night_timestamp = get_night_start(timestamp);

    // Check pool has sufficient balance
    let pool_key = zone_time_key(PREFIX_POOL_BALANCE, zone_id, night_timestamp);
    let mut pool_balance = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &pool_key, &mut &mut pool_balance[..]);
    let current_pool = u64::from_le_bytes([pool_balance[0], pool_balance[1], pool_balance[2], pool_balance[3],
                                            pool_balance[4], pool_balance[5], pool_balance[6], pool_balance[7]]);

    if current_pool < withdrawal_amount {
        revert(ERROR_INSUFFICIENT_POOL);
    }

    // Mark nullifier as used
    api::set_storage(StorageFlags::empty(), &nullifier_key, &[1u8]);

    // Update pool balance
    let new_pool = match safe_sub(current_pool, withdrawal_amount) {
        Ok(b) => b,
        Err(e) => revert(e.as_bytes()),
    };
    pool_balance[..8].copy_from_slice(&new_pool.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &pool_key, &pool_balance);

    // Track accumulated fees
    let fees_key = storage_key(PREFIX_ACCUMULATED_FEES, b"");
    let mut fees_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &fees_key, &mut &mut fees_bytes[..]);
    let current_fees = u64::from_le_bytes([fees_bytes[0], fees_bytes[1], fees_bytes[2], fees_bytes[3],
                                            fees_bytes[4], fees_bytes[5], fees_bytes[6], fees_bytes[7]]);
    let new_fees = match safe_add(current_fees, fee) {
        Ok(f) => f,
        Err(e) => revert(e.as_bytes()),
    };
    fees_bytes[..8].copy_from_slice(&new_fees.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &fees_key, &fees_bytes);

    // Transfer funds to recipient
    let mut withdraw_value = [0u8; 32];
    withdraw_value[..8].copy_from_slice(&amount_after_fee.to_le_bytes());

    match api::call(
        CallFlags::empty(),
        &recipient,
        u64::MAX,              // ref_time limit
        u64::MAX,              // proof_size limit
        &[u8::MAX; 32],       // deposit limit
        &withdraw_value,
        &[],
        None,
    ) {
        Ok(()) => { /* Transfer successful */ },
        Err(_) => revert(b"TransferFailed"),
    }

    // Set random delay for next withdrawal (10-30 minutes)
    let random_delay = MIN_DELAY_SECONDS + (timestamp % (MAX_DELAY_SECONDS - MIN_DELAY_SECONDS));
    let next_allowed = timestamp + random_delay;
    let mut delay_bytes = [0u8; 32];
    delay_bytes[..8].copy_from_slice(&next_allowed.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &delay_key, &delay_bytes);

    // Emit Withdrawal event
    let mut topic1 = [0u8; 32];
    topic1[..4].copy_from_slice(&zone_id.to_le_bytes());
    let mut topic2 = [0u8; 32];
    topic2[..20].copy_from_slice(&recipient);
    let topics = [[0x33; 32], topic1, topic2, nullifier];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&amount_after_fee.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

// ============================================================================
// View Functions
// ============================================================================

fn handle_get_pool_balance() {
    // getPoolBalance(uint32 zone_id, uint256 night_timestamp)
    let mut input = [0u8; 68];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(32), ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let zone_id = match &tokens[0] {
        Token::Uint(v) => v.as_u32(),
        _ => revert(b"InvalidZoneId"),
    };

    let night_timestamp = match &tokens[1] {
        Token::Uint(v) => v.as_u64(),
        _ => 0,
    };

    let pool_key = zone_time_key(PREFIX_POOL_BALANCE, zone_id, night_timestamp);
    let mut pool_balance = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &pool_key, &mut &mut pool_balance[..]);
    let balance = u64::from_le_bytes([pool_balance[0], pool_balance[1], pool_balance[2], pool_balance[3],
                                       pool_balance[4], pool_balance[5], pool_balance[6], pool_balance[7]]);

    let output = encode(&[Token::Uint(U256::from(balance))]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_is_nullifier_used() {
    // isNullifierUsed(bytes32 nullifier)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::FixedBytes(32)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let nullifier = match &tokens[0] {
        Token::FixedBytes(b) => {
            let mut n = [0u8; 32];
            n.copy_from_slice(&b[..32]);
            n
        }
        _ => revert(b"InvalidNullifier"),
    };

    let nullifier_key = storage_key(PREFIX_NULLIFIER, &nullifier);
    let mut check_buffer = [0u8; 1];
    let is_used = api::get_storage(StorageFlags::empty(), &nullifier_key, &mut &mut check_buffer[..]).is_ok();

    let output = encode(&[Token::Bool(is_used)]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_get_min_deposit() {
    let min_deposit_key = storage_key(PREFIX_MIN_DEPOSIT, b"");
    let mut min_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &min_deposit_key, &mut &mut min_bytes[..]);
    let min_deposit = u64::from_le_bytes([min_bytes[0], min_bytes[1], min_bytes[2], min_bytes[3],
                                           min_bytes[4], min_bytes[5], min_bytes[6], min_bytes[7]]);

    let output = encode(&[Token::Uint(U256::from(min_deposit))]);
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

fn get_night_start(timestamp: u64) -> u64 {
    // Round down to start of night (2 AM)
    let seconds_in_day = timestamp % 86400;
    let night_start = 2 * 3600; // 2 AM in seconds

    if seconds_in_day >= night_start {
        // Current day's night
        timestamp - (seconds_in_day - night_start)
    } else {
        // Previous day's night
        timestamp - seconds_in_day - (86400 - night_start)
    }
}

fn revert(error: &[u8]) -> ! {
    api::return_value(ReturnFlags::REVERT, error);
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}
