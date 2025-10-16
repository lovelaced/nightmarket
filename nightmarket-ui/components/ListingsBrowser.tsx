'use client';

import { useState, useEffect } from 'react';
import { useListings } from '@/hooks/useListings';
import { ListingCard } from './ListingCard';

export function ListingsBrowser() {
  const { listings, activeCount, fetchListings, loading } = useListings();
  const [zoneFilter, setZoneFilter] = useState('all');

  useEffect(() => {
    fetchListings();
    const interval = setInterval(fetchListings, 15000); // Refresh every 15s
    return () => clearInterval(interval);
  }, [fetchListings]);

  return (
    <div className="space-y-8">
      {/* Header */}
      <div className="flex items-center justify-between">
        <div>
          <h2 className="text-2xl font-extralight tracking-widest uppercase">
            Active Listings
          </h2>
          <p className="text-xs text-gray-600 mt-1 font-mono">
            {activeCount} items available in the shadows
          </p>
        </div>

        {/* Zone Filter */}
        <select
          value={zoneFilter}
          onChange={(e) => setZoneFilter(e.target.value)}
          className="bg-black border border-white/10 rounded-sm px-4 py-2
                     text-xs tracking-wider focus:border-moonlight/50 focus:outline-none"
        >
          <option value="all">ALL ZONES</option>
          <option value="1">ZONE 1</option>
          <option value="2">ZONE 2</option>
          <option value="3">ZONE 3</option>
        </select>
      </div>

      {/* Loading State */}
      {loading && (
        <div className="flex items-center justify-center py-20">
          <div className="flex flex-col items-center gap-4">
            <div className="w-8 h-8 border-2 border-white/10 border-t-white/60 rounded-full animate-spin" />
            <p className="text-xs text-gray-600 tracking-wider">Loading listings...</p>
          </div>
        </div>
      )}

      {/* Listings Grid */}
      {!loading && listings.length > 0 && (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
          {listings.map((listing, index) => (
            <ListingCard
              key={listing.id}
              listing={listing}
              style={{ animationDelay: `${index * 100}ms` }}
            />
          ))}
        </div>
      )}

      {/* Empty State */}
      {!loading && listings.length === 0 && (
        <div className="glass rounded-sm p-20 text-center">
          <p className="text-gray-600 font-light tracking-wide">
            No active listings in this zone
          </p>
          <p className="text-xs text-gray-700 mt-2 font-mono">
            Be the first to create one
          </p>
        </div>
      )}
    </div>
  );
}
