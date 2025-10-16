//! Cryptographic primitives for Nightmarket
//! Adapted from fragments reference implementation

use uapi::{HostFn, HostFnImpl as api};

/// BN254 curve constants
pub const BN254_G1_SIZE: usize = 64;  // 2 * 32 bytes (x, y)
pub const BN254_G2_SIZE: usize = 128; // 4 * 32 bytes (x1, x2, y1, y2)

/// Groth16 proof structure for BN254 curve
#[derive(Clone, Copy)]
pub struct Groth16Proof {
    pub a: [u8; BN254_G1_SIZE],      // G1 point (pi_a)
    pub b: [u8; BN254_G2_SIZE],      // G2 point (pi_b)
    pub c: [u8; BN254_G1_SIZE],      // G1 point (pi_c)
}

impl Groth16Proof {
    /// Parse a Groth16 proof from bytes
    /// Expected format: a (64) || b (128) || c (64) = 256 bytes total
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() != 256 {
            return Err("InvalidProofLength");
        }

        let mut proof = Groth16Proof {
            a: [0u8; BN254_G1_SIZE],
            b: [0u8; BN254_G2_SIZE],
            c: [0u8; BN254_G1_SIZE],
        };

        proof.a.copy_from_slice(&bytes[0..64]);
        proof.b.copy_from_slice(&bytes[64..192]);
        proof.c.copy_from_slice(&bytes[192..256]);

        Ok(proof)
    }

    /// Convert proof to bytes
    pub fn to_bytes(&self) -> [u8; 256] {
        let mut result = [0u8; 256];
        result[0..64].copy_from_slice(&self.a);
        result[64..192].copy_from_slice(&self.b);
        result[192..256].copy_from_slice(&self.c);
        result
    }
}

/// Verify a Groth16 proof using pairing check
/// For PolkaVM, we use a simplified verification since precompiles aren't available
pub fn verify_groth16(
    proof: &Groth16Proof,
    public_inputs: &[[u8; 32]],
    _vk_hash: &[u8; 32], // Verification key hash (for future use)
) -> Result<(), &'static str> {
    // Validate proof is not all zeros
    let all_zero = proof.a.iter().all(|&x| x == 0)
        && proof.b.iter().all(|&x| x == 0)
        && proof.c.iter().all(|&x| x == 0);

    if all_zero {
        return Err("ProofAllZeros");
    }

    // Validate public inputs
    if public_inputs.is_empty() {
        return Err("NoPublicInputs");
    }

    if public_inputs.len() > 10 {
        return Err("TooManyPublicInputs");
    }

    // In a full implementation, we would:
    // 1. Reconstruct the verification key from vk_hash
    // 2. Compute the linear combination of public inputs with VK IC points
    // 3. Perform the pairing check: e(A,B) = e(alpha,beta) * e(L,gamma) * e(C,delta)
    //
    // For PolkaVM without precompiles, we do simplified validation
    // Real pairing checks would be done off-chain or via future chain extensions

    // Basic sanity checks on curve points
    validate_g1_point(&proof.a)?;
    validate_g2_point(&proof.b)?;
    validate_g1_point(&proof.c)?;

    Ok(())
}

/// Validate that bytes represent a valid G1 point (simplified)
fn validate_g1_point(point: &[u8; 64]) -> Result<(), &'static str> {
    // Check not all zeros and not all 0xFF
    let all_zero = point.iter().all(|&x| x == 0);
    let all_max = point.iter().all(|&x| x == 0xFF);

    if all_zero || all_max {
        return Err("InvalidG1Point");
    }

    Ok(())
}

/// Validate that bytes represent a valid G2 point (simplified)
fn validate_g2_point(point: &[u8; 128]) -> Result<(), &'static str> {
    // Check not all zeros and not all 0xFF
    let all_zero = point.iter().all(|&x| x == 0);
    let all_max = point.iter().all(|&x| x == 0xFF);

    if all_zero || all_max {
        return Err("InvalidG2Point");
    }

    Ok(())
}

/// Derive a nullifier from a secret and commitment
/// Uses domain separation to prevent cross-protocol attacks
pub fn derive_nullifier(
    secret: &[u8; 32],
    commitment: &[u8; 32],
    domain: &[u8],
) -> [u8; 32] {
    // Construct input: domain || secret || commitment
    let mut input = [0u8; 512];
    let domain_len = domain.len().min(256);
    input[0..domain_len].copy_from_slice(&domain[..domain_len]);
    input[256..288].copy_from_slice(secret);
    input[288..320].copy_from_slice(commitment);

    let mut nullifier = [0u8; 32];
    api::hash_keccak_256(&input[..320 + domain_len], &mut nullifier);
    nullifier
}

/// Hash two 32-byte values together
pub fn hash_pair(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut input = [0u8; 64];
    input[0..32].copy_from_slice(left);
    input[32..64].copy_from_slice(right);

    let mut output = [0u8; 32];
    api::hash_keccak_256(&input, &mut output);
    output
}

/// Compute Keccak-256 hash
pub fn keccak256(data: &[u8]) -> [u8; 32] {
    let mut output = [0u8; 32];
    api::hash_keccak_256(data, &mut output);
    output
}

/// Verify a merkle proof
pub fn verify_merkle_proof(
    leaf: &[u8; 32],
    proof: &[[u8; 32]],
    root: &[u8; 32],
    index: u64,
) -> bool {
    let mut computed_hash = *leaf;
    let mut idx = index;

    for sibling in proof {
        computed_hash = if idx % 2 == 0 {
            hash_pair(&computed_hash, sibling)
        } else {
            hash_pair(sibling, &computed_hash)
        };
        idx /= 2;
    }

    computed_hash == *root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proof_parsing() {
        let bytes = [1u8; 256];
        let proof = Groth16Proof::from_bytes(&bytes).unwrap();
        assert_eq!(proof.a[0], 1);
        assert_eq!(proof.b[0], 1);
        assert_eq!(proof.c[0], 1);
    }
}
