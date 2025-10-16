import { ethers } from 'ethers';
import * as fs from 'fs';
import * as path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

import * as dotenv from 'dotenv';
dotenv.config();

const RPC_URL = process.env.RPC_URL || 'https://testnet-passet-hub-eth-rpc.polkadot.io';
const PRIVATE_KEY = process.env.PRIVATE_KEY;

if (!PRIVATE_KEY) {
    console.error('Error: PRIVATE_KEY not set in .env file');
    process.exit(1);
}

const contracts = [
    { name: 'NightmarketZones', file: 'nightmarket_zones.polkavm' },
    { name: 'NightmarketListings', file: 'nightmarket_listings.polkavm' },
    { name: 'NightmarketMixer', file: 'nightmarket_mixer.polkavm' },
    { name: 'NightmarketEscrow', file: 'nightmarket_escrow.polkavm' },
    { name: 'NightmarketReputation', file: 'nightmarket_reputation.polkavm' },
];

async function deployAll() {
    console.log('='.repeat(60));
    console.log('Deploying All Nightmarket Contracts');
    console.log('='.repeat(60));
    console.log();

    const provider = new ethers.JsonRpcProvider(RPC_URL);
    const wallet = new ethers.Wallet(PRIVATE_KEY, provider);

    console.log('Deployer Address:', wallet.address);
    const balance = await provider.getBalance(wallet.address);
    console.log('Balance:', ethers.formatEther(balance), 'ETH');
    console.log();

    const deployments: any[] = [];

    for (const contractInfo of contracts) {
        console.log('-'.repeat(60));
        console.log(`Deploying ${contractInfo.name}...`);
        console.log('-'.repeat(60));

        const bytecodePath = path.join(__dirname, '../build', contractInfo.file);
        const bytecode = '0x' + fs.readFileSync(bytecodePath).toString('hex');

        console.log('Bytecode size:', (bytecode.length - 2) / 2, 'bytes');

        const factory = new ethers.ContractFactory([], bytecode, wallet);
        const contract = await factory.deploy();

        console.log('Transaction hash:', contract.deploymentTransaction()?.hash);

        await contract.waitForDeployment();
        const address = await contract.getAddress();

        console.log('✓ Deployed to:', address);
        console.log();

        deployments.push({
            contract: contractInfo.name,
            address: address,
            bytecodeHash: ethers.keccak256(bytecode),
        });

        // Wait a bit between deployments
        await new Promise(resolve => setTimeout(resolve, 2000));
    }

    // Save all deployments
    const allDeployments = {
        network: 'Paseo Asset Hub Testnet',
        chainId: 420420422,
        deployer: wallet.address,
        deployedAt: new Date().toISOString(),
        contracts: deployments,
    };

    const deploymentPath = path.join(__dirname, 'nightmarket_deployment.json');
    fs.writeFileSync(deploymentPath, JSON.stringify(allDeployments, null, 2));

    console.log('='.repeat(60));
    console.log('All Contracts Deployed Successfully!');
    console.log('='.repeat(60));
    console.log();
    console.log('Deployment Summary:');
    deployments.forEach(d => {
        console.log(`  ${d.contract.padEnd(25)} ${d.address}`);
    });
    console.log();
    console.log('Full deployment info saved to:', deploymentPath);
}

deployAll()
    .then(() => process.exit(0))
    .catch((error) => {
        console.error('Deployment failed:', error);
        process.exit(1);
    });
