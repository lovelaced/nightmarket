/**
 * Listing Encryption Utilities
 *
 * Implements AES-256-GCM encryption for marketplace listings.
 * Encryption keys are derived from zone_id + date, so only users with
 * valid location proofs for that zone can decrypt.
 */

export interface ListingData {
  title: string;
  description: string;
  instructions: string;
}

export class ListingEncryption {
  private static readonly ALGORITHM = 'AES-GCM';
  private static readonly KEY_LENGTH = 256;
  private static readonly IV_LENGTH = 12; // 96 bits for GCM
  private static readonly TAG_LENGTH = 128; // 128 bits authentication tag

  /**
   * Derive encryption key from zone_id and date
   * Same zone + same date = same key (so users in zone can decrypt)
   *
   * @param zoneId - Zone identifier
   * @param date - Date for key derivation (defaults to today)
   * @returns CryptoKey for AES-GCM
   */
  private static async deriveKey(zoneId: number, date?: Date): Promise<CryptoKey> {
    const targetDate = date || new Date();

    // Use date in YYYY-MM-DD format (UTC)
    const dateStr = targetDate.toISOString().split('T')[0];

    // Derive key from: zone_id || date || static salt
    // In production, fetch salt from contract or use contract address
    const staticSalt = 'nightmarket-v1-encryption-salt';
    const keyMaterial = `zone-${zoneId}-${dateStr}-${staticSalt}`;

    // Convert string to key material
    const encoder = new TextEncoder();
    const keyData = encoder.encode(keyMaterial);

    // Hash to get uniform key material
    const hashBuffer = await crypto.subtle.digest('SHA-256', keyData);

    // Import as AES key
    return crypto.subtle.importKey(
      'raw',
      hashBuffer,
      { name: this.ALGORITHM, length: this.KEY_LENGTH },
      false,
      ['encrypt', 'decrypt']
    );
  }

  /**
   * Encrypt listing data
   *
   * @param data - Listing data (title, description, instructions)
   * @param zoneId - Zone where listing is posted
   * @returns Encrypted data (256 bytes fixed)
   */
  static async encrypt(data: ListingData, zoneId: number): Promise<Uint8Array> {
    try {
      // 1. Derive key from zone + date
      const key = await this.deriveKey(zoneId);

      // 2. Generate random IV (12 bytes for GCM)
      const iv = crypto.getRandomValues(new Uint8Array(this.IV_LENGTH));

      // 3. Convert data to bytes
      const plaintext = JSON.stringify(data);
      const plaintextBytes = new TextEncoder().encode(plaintext);

      // 4. Encrypt with AES-256-GCM
      const ciphertext = await crypto.subtle.encrypt(
        {
          name: this.ALGORITHM,
          iv,
          tagLength: this.TAG_LENGTH,
        },
        key,
        plaintextBytes
      );

      // 5. Combine IV + ciphertext into fixed 256-byte array
      // Layout: IV(12) + ciphertext(up to 244)
      const result = new Uint8Array(256);
      result.set(iv, 0);
      result.set(new Uint8Array(ciphertext).slice(0, 244), 12);

      return result;
    } catch (error) {
      console.error('Encryption failed:', error);
      throw new Error('Failed to encrypt listing data');
    }
  }

  /**
   * Decrypt listing data
   *
   * @param encryptedBytes - Encrypted data (256 bytes)
   * @param zoneId - Zone where listing is posted
   * @param date - Date for key derivation (defaults to today)
   * @returns Decrypted listing data
   */
  static async decrypt(
    encryptedBytes: Uint8Array | string,
    zoneId: number,
    date?: Date
  ): Promise<ListingData> {
    try {
      // Convert hex string to bytes if needed
      let bytes: Uint8Array;
      if (typeof encryptedBytes === 'string') {
        const hex = encryptedBytes.startsWith('0x') ? encryptedBytes.slice(2) : encryptedBytes;
        bytes = new Uint8Array(
          hex.match(/.{1,2}/g)!.map(byte => parseInt(byte, 16))
        );
      } else {
        bytes = encryptedBytes;
      }

      // 1. Derive same key
      const key = await this.deriveKey(zoneId, date);

      // 2. Extract IV and ciphertext
      const iv = bytes.slice(0, this.IV_LENGTH);
      const ciphertext = bytes.slice(this.IV_LENGTH);

      // Find actual end of ciphertext (before padding zeros)
      let ciphertextEnd = ciphertext.length;
      for (let i = ciphertext.length - 1; i >= 0; i--) {
        if (ciphertext[i] !== 0) {
          ciphertextEnd = i + 1;
          break;
        }
      }

      const actualCiphertext = ciphertext.slice(0, ciphertextEnd);

      // 3. Decrypt with AES-256-GCM
      const decryptedBuffer = await crypto.subtle.decrypt(
        {
          name: this.ALGORITHM,
          iv,
          tagLength: this.TAG_LENGTH,
        },
        key,
        actualCiphertext
      );

      // 4. Convert bytes to string
      const decryptedText = new TextDecoder().decode(decryptedBuffer);

      // 5. Parse JSON
      return JSON.parse(decryptedText);
    } catch (error) {
      console.error('Decryption failed:', error);
      throw new Error('Failed to decrypt listing. You may not have access to this zone.');
    }
  }

  /**
   * Check if user can decrypt (has valid proof for zone)
   * In production: Check contract for hasValidProof(address, zone_id)
   *
   * @param userAddress - User's wallet address
   * @param zoneId - Zone to check
   * @returns true if user can decrypt listings in this zone
   */
  static async canDecrypt(userAddress: string, zoneId: number): Promise<boolean> {
    // TODO: Call contract to check if user has valid location proof for zone
    // For now, return true (Phase 1 simplified)
    return true;
  }

  /**
   * Decrypt if user has access, otherwise return placeholder
   *
   * @param encryptedBytes - Encrypted data
   * @param zoneId - Zone ID
   * @param userAddress - User's address
   * @returns Decrypted data or placeholder if no access
   */
  static async decryptWithAccess(
    encryptedBytes: Uint8Array | string,
    zoneId: number,
    userAddress: string
  ): Promise<ListingData> {
    const hasAccess = await this.canDecrypt(userAddress, zoneId);

    if (!hasAccess) {
      return {
        title: '[ENCRYPTED]',
        description: 'You need a valid location proof for this zone to view this listing.',
        instructions: '[ENCRYPTED]',
      };
    }

    return this.decrypt(encryptedBytes, zoneId);
  }
}

// Export singleton for convenience
export const listingEncryption = ListingEncryption;
