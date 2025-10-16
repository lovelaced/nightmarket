# Nightmarket UI

Anonymous decentralized marketplace frontend. Dark, minimal, artsy interface for privacy-preserving commerce during night hours (2:00-5:00 AM).

## Features

- **Time-restricted access**: Only operates during night hours
- **Location proofs**: Zero-knowledge proof generation
- **Encrypted listings**: AES-256-GCM encryption with zone keys
- **Wallet integration**: RainbowKit with Paseo Asset Hub
- **Dark minimal UI**: Inspired by ephemeral fragments aesthetic
- **Real-time updates**: Auto-refresh for active listings

## Setup

```bash
npm install
cp .env.local.example .env.local
# Edit .env.local with contract addresses and Wallet Connect project ID
npm run dev
```

Open [http://localhost:3000](http://localhost:3000)

## Tech Stack

- **Next.js 15** - React framework
- **TypeScript** - Type safety
- **Tailwind CSS 4** - Styling
- **RainbowKit** - Wallet connection
- **Wagmi** - Ethereum hooks
- **Ethers.js** - Contract interactions
- **SnarkJS** - ZK proof generation

## Theme

Minimal dark interface with:
- Pure black background (#000000)
- White text with varying opacities
- Moonlight blue accents (#6b8cff)
- Glass morphism effects
- Smooth animations (fade, emerge, pulse)
- Monospace fonts for data
- Sans-serif for UI (Inter)
- Serif for emphasis (Crimson Text)

## Components

- **NightStatus**: Real-time night/day indicator
- **LocationProof**: ZK proof generation interface
- **ListingsBrowser**: Grid view of active listings
- **ListingCard**: Individual listing display
- **CreateListing**: Form to create encrypted listings

## Contract Integration

All 5 Nightmarket contracts:
- NightmarketZones - Location verification
- NightmarketListings - Listing management
- NightmarketMixer - Privacy layer
- NightmarketEscrow - Trade coordination
- NightmarketReputation - Anonymous scores

## Development

```bash
npm run dev      # Start development server
npm run build    # Production build
npm run start    # Start production server
```

## License

MIT
