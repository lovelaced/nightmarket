'use client';

import { useState, useEffect } from 'react';
import { useNightmarket } from '@/hooks/useNightmarket';
import { useAccount } from 'wagmi';
import { locationProofGenerator } from '@/lib/zkProofs';
import { globalZoneGrid, type GridZone } from '@/lib/globalZoneGrid';
import { geolocation } from '@/lib/geolocation';

interface LocationProofProps {
  onProofVerified: () => void;
}

export function LocationProof({ onProofVerified }: LocationProofProps) {
  const { address } = useAccount();
  const { verifyLocationProof, hasValidProof, isNightTime, checkNightTime } = useNightmarket();
  const [generating, setGenerating] = useState(false);
  const [checking, setChecking] = useState(true);
  const [status, setStatus] = useState('');
  const [error, setError] = useState<string | null>(null);
  const [currentZone, setCurrentZone] = useState<GridZone | null>(null);

  // Check night time when component mounts
  useEffect(() => {
    const check = async () => {
      await checkNightTime();
      setChecking(false);
    };
    check();
  }, [checkNightTime]);

  // Calculate zone from GPS
  useEffect(() => {
    const detectZone = async () => {
      if (!isNightTime) return;

      try {
        setStatus('calculating zone');

        const location = await geolocation.getLocation();
        const zone = globalZoneGrid.getZoneForCoordinates(location);

        setCurrentZone(zone);
        setStatus('');
      } catch (error: any) {
        console.error('Zone detection failed:', error);
        if (error.message?.includes('permission')) {
          setError('location permission required');
        } else if (error.message?.includes('unavailable')) {
          setError('gps unavailable');
        } else {
          setError('enable location access');
        }
        setStatus('');
      }
    };

    detectZone();
  }, [isNightTime]);

  const handleGenerateProof = async () => {
    if (!address || !isNightTime || !currentZone) return;

    setGenerating(true);
    setError(null);

    try {
      setStatus('generating zero-knowledge proof');

      const proofData = await locationProofGenerator.generate(
        currentZone.id,
        currentZone.bounds
      );

      setStatus('submitting proof');
      await verifyLocationProof(
        proofData.zoneId,
        proofData.proof,
        proofData.nullifier
      );

      setStatus('verified');
      setTimeout(() => {
        onProofVerified();
      }, 1000);

    } catch (error: any) {
      console.error('Proof generation failed:', error);
      setError(error.message || 'proof generation failed');
      setStatus('');
    } finally {
      setGenerating(false);
    }
  };

  // Check if user already has valid proof
  useEffect(() => {
    if (address) {
      hasValidProof(address).then(valid => {
        if (valid) onProofVerified();
      });
    }
  }, [address, hasValidProof, onProofVerified]);

  if (!address) {
    return (
      <div className="glass-strong rounded-sm p-12 text-center">
        <h2 className="text-xl font-light tracking-widest mb-4 uppercase">
          connect wallet
        </h2>
        <p className="text-sm text-gray-500 font-light tracking-wide">
          wallet connection required to prove location
        </p>
      </div>
    );
  }

  if (checking) {
    return (
      <div className="glass-strong rounded-sm p-12 text-center">
        <div className="flex items-center justify-center gap-3">
          <div className="w-4 h-4 border-2 border-white/20 border-t-white/60 rounded-full animate-spin" />
          <p className="text-sm text-gray-500 tracking-wider">checking market hours</p>
        </div>
      </div>
    );
  }

  if (!isNightTime) {
    return (
      <div className="glass-strong rounded-sm p-12 text-center border border-red-500/20">
        <h2 className="text-xl font-light tracking-widest mb-4 uppercase text-red-400">
          market closed
        </h2>
        <p className="text-sm text-gray-500 font-light mb-2 tracking-wide">
          hours of operation: 06:00 — 05:00 utc
        </p>
        <p className="text-[10px] text-gray-700 font-mono mt-4">
          return during operational hours to access market
        </p>
      </div>
    );
  }

  return (
    <div className="glass-strong rounded-sm p-12">
      <div className="max-w-2xl mx-auto">
        <h2 className="text-2xl font-extralight tracking-widest mb-6 uppercase text-center">
          location verification
        </h2>

        <p className="text-sm text-gray-500 font-light mb-8 text-center tracking-wide leading-relaxed">
          cryptographic proof of physical presence
          <br />
          <span className="text-xs text-gray-700">
            location data never leaves your device
          </span>
        </p>

        <div className="space-y-6">
          {/* Current Zone */}
          {currentZone && (
            <div className="bg-moonlight/5 border border-moonlight/20 rounded-sm p-6">
              <div className="flex items-center justify-between">
                <div>
                  <div className="text-[10px] text-gray-600 tracking-widest uppercase mb-1">
                    zone
                  </div>
                  <div className="text-2xl font-mono text-moonlight tracking-wider">
                    {currentZone.name}
                  </div>
                  <div className="text-[10px] text-gray-700 mt-1 font-mono">
                    grid [{currentZone.gridCoords.latIndex}, {currentZone.gridCoords.lonIndex}]
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* Status */}
          {status && (
            <div className="bg-moonlight/5 border border-moonlight/20 rounded-sm p-4">
              <p className="text-xs text-moonlight/80 font-mono text-center tracking-wide">
                {status}
              </p>
            </div>
          )}

          {/* Error */}
          {error && (
            <div className="bg-red-500/5 border border-red-500/20 rounded-sm p-4">
              <p className="text-xs text-red-400 font-mono tracking-wide">
                {error}
              </p>
            </div>
          )}

          {/* Generate Button */}
          <button
            onClick={handleGenerateProof}
            disabled={generating || !currentZone}
            className="w-full py-4 text-xs tracking-widest uppercase
                       border border-white/20 hover:border-moonlight/50
                       hover:bg-white/5 transition-all duration-500
                       disabled:opacity-50 disabled:cursor-not-allowed
                       relative overflow-hidden group"
          >
            {generating ? (
              <span className="flex items-center justify-center gap-2">
                <div className="w-4 h-4 border-2 border-white/20 border-t-white/80 rounded-full animate-spin" />
                {status || 'proving'}
              </span>
            ) : (
              <>
                <span className="relative z-10">generate proof</span>
                <div className="absolute inset-0 bg-gradient-to-r from-moonlight/0 via-moonlight/10 to-moonlight/0
                                translate-x-[-100%] group-hover:translate-x-[100%] transition-transform duration-1000" />
              </>
            )}
          </button>

          {/* Info */}
          <div className="text-[10px] text-gray-700 space-y-1 font-mono pt-4 border-t border-white/5 leading-relaxed">
            <p>→ zone automatically calculated from coordinates</p>
            <p>→ proof expires at 06:00 utc</p>
            <p>→ rate limited to one proof per hour</p>
            <p>→ zero-knowledge protocol preserves location privacy</p>
          </div>
        </div>
      </div>
    </div>
  );
}
