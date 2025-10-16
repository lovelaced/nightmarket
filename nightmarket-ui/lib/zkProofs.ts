/**
 * Zero-Knowledge Proof Generation Library
 *
 * Provides client-side ZK proof generation for:
 * - Location proofs (prove presence in zone)
 * - Mixer withdrawals (anonymous fund retrieval)
 * - Reputation threshold (prove score >= X)
 */

// @ts-ignore - snarkjs doesn't have official TypeScript types
import { groth16 } from 'snarkjs';
import { geolocation, type Coordinates, type ZoneBounds } from './geolocation';
import { ethers } from 'ethers';

export interface LocationProofData {
  proof: Uint8Array;      // 256 bytes - formatted Groth16 proof
  nullifier: Uint8Array;  // 32 bytes
  zoneId: number;
  timestamp: number;
}

export interface MixerProofData {
  proof: Uint8Array;
  nullifier: Uint8Array;
  commitment: Uint8Array;
}

export interface ReputationProofData {
  proof: Uint8Array;
  ephemeralId: Uint8Array;
}

/**
 * Location Proof Generator
 */
export class LocationProofGenerator {
  /**
   * Generate a location proof for the given zone
   *
   * @param zoneId - Zone identifier
   * @param zoneBounds - Zone boundary coordinates
   * @returns Location proof data ready for contract submission
   */
  async generate(zoneId: number, zoneBounds: ZoneBounds): Promise<LocationProofData> {
    try {
      // 1. Get user's current location
      const location = await geolocation.getLocation();

      // 2. Verify user is actually in the zone (client-side check)
      if (!geolocation.isInZone(location, zoneBounds)) {
        throw new Error(
          `You are not in Zone ${zoneId}. ` +
          `Current location: (${location.lat.toFixed(4)}, ${location.lon.toFixed(4)})`
        );
      }

      // 3. Collect signal data
      const signals = await geolocation.collectSignals();

      if (signals.length < 8) {
        throw new Error('Insufficient signal data. Need at least 8 signals.');
      }

      // 4. Generate random secret
      const secret = this.generateSecret();

      // 5. Get current timestamp
      const timestamp = BigInt(Math.floor(Date.now() / 1000));

      // 6. Format coordinates for circuit (scale by 1e6)
      const scaledLocation = geolocation.formatCoordinatesForCircuit(location);
      const scaledBounds = {
        latMin: Math.round(zoneBounds.latMin * 1e6),
        latMax: Math.round(zoneBounds.latMax * 1e6),
        lonMin: Math.round(zoneBounds.lonMin * 1e6),
        lonMax: Math.round(zoneBounds.lonMax * 1e6),
      };

      // 7. Prepare circuit inputs
      const input = {
        // Private inputs
        secret: secret.toString(),
        location_lat: scaledLocation.lat.toString(),
        location_lon: scaledLocation.lon.toString(),
        signal_hashes: signals.map(s => s.toString()),
        zone_lat_min: scaledBounds.latMin.toString(),
        zone_lat_max: scaledBounds.latMax.toString(),
        zone_lon_min: scaledBounds.lonMin.toString(),
        zone_lon_max: scaledBounds.lonMax.toString(),

        // Public inputs
        zone_id: zoneId.toString(),
        timestamp: timestamp.toString(),
      };

      // 8. Generate ZK proof
      const { proof, publicSignals } = await groth16.fullProve(
        input,
        '/circuits/location_proof.wasm',
        '/circuits/location_proof.zkey'
      );

      // 9. Format proof for contract
      const proofBytes = this.formatProofForContract(proof);

      // 10. Extract nullifier from public signals
      // Public signals order: [zone_id, timestamp, nullifier]
      const nullifier = publicSignals[2];

      return {
        proof: proofBytes,
        nullifier: this.hexToBytes32(nullifier),
        zoneId,
        timestamp: Number(timestamp),
      };
    } catch (error: any) {
      if (error.message?.includes('not in zone') || error.message?.includes('not in Zone')) {
        throw error; // Re-throw with original message
      }
      throw new Error(`Location proof generation failed: ${error.message}`);
    }
  }

  /**
   * Generate a random secret for proof generation
   */
  private generateSecret(): bigint {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    return BigInt('0x' + Array.from(bytes)
      .map(b => b.toString(16).padStart(2, '0'))
      .join(''));
  }

