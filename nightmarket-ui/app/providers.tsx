'use client';

import '@rainbow-me/rainbowkit/styles.css';
import { RainbowKitProvider, darkTheme } from '@rainbow-me/rainbowkit';
import { WagmiProvider } from 'wagmi';
import { QueryClientProvider, QueryClient } from '@tanstack/react-query';
import { config } from '@/lib/wagmi';

const queryClient = new QueryClient();

export function Providers({ children }: { children: React.ReactNode }) {
  const customTheme = darkTheme({
    accentColor: '#6b8cff',
    accentColorForeground: '#ffffff',
    borderRadius: 'none',
    fontStack: 'system',
    overlayBlur: 'large',
  });

  customTheme.colors.modalBackground = '#000000';
  customTheme.colors.modalBackdrop = 'rgba(0, 0, 0, 0.95)';
  customTheme.colors.modalBorder = 'rgba(255, 255, 255, 0.05)';
  customTheme.radii.modal = '0px';
  customTheme.radii.modalMobile = '0px';

  return (
    <WagmiProvider config={config}>
      <QueryClientProvider client={queryClient}>
        <RainbowKitProvider
          theme={customTheme}
          modalSize="compact"
          showRecentTransactions={false}
          avatar={() => null}
        >
          {children}
        </RainbowKitProvider>
      </QueryClientProvider>
    </WagmiProvider>
  );
}
