/**
 * Global Zone Grid System
 *
 * Every location on Earth is in a zone. Zones are automatically calculated
 * from GPS coordinates using a global grid system.
 *
 * No manual zone configuration needed - zones are deterministic and universal.
 */

import type { Coordinates } from './geolocation';

export interface GridZone {
  id: number;
  name: string;
  bounds: {
    latMin: number;
    latMax: number;
    lonMin: number;
    lonMax: number;
  };
  gridCoords: {
    latIndex: number;
    lonIndex: number;
  };
}

/**
 * Global Zone Grid Configuration
 */
const GRID_CONFIG = {
  // Zone size in degrees (approximately 5-10km at equator)
  ZONE_SIZE_LAT: 0.05,  // ~5.5km
  ZONE_SIZE_LON: 0.05,  // ~5.5km at equator, varies by latitude

  // Origin point for grid (can be 0,0 or any reference)
  ORIGIN_LAT: 0,
  ORIGIN_LON: 0,
};

export class GlobalZoneGrid {
  /**
   * Calculate which zone a coordinate is in
   * This is deterministic - same coordinates always return same zone
   *
   * @param coords - GPS coordinates
   * @returns Zone information
   */
  getZoneForCoordinates(coords: Coordinates): GridZone {
    // Calculate grid indices
    const latIndex = Math.floor(
      (coords.lat - GRID_CONFIG.ORIGIN_LAT) / GRID_CONFIG.ZONE_SIZE_LAT
    );
    const lonIndex = Math.floor(
      (coords.lon - GRID_CONFIG.ORIGIN_LON) / GRID_CONFIG.ZONE_SIZE_LON
    );

    // Calculate zone ID from grid coordinates
    // Use a deterministic hash-like function to map (latIndex, lonIndex) -> zone_id
    const zoneId = this.gridCoordsToZoneId(latIndex, lonIndex);

    // Calculate zone boundaries
    const latMin = GRID_CONFIG.ORIGIN_LAT + (latIndex * GRID_CONFIG.ZONE_SIZE_LAT);
    const latMax = latMin + GRID_CONFIG.ZONE_SIZE_LAT;
    const lonMin = GRID_CONFIG.ORIGIN_LON + (lonIndex * GRID_CONFIG.ZONE_SIZE_LON);
    const lonMax = lonMin + GRID_CONFIG.ZONE_SIZE_LON;

    return {
      id: zoneId,
      name: this.generateZoneName(latIndex, lonIndex, coords),
      bounds: { latMin, latMax, lonMin, lonMax },
      gridCoords: { latIndex, lonIndex },
    };
  }

  /**
   * Convert grid coordinates to a unique zone ID
   * Uses a deterministic hash to avoid sequential zone IDs (privacy)
   */
  private gridCoordsToZoneId(latIndex: number, lonIndex: number): number {
    // Mix the indices to create a pseudo-random but deterministic ID
    // This prevents sequential zones from having sequential IDs (privacy benefit)

    // Simple mixing function (can be improved with better hash)
    const mixed = (latIndex * 73856093) ^ (lonIndex * 19349663);

    // Ensure positive and within uint32 range
    return (mixed >>> 0) % 0xFFFFFFFF;
  }

  /**
   * Generate a human-readable name for a zone
   * Based on grid coordinates and approximate location
   */
  private generateZoneName(latIndex: number, lonIndex: number, coords: Coordinates): string {
    // Determine hemisphere and general area
    const latHemisphere = coords.lat >= 0 ? 'N' : 'S';
    const lonHemisphere = coords.lon >= 0 ? 'E' : 'W';

    // Format as grid reference (like map coordinates)
    const latAbs = Math.abs(latIndex);
    const lonAbs = Math.abs(lonIndex);

    return `${latHemisphere}${latAbs}-${lonHemisphere}${lonAbs}`;
  }

  /**
   * Get all adjacent zones (8 neighbors + current zone)
   * Useful for finding nearby listings
   */
  getAdjacentZones(coords: Coordinates): GridZone[] {
    const currentZone = this.getZoneForCoordinates(coords);
    const { latIndex, lonIndex } = currentZone.gridCoords;

    const zones: GridZone[] = [currentZone];

    // Add 8 surrounding zones
    for (let dLat = -1; dLat <= 1; dLat++) {
      for (let dLon = -1; dLon <= 1; dLon++) {
        if (dLat === 0 && dLon === 0) continue; // Skip current zone

        const adjLatIndex = latIndex + dLat;
        const adjLonIndex = lonIndex + dLon;

        // Calculate adjacent zone bounds
        const latMin = GRID_CONFIG.ORIGIN_LAT + (adjLatIndex * GRID_CONFIG.ZONE_SIZE_LAT);
        const latMax = latMin + GRID_CONFIG.ZONE_SIZE_LAT;
        const lonMin = GRID_CONFIG.ORIGIN_LON + (adjLonIndex * GRID_CONFIG.ZONE_SIZE_LON);
        const lonMax = lonMin + GRID_CONFIG.ZONE_SIZE_LON;

        zones.push({
          id: this.gridCoordsToZoneId(adjLatIndex, adjLonIndex),
          name: this.generateZoneName(adjLatIndex, adjLonIndex, {
            lat: (latMin + latMax) / 2,
            lon: (lonMin + lonMax) / 2,
            accuracy: 0,
          }),
          bounds: { latMin, latMax, lonMin, lonMax },
          gridCoords: { latIndex: adjLatIndex, lonIndex: adjLonIndex },
        });
      }
    }

    return zones;
  }

  /**
   * Calculate zone from zone ID (reverse lookup)
   * Note: This requires searching or storing the grid coords in the ID
   * For now, we don't support reverse lookup (not needed for UX)
   */
  getZoneById(zoneId: number): GridZone | null {
    // Not implemented - zones are calculated from GPS, not looked up by ID
    // This is acceptable because users never manually select zones
    return null;
  }

  /**
   * Get approximate zone center coordinates
   */
  getZoneCenter(zone: GridZone): Coordinates {
    return {
      lat: (zone.bounds.latMin + zone.bounds.latMax) / 2,
      lon: (zone.bounds.lonMin + zone.bounds.lonMax) / 2,
      accuracy: 0,
    };
  }

  /**
   * Calculate zone size in meters (varies by latitude)
   */
  getZoneSizeMeters(latitude: number): { width: number; height: number } {
    // Height is constant (latitude degrees)
    const heightMeters = GRID_CONFIG.ZONE_SIZE_LAT * 111_000; // ~5.5km

    // Width varies by latitude (longitude degrees compress near poles)
    const widthMeters = GRID_CONFIG.ZONE_SIZE_LON * 111_000 * Math.cos((latitude * Math.PI) / 180);

    return {
      width: widthMeters,
      height: heightMeters,
    };
  }
}

// Export singleton instance
export const globalZoneGrid = new GlobalZoneGrid();
