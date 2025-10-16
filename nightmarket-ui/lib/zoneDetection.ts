/**
 * Zone Auto-Detection Service
 *
 * Automatically determines which zone(s) a user is in based on GPS coordinates.
 * Users should never need to manually select zones.
 */

import { ethers } from 'ethers';
import { CONTRACTS, ZONES_ABI } from './contracts';
import { geolocation, type Coordinates, type ZoneBounds } from './geolocation';

export interface Zone {
  id: number;
  bounds: ZoneBounds;
  name?: string;
  distance?: number; // Distance from user to zone center (meters)
}

export class ZoneDetectionService {
  private zoneCache: Zone[] | null = null;
  private cacheTimestamp: number = 0;
  private readonly CACHE_DURATION = 60 * 60 * 1000; // 1 hour

  /**
   * Detect which zone(s) the user is currently in
   * Returns array of zones (usually 1, but can be multiple if zones overlap)
   *
   * @param provider - Ethers provider for contract calls
   * @param userLocation - Optional user location (if already known)
   * @returns Array of zones user is currently in
   */
  async detectCurrentZones(
    provider: ethers.JsonRpcProvider,
    userLocation?: Coordinates
  ): Promise<Zone[]> {
    // 1. Get user's location if not provided
    const location = userLocation || await geolocation.getLocation();

    // 2. Fetch all zones
    const allZones = await this.getAllZones(provider);

    // 3. Filter to zones that contain the user
    const matchingZones = allZones.filter(zone =>
      geolocation.isInZone(location, zone.bounds)
    );

    // 4. Calculate distance to zone center for each match
    return matchingZones.map(zone => ({
      ...zone,
      distance: geolocation.calculateDistance(location, this.getZoneCenter(zone.bounds)),
    })).sort((a, b) => (a.distance || 0) - (b.distance || 0)); // Closest first
  }

  /**
   * Detect the primary zone (closest to user's location)
   *
   * @param provider - Ethers provider
   * @param userLocation - Optional user location
   * @returns Primary zone or null if not in any zone
   */
  async detectPrimaryZone(
    provider: ethers.JsonRpcProvider,
    userLocation?: Coordinates
  ): Promise<Zone | null> {
    const zones = await this.detectCurrentZones(provider, userLocation);
    return zones[0] || null; // Return closest zone
  }

  /**
   * Fetch all zones from the contract
   * Results are cached for 1 hour to reduce RPC calls
   *
   * @param provider - Ethers provider
   * @returns Array of all configured zones
   */
  async getAllZones(provider: ethers.JsonRpcProvider): Promise<Zone[]> {
    // Check cache
    const now = Date.now();
    if (this.zoneCache && (now - this.cacheTimestamp < this.CACHE_DURATION)) {
      return this.zoneCache;
    }

    try {
      const contract = new ethers.Contract(CONTRACTS.ZONES, ZONES_ABI, provider);

      // Get total zone count
      const zoneCount = await contract.getZoneCount();
      const count = Number(zoneCount);

      // Fetch all zones in parallel
      const zonePromises = Array.from({ length: count }, (_, i) =>
        this.fetchZone(contract, i + 1)
      );

      const zones = await Promise.all(zonePromises);
      const validZones = zones.filter(z => z !== null) as Zone[];

      // Update cache
      this.zoneCache = validZones;
      this.cacheTimestamp = now;

      return validZones;
    } catch (error) {
      console.error('Error fetching zones:', error);
      return [];
    }
  }

  /**
   * Fetch a single zone from the contract
   */
  private async fetchZone(contract: ethers.Contract, zoneId: number): Promise<Zone | null> {
    try {
      const zoneData = await contract.getZone(zoneId);

      // Zone data format: [lat_min, lon_min, lat_max, lon_max] (int32, scaled by 1e6)
      return {
        id: zoneId,
        bounds: {
          latMin: Number(zoneData[0]) / 1e6,
          latMax: Number(zoneData[2]) / 1e6,
          lonMin: Number(zoneData[1]) / 1e6,
          lonMax: Number(zoneData[3]) / 1e6,
        },
        name: this.getZoneName(zoneId), // Optional: fetch from metadata or hardcode
      };
    } catch (error) {
      console.error(`Error fetching zone ${zoneId}:`, error);
      return null;
    }
  }

  /**
   * Calculate center point of a zone
   */
  private getZoneCenter(bounds: ZoneBounds): { lat: number; lon: number } {
    return {
      lat: (bounds.latMin + bounds.latMax) / 2,
      lon: (bounds.lonMin + bounds.lonMax) / 2,
    };
  }

  /**
   * Get human-readable name for a zone
   * In production: fetch from contract metadata or external API
   */
  private getZoneName(zoneId: number): string {
    const names: Record<number, string> = {
      1: 'Downtown',
      2: 'Eastside',
      3: 'Northside',
      4: 'Westside',
      5: 'Southside',
      // Add more as zones are configured
    };
    return names[zoneId] || `Zone ${zoneId}`;
  }

  /**
   * Check if user is in ANY zone
   *
   * @param provider - Ethers provider
   * @param userLocation - Optional user location
   * @returns true if user is in at least one zone
   */
  async isInAnyZone(
    provider: ethers.JsonRpcProvider,
    userLocation?: Coordinates
  ): Promise<boolean> {
    const zones = await this.detectCurrentZones(provider, userLocation);
    return zones.length > 0;
  }

  /**
   * Get nearby zones (within a certain radius, even if user not inside)
   * Useful for showing "You're close to Zone X (500m away)"
   *
   * @param provider - Ethers provider
   * @param radiusMeters - Radius to search (default 5000m = 5km)
   * @param userLocation - Optional user location
   * @returns Array of nearby zones with distances
   */
  async getNearbyZones(
    provider: ethers.JsonRpcProvider,
    radiusMeters: number = 5000,
    userLocation?: Coordinates
  ): Promise<Zone[]> {
    const location = userLocation || await geolocation.getLocation();
    const allZones = await this.getAllZones(provider);

    return allZones
      .map(zone => ({
        ...zone,
        distance: geolocation.calculateDistance(location, this.getZoneCenter(zone.bounds)),
      }))
      .filter(zone => (zone.distance || 0) <= radiusMeters)
      .sort((a, b) => (a.distance || 0) - (b.distance || 0));
  }

  /**
   * Clear the zone cache (force refresh)
   */
  clearCache(): void {
    this.zoneCache = null;
    this.cacheTimestamp = 0;
  }
}

// Export singleton instance
export const zoneDetection = new ZoneDetectionService();
