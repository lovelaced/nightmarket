#![no_std]
#![no_main]
extern crate alloc;

use simplealloc::SimpleAlloc;

#[global_allocator]
static GLOBAL_ALLOCATOR: SimpleAlloc<{ 1024 * 50 }> = SimpleAlloc::new();

use uapi::{HostFn, HostFnImpl as api, StorageFlags, ReturnFlags, CallFlags};
use ethabi::{decode, encode, Token, ParamType, ethereum_types::U256};
use nightmarket_shared::{
    safe_add, safe_sub, safe_percentage,
    storage_key,
};

include!("../../../shared/src/panic_handler.rs");

// ============================================================================
// Storage Prefixes
// ============================================================================

const PREFIX_OWNER: u8 = 0;
const PREFIX_TRADE_DATA: u8 = 1;          // trade_id -> TradeData
const PREFIX_TRADE_COUNT: u8 = 2;
const PREFIX_COORDINATE_STAGE: u8 = 3;    // trade_id -> current_stage
const PREFIX_HEARTBEAT: u8 = 4;           // trade_id -> last_heartbeat
const PREFIX_PAUSED: u8 = 5;
const PREFIX_ACCUMULATED_FEES: u8 = 6;    // Total accumulated fees

// Trade states
const STATE_CREATED: u8 = 0;
const STATE_LOCKED: u8 = 1;
const STATE_COORDINATES_REVEALED: u8 = 2;
const STATE_COMPLETED: u8 = 3;
const STATE_DISPUTED: u8 = 4;
const STATE_CANCELLED: u8 = 5;

// ============================================================================
// Constants
// ============================================================================

const DISPUTE_WINDOW: u64 = 1800;         // 30 minutes
const HEARTBEAT_INTERVAL: u64 = 1200;     // 20 minutes
const MAX_TRADE_DURATION: u64 = 7200;     // 2 hours
const NUM_COORDINATE_STAGES: u8 = 4;      // 4 stages of revelation
const FEE_BASIS_POINTS: u64 = 100;        // 1% escrow fee

// ============================================================================
// Function Selectors
// ============================================================================

// Admin
const SELECTOR_INITIALIZE: [u8; 4] = [0x81, 0x29, 0xfc, 0x1c];
const SELECTOR_SET_PAUSED: [u8; 4] = [0x16, 0xc3, 0x8b, 0x3c];
const SELECTOR_WITHDRAW_FEES: [u8; 4] = [0x47, 0x6d, 0x39, 0x8e];

// User functions
const SELECTOR_CREATE_TRADE: [u8; 4] = [0x63, 0x5c, 0xf1, 0x8e];  // createTrade(uint256,address,uint256)
const SELECTOR_LOCK_FUNDS: [u8; 4] = [0x0d, 0x2e, 0xac, 0xfa];    // lockFunds(uint256)
const SELECTOR_CANCEL_TRADE: [u8; 4] = [0x2e, 0x1a, 0x7d, 0x4d];  // cancelTrade(uint256)
const SELECTOR_REVEAL_COORDINATES: [u8; 4] = [0xee, 0x48, 0x3a, 0xcd]; // revealCoordinates(uint256,uint8,bytes)
const SELECTOR_SUBMIT_HEARTBEAT: [u8; 4] = [0x1e, 0xef, 0x45, 0x27]; // submitHeartbeat(uint256)
const SELECTOR_COMPLETE_TRADE: [u8; 4] = [0x90, 0x79, 0xd4, 0xc4]; // completeTrade(uint256)
const SELECTOR_DISPUTE_TRADE: [u8; 4] = [0xe5, 0x52, 0x16, 0x21]; // disputeTrade(uint256)
const SELECTOR_RESOLVE_DISPUTE: [u8; 4] = [0x34, 0xb2, 0x5e, 0xe2]; // resolveDispute(uint256,bool)

// View functions
const SELECTOR_GET_TRADE: [u8; 4] = [0x2d, 0xb2, 0x5e, 0x05];     // getTrade(uint256)
const SELECTOR_GET_COORDINATES: [u8; 4] = [0x13, 0x54, 0xe3, 0x77]; // getCoordinates(uint256,uint8)
const SELECTOR_GET_TRADE_STATE: [u8; 4] = [0xc5, 0x96, 0x94, 0xcf]; // getTradeState(uint256)

