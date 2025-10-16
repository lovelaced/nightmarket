import { getDefaultConfig } from '@rainbow-me/rainbowkit';
import { paseo } from './chains';

export const config = getDefaultConfig({
  appName: 'Nightmarket',
  projectId: process.env.NEXT_PUBLIC_WALLET_CONNECT_PROJECT_ID || 'YOUR_PROJECT_ID',
  chains: [paseo],
  ssr: true,
});
