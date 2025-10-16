import { Chain } from 'wagmi/chains';

export const paseo: Chain = {
  id: 420420422,
  name: 'Paseo TestNet',
  nativeCurrency: {
    decimals: 18,
    name: 'PAS',
    symbol: 'PAS',
  },
  rpcUrls: {
    default: {
      http: ['https://testnet-passet-hub-eth-rpc.polkadot.io'],
    },
    public: {
      http: ['https://testnet-passet-hub-eth-rpc.polkadot.io'],
    },
  },
  blockExplorers: {
    default: {
      name: 'BlockScout',
      url: 'https://blockscout-passet-hub.parity-testnet.parity.io'
    },
  },
  testnet: true,
};
