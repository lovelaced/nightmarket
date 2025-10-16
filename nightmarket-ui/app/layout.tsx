import type { Metadata } from 'next';
import './globals.css';
import { Providers } from './providers';

export const metadata: Metadata = {
  title: 'Nightmarket - Anonymous Marketplace',
  description: 'Decentralized anonymous marketplace operating during night hours (2:00-5:00 AM)',
  keywords: ['nightmarket', 'anonymous', 'marketplace', 'zk-proofs', 'privacy'],
};

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  return (
    <html lang="en">
      <body className="antialiased">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
