'use client';

import { useState, useCallback } from 'react';
import { ethers, BrowserProvider } from 'ethers';
import { useAccount, useWalletClient } from 'wagmi';
import { CONTRACTS, LISTINGS_ABI } from '@/lib/contracts';

interface Listing {
  id: string;
  seller: string;
  zoneId: number;
  encryptedData: string;
  price: string;
  dropZoneHash: string;
  expiresAt: number;
}

export function useListings() {
  const { address } = useAccount();
  const { data: walletClient } = useWalletClient();

  const [listings, setListings] = useState<Listing[]>([]);
  const [activeCount, setActiveCount] = useState(0);
  const [loading, setLoading] = useState(false);
  const [creating, setCreating] = useState(false);

  // Fetch active listings
  const fetchListings = useCallback(async () => {
    if (!CONTRACTS.LISTINGS) return;

    setLoading(true);
    try {
      const rpcUrl = process.env.NEXT_PUBLIC_RPC_URL || 'https://testnet-passet-hub-eth-rpc.polkadot.io';
      const provider = new ethers.JsonRpcProvider(rpcUrl);
      const contract = new ethers.Contract(CONTRACTS.LISTINGS, LISTINGS_ABI, provider);

      // Get active count
      const count = await contract.getActiveCount();
      setActiveCount(Number(count));

      // Fetch listings in batches
      const batchSize = 10;
      const listingIds: bigint[] = [];

      for (let i = 0; i < Math.min(Number(count), 100); i += batchSize) {
        const zoneListings = await contract.getListingsByZone(1, i, batchSize);
        listingIds.push(...zoneListings);
      }

      // Fetch full listing data
      const listingsData: Listing[] = [];
      for (const id of listingIds.slice(0, 30)) {
        // Limit to 30 for now
        try {
          // Contract returns raw 328 bytes, need to call directly
          const iface = new ethers.Interface(LISTINGS_ABI);
          const calldata = iface.encodeFunctionData('getListing', [id]);

          const result = await provider.call({
            to: CONTRACTS.LISTINGS,
            data: calldata,
          });

          // Result is raw 328 bytes (0x + 656 hex chars)
          if (result.length < 658) { // 0x + 656 chars
            console.warn(`Listing ${id} returned incomplete data`);
            continue;
          }

          // Parse raw bytes (no ABI decoding)
          // Layout: seller(20) + zone_id(4) + encrypted(256) + price(8) + drop_hash(32) + expiry(8)
          const bytes = result.slice(2); // Remove 0x

          listingsData.push({
            id: id.toString(),
            seller: '0x' + bytes.slice(0, 40), // 20 bytes
            zoneId: parseInt(bytes.slice(40, 48), 16), // 4 bytes
            encryptedData: bytes.slice(48, 560), // 256 bytes
            price: ethers.formatEther('0x' + bytes.slice(560, 576)), // 8 bytes
            dropZoneHash: '0x' + bytes.slice(576, 640), // 32 bytes
            expiresAt: parseInt(bytes.slice(640, 656), 16) * 1000, // 8 bytes, convert to ms
          });
        } catch (error) {
          console.error(`Error fetching listing ${id}:`, error);
        }
      }

      setListings(listingsData);
    } catch (error) {
      console.error('Error fetching listings:', error);
    } finally {
      setLoading(false);
    }
  }, []);

  // Create new listing
  const createListing = useCallback(
    async (
      zoneId: number,
      encryptedData: Uint8Array,
      price: bigint,
      dropZoneHash: string
    ) => {
      if (!walletClient || !address) throw new Error('Wallet not connected');

      setCreating(true);
      try {
        const provider = new BrowserProvider(walletClient as any);
        const signer = await provider.getSigner();
        const contract = new ethers.Contract(CONTRACTS.LISTINGS, LISTINGS_ABI, signer);

        const tx = await contract.createListing(zoneId, encryptedData, price, dropZoneHash);
        const receipt = await tx.wait();

        console.log('Listing created:', receipt.transactionHash);

        // Refresh listings
        await fetchListings();
      } catch (error) {
        console.error('Error creating listing:', error);
        throw error;
      } finally {
        setCreating(false);
      }
    },
    [walletClient, address, fetchListings]
  );

  return {
    listings,
    activeCount,
    loading,
    creating,
    fetchListings,
    createListing,
  };
}
