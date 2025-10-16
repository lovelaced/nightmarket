/**
 * Geolocation and Signal Collection Utilities
 *
 * Provides location services and signal fingerprinting for ZK proof generation.
 */

export interface Coordinates {
  lat: number;
  lon: number;
  accuracy: number;
}

export interface ZoneBounds {
  latMin: number;
  latMax: number;
  lonMin: number;
  lonMax: number;
}

export interface SignalData {
  hash: bigint;
  strength?: number;
  type: 'wifi' | 'cellular' | 'browser';
}

export class GeolocationService {
  /**
   * Get user's current GPS location
   * Requests permission if not already granted
   */
  async getLocation(): Promise<Coordinates> {
    return new Promise((resolve, reject) => {
      if (!navigator.geolocation) {
        reject(new Error('Geolocation not supported by this browser'));
        return;
      }

      navigator.geolocation.getCurrentPosition(
        (position) => {
          resolve({
            lat: position.coords.latitude,
            lon: position.coords.longitude,
            accuracy: position.coords.accuracy,
          });
        },
        (error) => {
          switch (error.code) {
            case error.PERMISSION_DENIED:
              reject(new Error('Location permission denied. Please enable location access.'));
              break;
            case error.POSITION_UNAVAILABLE:
              reject(new Error('Location unavailable. Please check GPS signal.'));
              break;
            case error.TIMEOUT:
              reject(new Error('Location request timed out. Please try again.'));
              break;
            default:
              reject(new Error(`Geolocation error: ${error.message}`));
          }
        },
        {
          enableHighAccuracy: true,
          timeout: 10000,
          maximumAge: 0, // Don't use cached location
        }
      );
    });
  }

  /**
   * Check if coordinates are within zone boundaries
   */
  isInZone(location: Coordinates, zone: ZoneBounds): boolean {
    return (
      location.lat >= zone.latMin &&
      location.lat <= zone.latMax &&
      location.lon >= zone.lonMin &&
      location.lon <= zone.lonMax
    );
  }

  /**
   * Calculate distance between two points (Haversine formula)
   * Returns distance in meters
   */
  calculateDistance(
    point1: { lat: number; lon: number },
    point2: { lat: number; lon: number }
  ): number {
    const R = 6371e3; // Earth radius in meters
    const φ1 = (point1.lat * Math.PI) / 180;
    const φ2 = (point2.lat * Math.PI) / 180;
    const Δφ = ((point2.lat - point1.lat) * Math.PI) / 180;
    const Δλ = ((point2.lon - point1.lon) * Math.PI) / 180;

    const a =
      Math.sin(Δφ / 2) * Math.sin(Δφ / 2) +
      Math.cos(φ1) * Math.cos(φ2) * Math.sin(Δλ / 2) * Math.sin(Δλ / 2);

    const c = 2 * Math.atan2(Math.sqrt(a), Math.sqrt(1 - a));

    return R * c;
  }

  /**
   * Collect signal data for location proof
   *
   * NOTE: Web browsers have limited access to WiFi/cellular signals for privacy.
   * This implementation uses browser fingerprinting as a fallback.
   *
   * For production, use a native mobile app with proper signal scanning APIs.
   */
  async collectSignals(): Promise<bigint[]> {
    // In a web browser, we can't access actual WiFi SSIDs or cell tower IDs
    // We fall back to browser fingerprinting which provides consistent signals
    // but less security than real signal data

    const signals: string[] = [];

    // 1. User Agent
    signals.push(navigator.userAgent);

    // 2. Timezone
    signals.push(Intl.DateTimeFormat().resolvedOptions().timeZone);

    // 3. Screen resolution
    signals.push(`${screen.width}x${screen.height}x${screen.colorDepth}`);

    // 4. Language preferences
    signals.push(JSON.stringify(navigator.languages));

    // 5. Platform
    signals.push(navigator.platform);

    // 6. Hardware concurrency
    signals.push(navigator.hardwareConcurrency?.toString() || '0');

    // 7. Device memory (if available)
    signals.push((navigator as any).deviceMemory?.toString() || '0');

    // 8. Connection type (if available)
    const connection = (navigator as any).connection;
    signals.push(connection?.effectiveType || 'unknown');

    // Hash each signal to create fingerprints
    const hashes = await Promise.all(
      signals.map(signal => this.hashString(signal))
    );

    return hashes;
  }

  /**
   * Hash a string to create a signal fingerprint
   */
  private async hashString(str: string): Promise<bigint> {
    const encoder = new TextEncoder();
    const data = encoder.encode(str);
    const hashBuffer = await crypto.subtle.digest('SHA-256', data);
    const hashArray = Array.from(new Uint8Array(hashBuffer));

    // Take first 8 bytes and convert to bigint
    const hex = hashArray
      .slice(0, 8)
      .map(b => b.toString(16).padStart(2, '0'))
      .join('');

    return BigInt('0x' + hex);
  }

  /**
   * Format coordinates for circuit (scaled by 1e6 to avoid decimals)
   */
  formatCoordinatesForCircuit(coords: Coordinates): {
    lat: number;
    lon: number;
  } {
    return {
      lat: Math.round(coords.lat * 1e6),
      lon: Math.round(coords.lon * 1e6),
    };
  }

  /**
   * Check if user has granted location permission
   */
  async checkLocationPermission(): Promise<boolean> {
    if (!navigator.permissions) {
      return false; // Permission API not supported
    }

    try {
      const result = await navigator.permissions.query({ name: 'geolocation' as PermissionName });
      return result.state === 'granted';
    } catch {
      return false;
    }
  }
}

// Export singleton instance
export const geolocation = new GeolocationService();