  /**
   * Format Groth16 proof for contract consumption (256 bytes)
   *
   * Layout:
   * - A (G1 point): 64 bytes (2 * 32)
   * - B (G2 point): 128 bytes (4 * 32)
   * - C (G1 point): 64 bytes (2 * 32)
   */
  private formatProofForContract(proof: any): Uint8Array {
    const proofBytes = new Uint8Array(256);

    // A (G1 point): 64 bytes
    const aX = BigInt(proof.pi_a[0]).toString(16).padStart(64, '0');
    const aY = BigInt(proof.pi_a[1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(aX), 0);
    proofBytes.set(this.hexToBytes(aY), 32);

    // B (G2 point): 128 bytes (2 field elements, each 64 bytes)
    const bX0 = BigInt(proof.pi_b[0][0]).toString(16).padStart(64, '0');
    const bX1 = BigInt(proof.pi_b[0][1]).toString(16).padStart(64, '0');
    const bY0 = BigInt(proof.pi_b[1][0]).toString(16).padStart(64, '0');
    const bY1 = BigInt(proof.pi_b[1][1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(bX0), 64);
    proofBytes.set(this.hexToBytes(bX1), 96);
    proofBytes.set(this.hexToBytes(bY0), 128);
    proofBytes.set(this.hexToBytes(bY1), 160);

    // C (G1 point): 64 bytes
    const cX = BigInt(proof.pi_c[0]).toString(16).padStart(64, '0');
    const cY = BigInt(proof.pi_c[1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(cX), 192);
    proofBytes.set(this.hexToBytes(cY), 224);

    return proofBytes;
  }

  private hexToBytes(hex: string): Uint8Array {
    const matches = hex.match(/.{1,2}/g);
    if (!matches) return new Uint8Array(32);
    return new Uint8Array(matches.map(byte => parseInt(byte, 16)));
  }

  private hexToBytes32(bigintStr: string): Uint8Array {
    const hex = BigInt(bigintStr).toString(16).padStart(64, '0');
    return this.hexToBytes(hex);
  }
}

/**
 * Mixer Withdrawal Proof Generator
 */
export class MixerProofGenerator {
  /**
   * Generate a mixer withdrawal proof
   *
   * @param secret - Secret from original deposit
   * @param zoneId - Zone where funds were deposited
   * @returns Mixer proof data
   */
  async generate(secret: bigint, zoneId: number): Promise<MixerProofData> {
    const input = {
      // Private input
      secret: secret.toString(),

      // Public input
      zone_id: zoneId.toString(),
    };

    const { proof, publicSignals } = await groth16.fullProve(
      input,
      '/circuits/mixer_withdrawal.wasm',
      '/circuits/mixer_withdrawal.zkey'
    );

    const proofBytes = this.formatProofForContract(proof);

    // Public signals: [zone_id, nullifier, commitment]
    const nullifier = publicSignals[1];
    const commitment = publicSignals[2];

    return {
      proof: proofBytes,
      nullifier: this.hexToBytes32(nullifier),
      commitment: this.hexToBytes32(commitment),
    };
  }

  private formatProofForContract(proof: any): Uint8Array {
    // Same as LocationProofGenerator
    const proofBytes = new Uint8Array(256);

    const aX = BigInt(proof.pi_a[0]).toString(16).padStart(64, '0');
    const aY = BigInt(proof.pi_a[1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(aX), 0);
    proofBytes.set(this.hexToBytes(aY), 32);

    const bX0 = BigInt(proof.pi_b[0][0]).toString(16).padStart(64, '0');
    const bX1 = BigInt(proof.pi_b[0][1]).toString(16).padStart(64, '0');
    const bY0 = BigInt(proof.pi_b[1][0]).toString(16).padStart(64, '0');
    const bY1 = BigInt(proof.pi_b[1][1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(bX0), 64);
    proofBytes.set(this.hexToBytes(bX1), 96);
    proofBytes.set(this.hexToBytes(bY0), 128);
    proofBytes.set(this.hexToBytes(bY1), 160);

    const cX = BigInt(proof.pi_c[0]).toString(16).padStart(64, '0');
    const cY = BigInt(proof.pi_c[1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(cX), 192);
    proofBytes.set(this.hexToBytes(cY), 224);

    return proofBytes;
  }

  private hexToBytes(hex: string): Uint8Array {
    const matches = hex.match(/.{1,2}/g);
    if (!matches) return new Uint8Array(32);
    return new Uint8Array(matches.map(byte => parseInt(byte, 16)));
  }

  private hexToBytes32(bigintStr: string): Uint8Array {
    const hex = BigInt(bigintStr).toString(16).padStart(64, '0');
    return this.hexToBytes(hex);
  }
}

/**
 * Reputation Threshold Proof Generator
 */
export class ReputationProofGenerator {
  /**
   * Generate a reputation threshold proof
   *
   * @param secret - User's reputation secret
   * @param score - Actual score (kept private)
   * @param threshold - Required minimum score
   * @param zoneId - Zone identifier
   * @param ephemeralId - Pre-computed ephemeral ID
   * @returns Reputation proof data
   */
  async generate(
    secret: bigint,
    score: number,
    threshold: number,
    zoneId: number,
    ephemeralId: bigint
  ): Promise<ReputationProofData> {
    if (score < threshold) {
      throw new Error(`Score ${score} is below threshold ${threshold}`);
    }

    const input = {
      // Private inputs
      secret: secret.toString(),
      score: score.toString(),

      // Public inputs
      zone_id: zoneId.toString(),
      ephemeral_id: ephemeralId.toString(),
      threshold: threshold.toString(),
    };

    const { proof, publicSignals } = await groth16.fullProve(
      input,
      '/circuits/reputation_threshold.wasm',
      '/circuits/reputation_threshold.zkey'
    );

    const proofBytes = this.formatProofForContract(proof);

    return {
      proof: proofBytes,
      ephemeralId: this.hexToBytes32(ephemeralId.toString()),
    };
  }

  private formatProofForContract(proof: any): Uint8Array {
    const proofBytes = new Uint8Array(256);

    const aX = BigInt(proof.pi_a[0]).toString(16).padStart(64, '0');
    const aY = BigInt(proof.pi_a[1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(aX), 0);
    proofBytes.set(this.hexToBytes(aY), 32);

    const bX0 = BigInt(proof.pi_b[0][0]).toString(16).padStart(64, '0');
    const bX1 = BigInt(proof.pi_b[0][1]).toString(16).padStart(64, '0');
    const bY0 = BigInt(proof.pi_b[1][0]).toString(16).padStart(64, '0');
    const bY1 = BigInt(proof.pi_b[1][1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(bX0), 64);
    proofBytes.set(this.hexToBytes(bX1), 96);
    proofBytes.set(this.hexToBytes(bY0), 128);
    proofBytes.set(this.hexToBytes(bY1), 160);

    const cX = BigInt(proof.pi_c[0]).toString(16).padStart(64, '0');
    const cY = BigInt(proof.pi_c[1]).toString(16).padStart(64, '0');
    proofBytes.set(this.hexToBytes(cX), 192);
    proofBytes.set(this.hexToBytes(cY), 224);

    return proofBytes;
  }

  private hexToBytes(hex: string): Uint8Array {
    const matches = hex.match(/.{1,2}/g);
    if (!matches) return new Uint8Array(32);
    return new Uint8Array(matches.map(byte => parseInt(byte, 16)));
  }

  private hexToBytes32(bigintStr: string): Uint8Array {
    const hex = BigInt(bigintStr).toString(16).padStart(64, '0');
    return this.hexToBytes(hex);
  }
}

/**
 * Utility functions
 */
export class ProofUtils {
  /**
   * Generate a cryptographically secure random secret
   */
  static generateSecret(): bigint {
    const bytes = crypto.getRandomValues(new Uint8Array(32));
    return BigInt('0x' + Array.from(bytes)
      .map(b => b.toString(16).padStart(2, '0'))
      .join(''));
  }

  /**
   * Convert bigint to 32-byte array
   */
  static bigintToBytes32(value: bigint): Uint8Array {
    const hex = value.toString(16).padStart(64, '0');
    const matches = hex.match(/.{1,2}/g);
    if (!matches) return new Uint8Array(32);
    return new Uint8Array(matches.map(byte => parseInt(byte, 16)));
  }

  /**
   * Convert 32-byte array to bigint
   */
  static bytes32ToBigint(bytes: Uint8Array): bigint {
    const hex = Array.from(bytes)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');
    return BigInt('0x' + hex);
  }

  /**
   * Verify proof locally before submitting to contract (optional)
   */
  static async verifyProofLocally(
    proof: any,
    publicSignals: string[],
    vkeyPath: string
  ): Promise<boolean> {
    try {
      const response = await fetch(vkeyPath);
      const vkey = await response.json();
      return await groth16.verify(vkey, publicSignals, proof);
    } catch (error) {
      console.error('Local verification failed:', error);
      return false;
    }
  }
}

// Export singleton instances
export const locationProofGenerator = new LocationProofGenerator();
export const mixerProofGenerator = new MixerProofGenerator();
export const reputationProofGenerator = new ReputationProofGenerator();
