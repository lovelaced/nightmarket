'use client';

import { useState, useEffect } from 'react';
import { useListings } from '@/hooks/useListings';
import { ethers } from 'ethers';
import { globalZoneGrid, type GridZone } from '@/lib/globalZoneGrid';
import { geolocation } from '@/lib/geolocation';
import { listingEncryption } from '@/lib/encryption';

export function CreateListing() {
  const { createListing, creating } = useListings();
  const [currentZone, setCurrentZone] = useState<GridZone | null>(null);
  const [detectingZone, setDetectingZone] = useState(true);
  const [stage, setStage] = useState<'item' | 'coordinates'>('item');
  const [formData, setFormData] = useState({
    title: '',
    description: '',
    price: '',
    // 4-stage coordinates (revealed progressively during trade)
    stage1_area: '',      // General area (1km) - "northwest quadrant"
    stage2_block: '',     // Block/street (100m) - "between 5th and 6th"
    stage3_exact: '',     // Exact location (1m) - "north side, third bench"
    stage4_details: '',   // Visual aids - "under newspapers, look for blue tape"
  });

  // Calculate zone from GPS when component mounts
  useEffect(() => {
    const detectZone = async () => {
      try {
        const location = await geolocation.getLocation();
        const zone = globalZoneGrid.getZoneForCoordinates(location);
        setCurrentZone(zone);
      } catch (error) {
        console.error('Zone detection failed:', error);
      } finally {
        setDetectingZone(false);
      }
    };

    detectZone();
  }, []);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!currentZone) {
      return;
    }

    // Encrypt listing data with AES-256-GCM
    const encryptedData = await listingEncryption.encrypt(
      {
        title: formData.title,
        description: formData.description,
        instructions: JSON.stringify({
          stage1: formData.stage1_area,
          stage2: formData.stage2_block,
          stage3: formData.stage3_exact,
          stage4: formData.stage4_details,
        }),
      },
      currentZone.id
    );

    // Hash of complete drop instructions (for verification)
    const dropInstructions = `${formData.stage1_area}|${formData.stage2_block}|${formData.stage3_exact}|${formData.stage4_details}`;
    const dropZoneHash = ethers.keccak256(ethers.toUtf8Bytes(dropInstructions));

    await createListing(
      currentZone.id,
      encryptedData,
      ethers.parseEther(formData.price),
      dropZoneHash
    );

    // Reset form
    setFormData({
      title: '',
      description: '',
      price: '',
      stage1_area: '',
      stage2_block: '',
      stage3_exact: '',
      stage4_details: '',
    });
    setStage('item');
  };

  if (detectingZone) {
    return (
      <div className="max-w-3xl mx-auto">
        <div className="glass-strong rounded-sm p-12 text-center">
          <div className="flex items-center justify-center gap-3">
            <div className="w-4 h-4 border-2 border-white/20 border-t-white/60 rounded-full animate-spin" />
            <p className="text-sm text-gray-500 tracking-wider">calculating zone</p>
          </div>
        </div>
      </div>
    );
  }

  if (!currentZone) {
    return (
      <div className="max-w-3xl mx-auto">
        <div className="glass-strong rounded-sm p-12 text-center border border-red-500/20">
          <h2 className="text-xl font-light tracking-widest mb-4 uppercase text-red-400">
            location required
          </h2>
          <p className="text-sm text-gray-500 font-light">
            enable location access to continue
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="max-w-3xl mx-auto">
      <div className="glass-strong rounded-sm p-12">
        {/* Zone Display */}
        <div className="mb-8 pb-6 border-b border-white/5">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-[10px] text-gray-600 tracking-wider uppercase mb-1">
                your zone
              </div>
              <div className="text-xl font-mono text-moonlight">
                {currentZone.name}
              </div>
            </div>
            <div className="text-right">
              <div className="text-[10px] text-gray-700 font-mono">
                auto-calculated
              </div>
            </div>
          </div>
        </div>

        <form onSubmit={handleSubmit} className="space-y-8">
          {/* Stage Indicator */}
          <div className="flex items-center gap-4">
            <div className={`w-2 h-2 rounded-full ${stage === 'item' ? 'bg-moonlight' : 'bg-white/20'}`} />
            <div className="text-xs tracking-wider text-gray-${stage === 'item' ? '400' : '600'} uppercase">
              item details
            </div>
            <div className="flex-1 h-px bg-white/5" />
            <div className={`w-2 h-2 rounded-full ${stage === 'coordinates' ? 'bg-moonlight' : 'bg-white/20'}`} />
            <div className="text-xs tracking-wider text-gray-${stage === 'coordinates' ? '400' : '600'} uppercase">
              dead drop location
            </div>
          </div>

          {stage === 'item' && (
            <div className="space-y-6 animate-emerge">
              {/* Title */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  item
                </label>
                <input
                  type="text"
                  value={formData.title}
                  onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors font-light"
                  required
                  maxLength={60}
                />
                <div className="text-[10px] text-gray-700 mt-1 text-right font-mono">
                  {formData.title.length}/60
                </div>
              </div>

              {/* Description */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  description
                </label>
                <textarea
                  value={formData.description}
                  onChange={(e) => setFormData({ ...formData, description: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors resize-none font-light"
                  rows={4}
                  required
                  maxLength={200}
                />
                <div className="text-[10px] text-gray-700 mt-1 text-right font-mono">
                  {formData.description.length}/200
                </div>
              </div>

              {/* Price */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  price (eth)
                </label>
                <input
                  type="number"
                  step="0.001"
                  value={formData.price}
                  onChange={(e) => setFormData({ ...formData, price: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors font-mono"
                  required
                  min="0.001"
                />
              </div>

              {/* Next Button */}
              <button
                type="button"
                onClick={() => setStage('coordinates')}
                className="w-full py-4 text-xs tracking-widest uppercase
                           border border-white/20 hover:border-moonlight/50
                           hover:bg-white/5 transition-all duration-500
                           relative overflow-hidden group"
              >
                <span className="relative z-10">next: dead drop location</span>
                <div className="absolute inset-0 bg-gradient-to-r from-moonlight/0 via-moonlight/10 to-moonlight/0
                                translate-x-[-100%] group-hover:translate-x-[100%] transition-transform duration-1000" />
              </button>
            </div>
          )}

          {stage === 'coordinates' && (
            <div className="space-y-6 animate-emerge">
              {/* Explanation */}
              <div className="bg-moonlight/5 border border-moonlight/20 rounded-sm p-6">
                <div className="text-xs text-moonlight/80 font-mono space-y-2 leading-relaxed">
                  <p>progressive coordinate revelation protocol:</p>
                  <div className="pl-4 space-y-1 text-gray-500">
                    <p>stage 1: general area (visible to all buyers)</p>
                    <p>stage 2: approximate location (revealed on escrow lock)</p>
                    <p>stage 3: exact coordinates (revealed on confirmation)</p>
                    <p>stage 4: identification markers (revealed on arrival)</p>
                  </div>
                </div>
              </div>

              {/* Stage 1: Area */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  stage 1: general area (1km)
                </label>
                <input
                  type="text"
                  value={formData.stage1_area}
                  onChange={(e) => setFormData({ ...formData, stage1_area: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors font-light"
                  required
                  maxLength={100}
                />
                <div className="text-[10px] text-gray-700 mt-1 font-mono">
                  northeast quadrant. near the river. central district.
                </div>
              </div>

              {/* Stage 2: Block */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  stage 2: approximate block (100m)
                </label>
                <input
                  type="text"
                  value={formData.stage2_block}
                  onChange={(e) => setFormData({ ...formData, stage2_block: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors font-light"
                  required
                  maxLength={100}
                />
                <div className="text-[10px] text-gray-700 mt-1 font-mono">
                  between 5th and 6th street. west side of park. near library.
                </div>
              </div>

              {/* Stage 3: Exact */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  stage 3: exact location (1m)
                </label>
                <input
                  type="text"
                  value={formData.stage3_exact}
                  onChange={(e) => setFormData({ ...formData, stage3_exact: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors font-light"
                  required
                  maxLength={100}
                />
                <div className="text-[10px] text-gray-700 mt-1 font-mono">
                  third bench from north entrance. base of large oak tree.
                </div>
              </div>

              {/* Stage 4: Details */}
              <div>
                <label className="block text-[10px] tracking-widest text-gray-500 mb-2 uppercase">
                  stage 4: visual aids
                </label>
                <textarea
                  value={formData.stage4_details}
                  onChange={(e) => setFormData({ ...formData, stage4_details: e.target.value })}
                  className="w-full bg-black border border-white/10 rounded-sm px-4 py-3
                             text-sm focus:border-moonlight/50 focus:outline-none
                             transition-colors resize-none font-light"
                  rows={3}
                  required
                  maxLength={150}
                />
                <div className="text-[10px] text-gray-700 mt-1 space-y-1 font-mono">
                  <p>wrapped in black plastic. marked with blue tape.</p>
                  <p>hidden under loose brick. behind third planter.</p>
                  <p className="text-right">{formData.stage4_details.length}/150</p>
                </div>
              </div>

              {/* Security Notice */}
              <div className="bg-yellow-500/5 border border-yellow-500/20 rounded-sm p-4">
                <div className="text-[10px] text-yellow-500/80 font-mono space-y-1 leading-relaxed">
                  <p>location data encrypted and stored on-chain</p>
                  <p>coordinates revealed progressively during transaction</p>
                  <p className="pt-2 text-yellow-500/60">select 24/7 accessible public locations</p>
                  <p className="text-yellow-500/60">avoid private property and residential areas</p>
                </div>
              </div>

              {/* Actions */}
              <div className="flex gap-4">
                <button
                  type="button"
                  onClick={() => setStage('item')}
                  className="flex-1 py-4 text-xs tracking-widest uppercase
                             border border-white/10 hover:border-white/20
                             hover:bg-white/5 transition-all duration-500"
                >
                  back
                </button>

                <button
                  type="submit"
                  disabled={creating}
                  className="flex-1 py-4 text-xs tracking-widest uppercase
                             border border-moonlight/40 hover:border-moonlight/60
                             bg-moonlight/5 hover:bg-moonlight/10
                             transition-all duration-500
                             disabled:opacity-50 disabled:cursor-not-allowed
                             shadow-moonlight"
                >
                  {creating ? 'encrypting & publishing' : 'create listing'}
                </button>
              </div>
            </div>
          )}
        </form>
      </div>
    </div>
  );
}
