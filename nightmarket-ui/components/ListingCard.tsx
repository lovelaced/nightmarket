'use client';

import { useState, useEffect } from 'react';
import { useAccount } from 'wagmi';
import { listingEncryption, type ListingData } from '@/lib/encryption';

interface Listing {
  id: string;
  seller: string;
  zoneId: number;
  encryptedData: string;
  price: string;
  expiresAt: number;
}

interface ListingCardProps {
  listing: Listing;
  style?: React.CSSProperties;
  onPurchase?: (listingId: string) => void;
}

export function ListingCard({ listing, style, onPurchase }: ListingCardProps) {
  const { address } = useAccount();
  const [expanded, setExpanded] = useState(false);
  const [decryptedData, setDecryptedData] = useState<ListingData | null>(null);
  const [decrypting, setDecrypting] = useState(false);
  const [stage1Coords, setStage1Coords] = useState<string>('');

  // Decrypt listing data
  useEffect(() => {
    const decrypt = async () => {
      if (!address) return;

      setDecrypting(true);
      try {
        const data = await listingEncryption.decryptWithAccess(
          listing.encryptedData,
          listing.zoneId,
          address
        );
        setDecryptedData(data);

        // Extract stage 1 coordinates (always visible)
        try {
          const instructions = JSON.parse(data.instructions);
          setStage1Coords(instructions.stage1 || '');
        } catch {
          setStage1Coords('');
        }
      } catch (error) {
        console.error('Decryption failed:', error);
        setDecryptedData({
          title: '[encrypted]',
          description: 'location proof required for this zone',
          instructions: '',
        });
      } finally {
        setDecrypting(false);
      }
    };

    decrypt();
  }, [listing.encryptedData, listing.zoneId, address]);

  const canPurchase = decryptedData && decryptedData.title !== '[encrypted]';

  return (
    <div
      className="glass hover:glass-strong rounded-sm p-6 border border-white/5
                 hover:border-moonlight/20 transition-all duration-500
                 hover:shadow-moonlight cursor-pointer animate-shadow-fade"
      style={style}
      onClick={() => setExpanded(!expanded)}
    >
      {/* Header */}
      <div className="flex items-start justify-between mb-4">
        <div className="flex-1">
          {decrypting ? (
            <div className="text-sm text-gray-600 tracking-wide">
              <div className="w-3 h-3 border-2 border-white/20 border-t-white/60 rounded-full animate-spin inline-block mr-2" />
              decrypting
            </div>
          ) : decryptedData ? (
            <>
              <div className="text-base font-light mb-1 tracking-wide">
                {decryptedData.title}
              </div>
              <div className="text-lg font-mono text-moonlight">
                {parseFloat(listing.price).toFixed(3)} eth
              </div>
            </>
          ) : (
            <div className="text-base font-light text-gray-600 tracking-wide">
              [encrypted]
            </div>
          )}
        </div>

        <div className="w-6 h-6 rounded-full border border-moonlight/30 flex items-center justify-center">
          <div className="w-2 h-2 rounded-full bg-moonlight/60" />
        </div>
      </div>

      {/* Description */}
      {decryptedData && canPurchase && (
        <div className="mb-4">
          <div className="text-xs text-gray-500 mb-2 tracking-wide font-light">
            {decryptedData.description}
          </div>

          {/* Stage 1 Coordinates (always visible) */}
          {stage1Coords && (
            <div className="mt-3 pt-3 border-t border-white/5">
              <div className="text-[10px] text-gray-700 uppercase tracking-wider mb-1">
                location (stage 1)
              </div>
              <div className="text-xs text-moonlight/80 font-mono">
                {stage1Coords}
              </div>
            </div>
          )}

          {/* Additional details when expanded */}
          {expanded && (
            <div className="mt-4 pt-4 border-t border-white/5 space-y-3">
              <div>
                <div className="text-[10px] text-gray-700 uppercase tracking-wider mb-1">
                  coordinate revelation
                </div>
                <div className="text-[10px] text-gray-600 font-mono space-y-1 leading-relaxed">
                  <p>additional stages revealed during transaction</p>
                  <p>escrow protects both parties</p>
                  <p>dispute resolution available if needed</p>
                </div>
              </div>
            </div>
          )}
        </div>
      )}

      {/* Footer */}
      <div className="flex items-center justify-between text-[10px] text-gray-700 pt-4 border-t border-white/5">
        <span className="font-mono">#{listing.id.slice(0, 6)}</span>
        <span className="tracking-wider font-mono">
          expires {new Date(listing.expiresAt).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
        </span>
      </div>

      {/* Purchase Button */}
      {expanded && canPurchase && onPurchase && (
        <button
          className="w-full mt-4 py-3 text-xs tracking-widest uppercase
                     border border-moonlight/30 hover:border-moonlight/60
                     hover:bg-moonlight/10 transition-all duration-300
                     relative overflow-hidden group"
          onClick={(e) => {
            e.stopPropagation();
            onPurchase(listing.id);
          }}
        >
          <span className="relative z-10">initiate purchase</span>
          <div className="absolute inset-0 bg-gradient-to-r from-moonlight/0 via-moonlight/20 to-moonlight/0
                          translate-x-[-100%] group-hover:translate-x-[100%] transition-transform duration-700" />
        </button>
      )}

      {/* Access restricted notice */}
      {expanded && !canPurchase && decryptedData && (
        <div className="mt-4 bg-red-500/5 border border-red-500/20 rounded-sm p-3">
          <p className="text-[10px] text-red-400/80 font-mono text-center leading-relaxed">
            location verification required for zone access
            <br />
            generate proof to decrypt listing details
          </p>
        </div>
      )}
    </div>
  );
}