// ============================================================================
// Error Messages
// ============================================================================

const ERROR_NOT_OWNER: &[u8] = b"NotOwner";
const ERROR_PAUSED: &[u8] = b"ContractPaused";
const ERROR_INVALID_TRADE: &[u8] = b"InvalidTrade";
const ERROR_NOT_BUYER: &[u8] = b"NotBuyer";
const ERROR_NOT_SELLER: &[u8] = b"NotSeller";
const ERROR_NOT_PARTY: &[u8] = b"NotPartyToTrade";
const ERROR_INSUFFICIENT_VALUE: &[u8] = b"InsufficientValue";
const ERROR_INVALID_STATE: &[u8] = b"InvalidState";
const ERROR_HEARTBEAT_EXPIRED: &[u8] = b"HeartbeatExpired";
const ERROR_DISPUTE_WINDOW_PASSED: &[u8] = b"DisputeWindowPassed";

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

    let count_key = storage_key(PREFIX_TRADE_COUNT, b"");
    let zero = [0u8; 32];
    api::set_storage(StorageFlags::empty(), &count_key, &zero);

    let paused_key = storage_key(PREFIX_PAUSED, b"");
    api::set_storage(StorageFlags::empty(), &paused_key, &[0u8; 1]);

    // Initialize accumulated fees to zero
    let fees_key = storage_key(PREFIX_ACCUMULATED_FEES, b"");
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
        SELECTOR_CREATE_TRADE => handle_create_trade(),
        SELECTOR_LOCK_FUNDS => handle_lock_funds(),
        SELECTOR_CANCEL_TRADE => handle_cancel_trade(),
        SELECTOR_REVEAL_COORDINATES => handle_reveal_coordinates(),
        SELECTOR_SUBMIT_HEARTBEAT => handle_submit_heartbeat(),
        SELECTOR_COMPLETE_TRADE => handle_complete_trade(),
        SELECTOR_DISPUTE_TRADE => handle_dispute_trade(),
        SELECTOR_RESOLVE_DISPUTE => handle_resolve_dispute(),
        SELECTOR_GET_TRADE => handle_get_trade(),
        SELECTOR_GET_COORDINATES => handle_get_coordinates(),
        SELECTOR_GET_TRADE_STATE => handle_get_trade_state(),
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

    // Reset fees to zero
    api::set_storage(StorageFlags::empty(), &fees_key, &[0u8; 32]);

    // Transfer to owner
    let mut owner = [0u8; 20];
    api::caller(&mut owner);

    let mut fee_value = [0u8; 32];
    fee_value[..8].copy_from_slice(&total_fees.to_le_bytes());

    match api::call(
        CallFlags::empty(),
        &owner,
        u64::MAX,
        u64::MAX,
        &[u8::MAX; 32],
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

fn handle_create_trade() {
    require_not_paused();

    // createTrade(uint256 listing_id, address seller, uint256 price)
    let mut input = [0u8; 100];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256), ParamType::Address, ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let listing_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidListingId"),
    };

    let seller = match &tokens[1] {
        Token::Address(a) => {
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&a.0);
            addr
        }
        _ => revert(b"InvalidAddress"),
    };

    let price = match &tokens[2] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidPrice"),
    };

    // CRITICAL FIX: Validate inputs
    if price == 0 {
        revert(b"PriceCannotBeZero");
    }

    // Check seller is not zero address
    if seller.iter().all(|&b| b == 0) {
        revert(b"InvalidSellerAddress");
    }

    let mut caller = [0u8; 20];
    api::caller(&mut caller);

    // Check buyer != seller
    if caller.as_slice() == seller.as_slice() {
        revert(b"BuyerCannotBeSeller");
    }

    // Generate trade ID
    let trade_id = get_next_trade_id();

    // Get current timestamp
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                        timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                        timestamp_buffer[6], timestamp_buffer[7]]);

    // Store trade data: buyer(20) + seller(20) + listing_id(8) + price(8) + state(1) + created_at(8) = 65 bytes
    let mut trade_data = [0u8; 65];
    trade_data[0..20].copy_from_slice(&caller);
    trade_data[20..40].copy_from_slice(&seller);
    trade_data[40..48].copy_from_slice(&listing_id.to_le_bytes());
    trade_data[48..56].copy_from_slice(&price.to_le_bytes());
    trade_data[56] = STATE_CREATED;
    trade_data[57..65].copy_from_slice(&timestamp.to_le_bytes());

    let trade_key = trade_storage_key(trade_id);
    api::set_storage(StorageFlags::empty(), &trade_key, &trade_data);

    // Initialize coordinate stages
    let stage_key = storage_key(PREFIX_COORDINATE_STAGE, &trade_id.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &stage_key, &[0u8; 1]);

    // Emit TradeCreated event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x22; 32], topic];
    api::deposit_event(&topics, &trade_data[..48]);

    let output = encode(&[Token::Uint(U256::from(trade_id))]);
    api::return_value(ReturnFlags::empty(), &output);
}

