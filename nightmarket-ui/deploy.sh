#!/usr/bin/env bash

# Nightmarket UI Deployment Script for Vercel
# Deploys the frontend to Vercel with proper configuration

set -e

echo "🚀 Deploying Nightmarket UI to Vercel..."
echo

# Check if Vercel CLI is installed
if ! command -v vercel &> /dev/null; then
    echo "❌ Vercel CLI not found. Installing..."
    npm install -g vercel
fi

# Function to trim whitespace and newlines
trim() {
    echo "$1" | tr -d '\n\r' | sed 's/^[[:space:]]*//;s/[[:space:]]*$//'
}

# Function to set environment variables interactively
setup_env_vars() {
    echo "📝 Setting up environment variables..."
    echo

    # Read and trim each variable
    read -p "RPC URL (default: https://testnet-passet-hub-eth-rpc.polkadot.io): " rpc_url
    rpc_url=$(trim "${rpc_url:-https://testnet-passet-hub-eth-rpc.polkadot.io}")

    read -p "Chain ID (default: 420420422): " chain_id
    chain_id=$(trim "${chain_id:-420420422}")

    echo
    echo "Contract Addresses (paste from deployment output):"
    read -p "Zones Contract: " zones_contract
    zones_contract=$(trim "$zones_contract")

    read -p "Listings Contract: " listings_contract
    listings_contract=$(trim "$listings_contract")

    read -p "Escrow Contract: " escrow_contract
    escrow_contract=$(trim "$escrow_contract")

    read -p "Mixer Contract: " mixer_contract
    mixer_contract=$(trim "$mixer_contract")

    read -p "Reputation Contract: " reputation_contract
    reputation_contract=$(trim "$reputation_contract")

    # Create .env.local file
    cat > .env.local << EOF
# Nightmarket UI Environment Variables
# Generated: $(date)

NEXT_PUBLIC_RPC_URL=${rpc_url}
NEXT_PUBLIC_CHAIN_ID=${chain_id}
NEXT_PUBLIC_ZONES_CONTRACT=${zones_contract}
NEXT_PUBLIC_LISTINGS_CONTRACT=${listings_contract}
NEXT_PUBLIC_ESCROW_CONTRACT=${escrow_contract}
NEXT_PUBLIC_MIXER_CONTRACT=${mixer_contract}
NEXT_PUBLIC_REPUTATION_CONTRACT=${reputation_contract}
NEXT_PUBLIC_ENABLE_MIXER=true
NEXT_PUBLIC_ENABLE_ESCROW=true
NEXT_PUBLIC_DEBUG_MODE=false
EOF

    echo
    echo "✅ Environment variables saved to .env.local"
    echo
}

# Check if .env.local exists or offer to create it
if [ ! -f ".env.local" ]; then
    echo "⚠️  No .env.local found"
    echo
    read -p "Would you like to set up environment variables now? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        setup_env_vars
    else
        echo "Creating from template..."
        cp .env.example .env.local
        echo "📝 Please edit .env.local with your contract addresses before deploying"
        echo
        read -p "Press enter to continue or Ctrl+C to abort..."
    fi
else
    # .env.local exists, ask if user wants to update
    echo "✅ Found existing .env.local"
    echo
    read -p "Would you like to update environment variables? (y/N): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]]; then
        setup_env_vars
    fi
fi

# Check if circuit artifacts exist
if [ ! -d "public/circuits" ] || [ -z "$(ls -A public/circuits 2>/dev/null)" ]; then
    echo "⚠️  Circuit artifacts not found in public/circuits/"
    echo
    echo "Copying circuit artifacts from ../circuits/build/..."

    if [ -d "../circuits/build" ]; then
        mkdir -p public/circuits

        # Copy WASM files
        cp ../circuits/build/location_proof_js/location_proof.wasm public/circuits/ 2>/dev/null || true
        cp ../circuits/build/mixer_withdrawal_js/mixer_withdrawal.wasm public/circuits/ 2>/dev/null || true
        cp ../circuits/build/reputation_threshold_js/reputation_threshold.wasm public/circuits/ 2>/dev/null || true

        # Copy zkey files
        cp ../circuits/build/location_proof.zkey public/circuits/ 2>/dev/null || true
        cp ../circuits/build/mixer_withdrawal.zkey public/circuits/ 2>/dev/null || true
        cp ../circuits/build/reputation_threshold.zkey public/circuits/ 2>/dev/null || true

        echo "✅ Circuit artifacts copied"
    else
        echo "❌ Circuit build directory not found at ../circuits/build/"
        echo "   Run: cd ../circuits && ./build.sh"
        echo
        read -p "Continue deployment without circuits? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            exit 1
        fi
    fi
fi

# Show deployment info
echo
echo "📊 Deployment Configuration:"
echo "  Framework: Next.js 15"
echo "  Build Command: npm run build"
echo "  Output Directory: .next"
echo "  Node Version: 20.x"
echo

# Check if production or preview
if [ "$1" == "--production" ] || [ "$1" == "-p" ]; then
    DEPLOY_CMD="vercel --prod"
    echo "🔴 Deploying to PRODUCTION"
else
    DEPLOY_CMD="vercel"
    echo "🟡 Deploying to PREVIEW"
    echo "   (Use --production flag for production deployment)"
fi

echo
read -p "Continue? (y/N) " -n 1 -r
echo
if [[ ! $REPLY =~ ^[Yy]$ ]]; then
    echo "Deployment cancelled"
    exit 0
fi

# Ask if user wants to set Vercel environment variables
if [ "$1" == "--production" ] || [ "$1" == "-p" ]; then
    echo
    read -p "Would you like to set environment variables in Vercel? (Y/n): " -n 1 -r
    echo
    if [[ $REPLY =~ ^[Yy]$ ]] || [[ -z $REPLY ]]; then
        echo
        echo "📝 Setting Vercel environment variables..."
        echo "   (Reading from .env.local)"
        echo

        # Read variables from .env.local and set in Vercel
        if [ -f ".env.local" ]; then
            # Extract and set each variable
            while IFS='=' read -r key value; do
                # Skip comments and empty lines
                [[ $key =~ ^#.*$ ]] && continue
                [[ -z $key ]] && continue

                # Trim key and value
                key=$(trim "$key")
                value=$(trim "$value")

                # Only set NEXT_PUBLIC_ variables
                if [[ $key == NEXT_PUBLIC_* ]]; then
                    echo "  Setting $key..."
                    echo "$value" | vercel env add "$key" production --force 2>/dev/null || true
                fi
            done < .env.local

            echo
            echo "✅ Environment variables set in Vercel"
        else
            echo "⚠️  .env.local not found, skipping Vercel env setup"
        fi
    fi
fi

# Run Vercel deployment
echo
echo "🚀 Starting Vercel deployment..."
echo

$DEPLOY_CMD

echo
echo "✅ Deployment complete!"
echo
echo "📋 Deployment checklist:"
if [ "$1" != "--production" ] && [ "$1" != "-p" ]; then
    echo "  1. This is a PREVIEW deployment"
    echo "  2. Test all features at the provided URL"
    echo "  3. If everything works, deploy to production with:"
    echo "     ./deploy.sh --production"
else
    echo "  1. ✅ Environment variables set in Vercel"
    echo "  2. Test the deployment at the provided URL"
    echo "  3. Verify all contract interactions work"
    echo "  4. Check ZK proof generation (may take 5-10s)"
    echo "  5. Verify listings encrypt/decrypt properly"
fi
echo
