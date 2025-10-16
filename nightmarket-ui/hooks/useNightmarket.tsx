'use client';

import { useState, useCallback, useEffect } from 'react';
import { ethers, BrowserProvider, JsonRpcProvider } from 'ethers';
import { useAccount, useWalletClient } from 'wagmi';
import { CONTRACTS, ZONES_ABI } from '@/lib/contracts';

export function useNightmarket() {
  const { address } = useAccount();
  const { data: walletClient } = useWalletClient();

  const [isNightTime, setIsNightTime] = useState(false);
  const [provider, setProvider] = useState<BrowserProvider | null>(null);
  const [readOnlyProvider, setReadOnlyProvider] = useState<JsonRpcProvider | null>(null);

  // Initialize providers
  useEffect(() => {
    const rpcUrl = process.env.NEXT_PUBLIC_RPC_URL || 'https://testnet-passet-hub-eth-rpc.polkadot.io';
    const readProvider = new ethers.JsonRpcProvider(rpcUrl);
    setReadOnlyProvider(readProvider);
  }, []);

  useEffect(() => {
    if (walletClient) {
      const ethersProvider = new BrowserProvider(walletClient as any);
      setProvider(ethersProvider);
    }
  }, [walletClient]);

  // Check if it's night time
  const checkNightTime = useCallback(async () => {
    if (!readOnlyProvider || !CONTRACTS.ZONES) return;

    try {
      const contract = new ethers.Contract(CONTRACTS.ZONES, ZONES_ABI, readOnlyProvider);
      const result = await contract.isNightTime();
      setIsNightTime(result);
    } catch (error) {
      console.error('Error checking night time:', error);
      // Fallback to UTC time check (market hours: 6 AM - 5 AM)
      const now = new Date();
      const hour = now.getUTCHours();
      setIsNightTime(hour >= 6 || hour < 5);
    }
  }, [readOnlyProvider]);

  // Verify location proof
  const verifyLocationProof = useCallback(
    async (zoneId: number, proof: Uint8Array, nullifier: Uint8Array) => {
      if (!provider || !address) throw new Error('Wallet not connected');

      const signer = await provider.getSigner();
      const contract = new ethers.Contract(CONTRACTS.ZONES, ZONES_ABI, signer);

      const tx = await contract.verifyLocationProof(zoneId, proof, nullifier);
      await tx.wait();
    },
    [provider, address]
  );

  // Check if user has valid proof
  const hasValidProof = useCallback(
    async (userAddress: string): Promise<boolean> => {
      if (!readOnlyProvider) return false;

      try {
        const contract = new ethers.Contract(CONTRACTS.ZONES, ZONES_ABI, readOnlyProvider);
        return await contract.hasValidProof(userAddress);
      } catch (error) {
        console.error('Error checking proof:', error);
        return false;
      }
    },
    [readOnlyProvider]
  );

  // Get zone boundaries
  const getZoneBounds = useCallback(
    async (zoneId: number): Promise<{ latMin: number; latMax: number; lonMin: number; lonMax: number } | null> => {
      if (!readOnlyProvider) return null;

      try {
        const contract = new ethers.Contract(CONTRACTS.ZONES, ZONES_ABI, readOnlyProvider);
        const zoneData = await contract.getZone(zoneId);

        // Zone data format: [lat_min, lon_min, lat_max, lon_max]
        // Convert from scaled integers (1e6) to decimals
        return {
          latMin: Number(zoneData[0]) / 1e6,
          latMax: Number(zoneData[2]) / 1e6,
          lonMin: Number(zoneData[1]) / 1e6,
          lonMax: Number(zoneData[3]) / 1e6,
        };
      } catch (error) {
        console.error('Error fetching zone:', error);
        return null;
      }
    },
    [readOnlyProvider]
  );

  return {
    isNightTime,
    checkNightTime,
    verifyLocationProof,
    hasValidProof,
    getZoneBounds,
    provider,
    readOnlyProvider,
  };
}