fn handle_lock_funds() {
    require_not_paused();

    // lockFunds(uint256 trade_id) - payable
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    // Get trade
    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    // Verify caller is buyer
    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    if caller.as_slice() != &trade_data[0..20] {
        revert(ERROR_NOT_BUYER);
    }

    // Verify state is CREATED
    if trade_data[56] != STATE_CREATED {
        revert(ERROR_INVALID_STATE);
    }

    // Verify value matches price exactly (no overpayment)
    let mut value_buffer = [0u8; 32];
    api::value_transferred(&mut value_buffer);
    let value = u64::from_le_bytes([value_buffer[0], value_buffer[1], value_buffer[2], value_buffer[3],
                                     value_buffer[4], value_buffer[5], value_buffer[6], value_buffer[7]]);

    let price = u64::from_le_bytes([trade_data[48], trade_data[49], trade_data[50], trade_data[51],
                                     trade_data[52], trade_data[53], trade_data[54], trade_data[55]]);

    if value != price {
        revert(b"ExactValueRequired");
    }

    // Update state to LOCKED
    trade_data[56] = STATE_LOCKED;
    api::set_storage(StorageFlags::empty(), &trade_key, &trade_data);

    // Emit FundsLocked event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x33; 32], topic];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&value.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_cancel_trade() {
    require_not_paused();

    // cancelTrade(uint256 trade_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    // Get trade
    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    // Verify caller is buyer or seller
    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    let is_buyer = caller.as_slice() == &trade_data[0..20];
    let is_seller = caller.as_slice() == &trade_data[20..40];

    if !is_buyer && !is_seller {
        revert(ERROR_NOT_PARTY);
    }

    let current_state = trade_data[56];

    // Can only cancel in CREATED or LOCKED states
    if current_state != STATE_CREATED && current_state != STATE_LOCKED {
        revert(ERROR_INVALID_STATE);
    }

    // Update state to CANCELLED
    trade_data[56] = STATE_CANCELLED;
    api::set_storage(StorageFlags::empty(), &trade_key, &trade_data);

    // If funds were locked, refund buyer
    if current_state == STATE_LOCKED {
        let buyer = &trade_data[0..20];
        let price = u64::from_le_bytes([trade_data[48], trade_data[49], trade_data[50],
                                         trade_data[51], trade_data[52], trade_data[53],
                                         trade_data[54], trade_data[55]]);

        let mut buyer_address = [0u8; 20];
        buyer_address.copy_from_slice(buyer);

        let mut refund_value = [0u8; 32];
        refund_value[..8].copy_from_slice(&price.to_le_bytes());

        match api::call(
            CallFlags::empty(),
            &buyer_address,
            u64::MAX,          // ref_time limit
            u64::MAX,          // proof_size limit
            &[u8::MAX; 32],   // deposit limit
            &refund_value,
            &[],
            None,
        ) {
            Ok(()) => { /* Refund successful */ },
            Err(_) => {
                // If refund fails, revert to allow retry
                revert(b"RefundFailed");
            }
        }
    }

    // Emit TradeCancelled event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x88; 32], topic];
    let cancelled_by = if is_buyer { [1u8] } else { [0u8] };
    api::deposit_event(&topics, &cancelled_by);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_reveal_coordinates() {
    require_not_paused();

    // revealCoordinates(uint256 trade_id, uint8 stage, bytes coordinates)
    let input_size = api::call_data_size();
    if input_size < 296 {
        revert(b"InputTooShort");
    }

    let mut input = [0u8; 512];
    api::call_data_copy(&mut input, 0);

    // zone_id (4 bytes) + stage (4 bytes) + coordinates (up to 256 bytes)
    let trade_id = u64::from_le_bytes([input[4], input[5], input[6], input[7],
                                        input[8], input[9], input[10], input[11]]);
    let stage = input[36];

    // CRITICAL FIX: Validate stage number
    if stage >= NUM_COORDINATE_STAGES {
        revert(b"InvalidStage");
    }

    // Get trade
    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    // Verify caller is seller
    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    if caller.as_slice() != &trade_data[20..40] {
        revert(ERROR_NOT_SELLER);
    }

    // CRITICAL FIX: Only allow reveal in LOCKED state
    if trade_data[56] != STATE_LOCKED {
        revert(ERROR_INVALID_STATE);
    }

    // Store coordinates for this stage (simplified - just store fixed 256 bytes)
    let coord_key = get_coordinate_key(trade_id, stage);
    let mut coordinates = [0u8; 256];
    coordinates.copy_from_slice(&input[40..296]);
    api::set_storage(StorageFlags::empty(), &coord_key, &coordinates);

    // Update current stage
    let stage_key = storage_key(PREFIX_COORDINATE_STAGE, &trade_id.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &stage_key, &[stage]);

    // Emit CoordinatesRevealed event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x44; 32], topic];
    api::deposit_event(&topics, &[stage]);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_submit_heartbeat() {
    require_not_paused();

    // submitHeartbeat(uint256 trade_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    // Get current timestamp
    let mut timestamp_buffer = [0u8; 32];
    api::now(&mut timestamp_buffer);
    let timestamp = u64::from_le_bytes([timestamp_buffer[0], timestamp_buffer[1], timestamp_buffer[2],
                                        timestamp_buffer[3], timestamp_buffer[4], timestamp_buffer[5],
                                        timestamp_buffer[6], timestamp_buffer[7]]);

    // Store heartbeat
    let heartbeat_key = storage_key(PREFIX_HEARTBEAT, &trade_id.to_le_bytes());
    let mut heartbeat_bytes = [0u8; 32];
    heartbeat_bytes[..8].copy_from_slice(&timestamp.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &heartbeat_key, &heartbeat_bytes);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_complete_trade() {
    require_not_paused();

    // completeTrade(uint256 trade_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    // Get trade
    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    // Verify caller is buyer
    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    if caller.as_slice() != &trade_data[0..20] {
        revert(ERROR_NOT_BUYER);
    }

    // Verify state is COORDINATES_REVEALED or LOCKED
    if trade_data[56] != STATE_LOCKED && trade_data[56] != STATE_COORDINATES_REVEALED {
        revert(ERROR_INVALID_STATE);
    }

    // Update state to COMPLETED
    trade_data[56] = STATE_COMPLETED;
    api::set_storage(StorageFlags::empty(), &trade_key, &trade_data);

    // Release funds to seller (minus fee)
    let price = u64::from_le_bytes([trade_data[48], trade_data[49], trade_data[50], trade_data[51],
                                     trade_data[52], trade_data[53], trade_data[54], trade_data[55]]);

    let fee = match safe_percentage(price, FEE_BASIS_POINTS) {
        Ok(f) => f,
        Err(e) => revert(e.as_bytes()),
    };

    let seller_amount = match safe_sub(price, fee) {
        Ok(a) => a,
        Err(e) => revert(e.as_bytes()),
    };

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

    // Transfer funds to seller
    let seller = &trade_data[20..40];
    let mut seller_address = [0u8; 20];
    seller_address.copy_from_slice(seller);

    let mut transfer_value = [0u8; 32];
    transfer_value[..8].copy_from_slice(&seller_amount.to_le_bytes());

    match api::call(
        CallFlags::empty(),
        &seller_address,
        u64::MAX,              // ref_time limit
        u64::MAX,              // proof_size limit
        &[u8::MAX; 32],       // deposit limit
        &transfer_value,       // Send value
        &[],                   // No call data (plain transfer)
        None,
    ) {
        Ok(()) => { /* Transfer successful */ },
        Err(_) => revert(b"TransferFailed"),
    }

    // Emit TradeCompleted event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x55; 32], topic];
    let mut event_data = [0u8; 8];
    event_data.copy_from_slice(&seller_amount.to_le_bytes());
    api::deposit_event(&topics, &event_data);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_dispute_trade() {
    require_not_paused();

    // disputeTrade(uint256 trade_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    // Get trade
    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    // CRITICAL FIX: Verify caller is buyer or seller
    let mut caller = [0u8; 20];
    api::caller(&mut caller);
    let is_buyer = caller.as_slice() == &trade_data[0..20];
    let is_seller = caller.as_slice() == &trade_data[20..40];

    if !is_buyer && !is_seller {
        revert(ERROR_NOT_PARTY);
    }

    // CRITICAL FIX: Only allow disputes in valid states
    let current_state = trade_data[56];
    if current_state != STATE_LOCKED && current_state != STATE_COORDINATES_REVEALED {
        revert(ERROR_INVALID_STATE);
    }

    // Update state to DISPUTED
    trade_data[56] = STATE_DISPUTED;
    api::set_storage(StorageFlags::empty(), &trade_key, &trade_data);

    // Emit TradeDisputed event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x66; 32], topic];
    api::deposit_event(&topics, &[]);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

fn handle_resolve_dispute() {
    require_owner();

    // resolveDispute(uint256 trade_id, bool favor_buyer)
    let mut input = [0u8; 68];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256), ParamType::Bool], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    let favor_buyer = match &tokens[1] {
        Token::Bool(b) => *b,
        _ => false,
    };

    // Get trade
    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    // CRITICAL FIX: Verify trade is actually disputed
    if trade_data[56] != STATE_DISPUTED {
        revert(ERROR_INVALID_STATE);
    }

    // Mark as completed
    trade_data[56] = STATE_COMPLETED;
    api::set_storage(StorageFlags::empty(), &trade_key, &trade_data);

    // Get price from trade data
    let price = u64::from_le_bytes([trade_data[48], trade_data[49], trade_data[50], trade_data[51],
                                     trade_data[52], trade_data[53], trade_data[54], trade_data[55]]);

    // Determine recipient based on dispute resolution
    let recipient = if favor_buyer {
        // Refund buyer (full price, no fee)
        &trade_data[0..20]
    } else {
        // Pay seller (price minus fee)
        &trade_data[20..40]
    };

    let (amount, fee_amount) = if favor_buyer {
        (price, 0u64)  // Buyer gets full refund, no fee
    } else {
        // Seller gets price minus fee
        let fee = match safe_percentage(price, FEE_BASIS_POINTS) {
            Ok(f) => f,
            Err(e) => revert(e.as_bytes()),
        };
        let amt = match safe_sub(price, fee) {
            Ok(a) => a,
            Err(e) => revert(e.as_bytes()),
        };
        (amt, fee)
    };

    // Track fees if seller wins
    if fee_amount > 0 {
        let fees_key = storage_key(PREFIX_ACCUMULATED_FEES, b"");
        let mut fees_bytes = [0u8; 32];
        let _ = api::get_storage(StorageFlags::empty(), &fees_key, &mut &mut fees_bytes[..]);
        let current_fees = u64::from_le_bytes([fees_bytes[0], fees_bytes[1], fees_bytes[2], fees_bytes[3],
                                                fees_bytes[4], fees_bytes[5], fees_bytes[6], fees_bytes[7]]);
        let new_fees = match safe_add(current_fees, fee_amount) {
            Ok(f) => f,
            Err(e) => revert(e.as_bytes()),
        };
        fees_bytes[..8].copy_from_slice(&new_fees.to_le_bytes());
        api::set_storage(StorageFlags::empty(), &fees_key, &fees_bytes);
    }

    // Transfer funds to winner
    let mut recipient_address = [0u8; 20];
    recipient_address.copy_from_slice(recipient);

    let mut transfer_value = [0u8; 32];
    transfer_value[..8].copy_from_slice(&amount.to_le_bytes());

    match api::call(
        CallFlags::empty(),
        &recipient_address,
        u64::MAX,              // ref_time limit
        u64::MAX,              // proof_size limit
        &[u8::MAX; 32],       // deposit limit
        &transfer_value,
        &[],
        None,
    ) {
        Ok(()) => { /* Transfer successful */ },
        Err(_) => revert(b"TransferFailed"),
    }

    // Emit DisputeResolved event
    let mut topic = [0u8; 32];
    topic[..8].copy_from_slice(&trade_id.to_le_bytes());
    let topics = [[0x77; 32], topic];
    let result = if favor_buyer { [1u8] } else { [0u8] };
    api::deposit_event(&topics, &result);

    api::return_value(ReturnFlags::empty(), &[1u8]);
}

// ============================================================================
// View Functions
// ============================================================================

fn handle_get_trade() {
    // getTrade(uint256 trade_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    api::return_value(ReturnFlags::empty(), &trade_data);
}

fn handle_get_coordinates() {
    // getCoordinates(uint256 trade_id, uint8 stage)
    let mut input = [0u8; 68];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256), ParamType::Uint(8)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    let stage = match &tokens[1] {
        Token::Uint(v) => v.as_u64() as u8,
        _ => 0,
    };

    let coord_key = get_coordinate_key(trade_id, stage);
    let mut coordinates = [0u8; 256];
    let _ = api::get_storage(StorageFlags::empty(), &coord_key, &mut &mut coordinates[..]);

    api::return_value(ReturnFlags::empty(), &coordinates);
}

fn handle_get_trade_state() {
    // getTradeState(uint256 trade_id)
    let mut input = [0u8; 36];
    api::call_data_copy(&mut input, 0);

    let tokens = match decode(&[ParamType::Uint(256)], &input[4..]) {
        Ok(t) => t,
        Err(_) => revert(b"DecodeFailed"),
    };

    let trade_id = match &tokens[0] {
        Token::Uint(v) => v.as_u64(),
        _ => revert(b"InvalidTradeId"),
    };

    let trade_key = trade_storage_key(trade_id);
    let mut trade_data = [0u8; 65];
    if api::get_storage(StorageFlags::empty(), &trade_key, &mut &mut trade_data[..]).is_err() {
        revert(ERROR_INVALID_TRADE);
    }

    let state = trade_data[56];
    let output = encode(&[Token::Uint(U256::from(state))]);
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

fn get_next_trade_id() -> u64 {
    let count_key = storage_key(PREFIX_TRADE_COUNT, b"");
    let mut count_bytes = [0u8; 32];
    let _ = api::get_storage(StorageFlags::empty(), &count_key, &mut &mut count_bytes[..]);
    let count = u64::from_le_bytes([count_bytes[0], count_bytes[1], count_bytes[2], count_bytes[3],
                                     count_bytes[4], count_bytes[5], count_bytes[6], count_bytes[7]]);

    // CRITICAL FIX: Check for overflow
    if count == u64::MAX {
        revert(b"MaxTradesReached");
    }

    let new_count = count + 1;
    let mut new_count_bytes = [0u8; 32];
    new_count_bytes[..8].copy_from_slice(&new_count.to_le_bytes());
    api::set_storage(StorageFlags::empty(), &count_key, &new_count_bytes);
    new_count
}

fn trade_storage_key(trade_id: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = PREFIX_TRADE_DATA;
    key[1..9].copy_from_slice(&trade_id.to_le_bytes());
    key
}

fn get_coordinate_key(trade_id: u64, stage: u8) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = PREFIX_COORDINATE_STAGE;
    key[1..9].copy_from_slice(&trade_id.to_le_bytes());
    key[9] = stage;
    key
}

fn revert(error: &[u8]) -> ! {
    api::return_value(ReturnFlags::REVERT, error);
    unsafe {
        core::arch::asm!("unimp");
        core::hint::unreachable_unchecked();
    }
}
