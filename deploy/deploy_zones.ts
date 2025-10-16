import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Load environment variables
import * as dotenv from 'dotenv';
dotenv.config();

const RPC_URL = process.env.RPC_URL || 'https://testnet-passet-hub-eth-rpc.polkadot.io';
const PRIVATE_KEY = process.env.PRIVATE_KEY;

if (!PRIVATE_KEY) {
    console.error('Error: PRIVATE_KEY not set in .env file');
    process.exit(1);
}

// Minimal ABI for deployment
const ABI = [
    "function initialize()",
    "function addZone(uint32,int32,int32,int32,int32)",
    "function updateFingerprint(uint32,bytes32)",
    "function isNightTime() view returns (bool)",
    "function getZoneCount() view returns (uint256)"
];

async function deployZonesContract() {
    console.log('Deploying NightmarketZones Contract...\n');

    // Connect to network
    const provider = new ethers.JsonRpcProvider(RPC_URL);
    const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

    console.log('Deployer Address:', wallet.address);
    const balance = await provider.getBalance(wallet.address);
    console.log('Balance:', ethers.formatEther(balance), 'ETH\n');

    // Load bytecode
    const bytecodePath = path.join(__dirname, '../build/nightmarket_zones.polkavm');
    const bytecode = '0x' + fs.readFileSync(bytecodePath).toString('hex');

    console.log('Bytecode size:', (bytecode.length - 2) / 2, 'bytes');

    // Deploy contract
    console.log('\nDeploying contract...');
    const factory = new ethers.ContractFactory(ABI, bytecode, wallet);
    const contract = await factory.deploy();

    console.log('Transaction hash:', contract.deploymentTransaction()?.hash);
    console.log('Waiting for confirmation...');

    await contract.waitForDeployment();
    const address = await contract.getAddress();

    console.log('\n✓ NightmarketZones deployed!');
    console.log('Contract Address:', address);

    // Save deployment info
    const deployment = {
        contract: 'NightmarketZones',
        address: address,
        deployer: wallet.address,
        network: 'Paseo Asset Hub Testnet',
        chainId: 420420422,
        deployedAt: new Date().toISOString(),
        bytecodeHash: ethers.keccak256(bytecode),
    };

    const deploymentPath = path.join(__dirname, `nightmarket_zones_deployment.json`);
    fs.writeFileSync(deploymentPath, JSON.stringify(deployment, null, 2));
    console.log('\nDeployment info saved to:', deploymentPath);

    return address;
}

deployZonesContract()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error('Deployment failed:', error);
        process.exit(1);
    });
