'use client';

import { useState } from 'react';
import { CustomConnectButton } from '@/components/CustomConnectButton';
import { NightStatus } from '@/components/NightStatus';
import { LocationProof } from '@/components/LocationProof';
import { ListingsBrowser } from '@/components/ListingsBrowser';
import { CreateListing } from '@/components/CreateListing';

export default function MarketPage() {
  const [view, setView] = useState<'browse' | 'create'>('browse');
  const [hasLocationProof, setHasLocationProof] = useState(false);

  return (
    <div className="min-h-screen bg-black text-white">
      {/* Header */}
      <header className="fixed top-0 left-0 right-0 z-50 glass-strong">
        <div className="max-w-7xl mx-auto px-6 py-4 flex items-center justify-between">
          <div className="flex items-center gap-4">
            <h1 className="text-xl font-extralight tracking-widest uppercase">
              nightmarket
            </h1>
            <div className="h-4 w-px bg-white/10" />
            <NightStatus />
          </div>

          <div className="flex items-center gap-4">
            <CustomConnectButton />
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="pt-24 px-6 pb-12 max-w-7xl mx-auto">
        {/* Location Proof Requirement */}
        {!hasLocationProof && (
          <div className="mb-12 animate-shadow-fade">
            <LocationProof onProofVerified={() => setHasLocationProof(true)} />
          </div>
        )}

        {/* Navigation Tabs */}
        {hasLocationProof && (
          <div className="mb-8 flex gap-2 border-b border-white/5">
            <button
              onClick={() => setView('browse')}
              className={`px-6 py-3 text-sm tracking-wider transition-all duration-300 ${
                view === 'browse'
                  ? 'text-white border-b-2 border-moonlight'
                  : 'text-gray-600 hover:text-gray-400'
              }`}
            >
              browse
            </button>
            <button
              onClick={() => setView('create')}
              className={`px-6 py-3 text-sm tracking-wider transition-all duration-300 ${
                view === 'create'
                  ? 'text-white border-b-2 border-moonlight'
                  : 'text-gray-600 hover:text-gray-400'
              }`}
            >
              sell
            </button>
          </div>
        )}

        {/* Content */}
        {hasLocationProof && (
          <div className="animate-emerge">
            {view === 'browse' && <ListingsBrowser />}
            {view === 'create' && <CreateListing />}
          </div>
        )}
      </main>

      {/* Footer */}
      <footer className="fixed bottom-0 left-0 right-0 py-4 px-6 text-center glass-strong border-t border-white/5">
        <div className="space-y-1">
          <p className="text-[10px] text-gray-700 tracking-wider font-mono">
            anonymous decentralized marketplace. time-restricted operations.
          </p>
          <p className="text-[9px] text-gray-800 tracking-wide font-mono">
            experimental protocol. use at your own risk. transactions are irreversible.
          </p>
        </div>
      </footer>
    </div>
  );
}
