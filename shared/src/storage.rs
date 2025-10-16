//! Storage key generation helpers
//! Provides consistent key generation patterns across contracts

use uapi::{HostFn, HostFnImpl as api};

/// Generate a storage key with a prefix and suffix
pub fn storage_key(prefix: u8, suffix: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = prefix;
    let copy_len = suffix.len().min(31);
    key[1..1 + copy_len].copy_from_slice(&suffix[..copy_len]);
    key
}

/// Build a composite key from multiple components
pub fn build_key(components: &[&[u8]]) -> [u8; 32] {
    let mut data = [0u8; 512];
    let mut offset = 0;

    for component in components {
        let len = component.len().min(512 - offset);
        data[offset..offset + len].copy_from_slice(&component[..len]);
        offset += len;
        if offset >= 512 {
            break;
        }
    }

    let mut key = [0u8; 32];
    api::hash_keccak_256(&data[..offset], &mut key);
    key
}

/// Generate a key for a mapping: prefix + key_bytes
pub fn mapping_key(prefix: u8, key: &[u8; 32]) -> [u8; 32] {
    let mut result = [0u8; 32];
    result[0] = prefix;
    let mut temp = [0u8; 33];
    temp[0] = prefix;
    temp[1..33].copy_from_slice(key);
    api::hash_keccak_256(&temp, &mut result);
    result
}

/// Generate a key for a double mapping: prefix + key1 + key2
pub fn double_mapping_key(prefix: u8, key1: &[u8; 32], key2: &[u8; 32]) -> [u8; 32] {
    let mut data = [0u8; 65];
    data[0] = prefix;
    data[1..33].copy_from_slice(key1);
    data[33..65].copy_from_slice(key2);

    let mut result = [0u8; 32];
    api::hash_keccak_256(&data, &mut result);
    result
}

/// Generate a key for an address -> value mapping
pub fn address_key(prefix: u8, address: &[u8; 20]) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = prefix;
    key[1..21].copy_from_slice(address);
    key
}

/// Generate a key for an address -> u64 -> value mapping
pub fn address_u64_key(prefix: u8, address: &[u8; 20], id: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = prefix;
    key[1..21].copy_from_slice(address);
    key[21..29].copy_from_slice(&id.to_le_bytes());
    key
}

/// Generate a key for a list element: prefix + index
pub fn list_key(prefix: u8, index: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = prefix;
    key[1..9].copy_from_slice(&index.to_le_bytes());
    key
}

/// Generate a key for zone + timestamp combination
pub fn zone_time_key(prefix: u8, zone_id: u32, timestamp: u64) -> [u8; 32] {
    let mut key = [0u8; 32];
    key[0] = prefix;
    key[1..5].copy_from_slice(&zone_id.to_le_bytes());
    key[5..13].copy_from_slice(&timestamp.to_le_bytes());
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_storage_key() {
        let key = storage_key(1, b"test");
        assert_eq!(key[0], 1);
        assert_eq!(&key[1..5], b"test");
    }

    #[test]
    fn test_address_key() {
        let addr = [0x42u8; 20];
        let key = address_key(5, &addr);
        assert_eq!(key[0], 5);
        assert_eq!(&key[1..21], &addr);
    }

    #[test]
    fn test_list_key() {
        let key = list_key(3, 42);
        assert_eq!(key[0], 3);
        assert_eq!(u64::from_le_bytes([key[1], key[2], key[3], key[4], key[5], key[6], key[7], key[8]]), 42);
    }
}
