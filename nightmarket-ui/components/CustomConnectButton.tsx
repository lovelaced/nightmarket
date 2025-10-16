'use client';

import { ConnectButton } from '@rainbow-me/rainbowkit';

export function CustomConnectButton() {
  return (
    <ConnectButton.Custom>
      {({
        account,
        chain,
        openAccountModal,
        openChainModal,
        openConnectModal,
        authenticationStatus,
        mounted,
      }) => {
        const ready = mounted && authenticationStatus !== 'loading';
        const connected = ready && account && chain;

        return (
          <div
            {...(!ready && {
              'aria-hidden': true,
              style: {
                opacity: 0,
                pointerEvents: 'none',
                userSelect: 'none',
              },
            })}
          >
            {(() => {
              if (!connected) {
                return (
                  <button
                    onClick={openConnectModal}
                    className="px-6 py-2.5 text-xs tracking-[0.2em] uppercase
                               border border-white/10 hover:border-moonlight/30
                               transition-all duration-500 hover:bg-white/5
                               font-light"
                    type="button"
                  >
                    Connect
                  </button>
                );
              }

              if (chain.unsupported) {
                return (
                  <button
                    onClick={openChainModal}
                    className="px-6 py-2.5 text-xs tracking-[0.2em] uppercase
                               border border-red-500/30 text-red-400
                               hover:border-red-500/50 transition-all duration-500"
                    type="button"
                  >
                    Wrong Network
                  </button>
                );
              }

              return (
                <button
                  onClick={openAccountModal}
                  className="px-6 py-2.5 text-xs tracking-[0.2em] uppercase
                             border border-moonlight/20 hover:border-moonlight/40
                             transition-all duration-500 hover:bg-moonlight/5
                             font-light font-mono"
                  type="button"
                >
                  {account.displayName}
                </button>
              );
            })()}
          </div>
        );
      }}
    </ConnectButton.Custom>
  );
}
