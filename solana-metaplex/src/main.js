// TODO: Ended up not using this, used spl-token metadata instead

import { createUmi } from '@metaplex-foundation/umi-bundle-defaults';
import { mplTokenMetadata } from '@metaplex-foundation/mpl-token-metadata';
import { createMetadataAccountV3 } from '@metaplex-foundation/mpl-token-metadata';
import { publicKey } from '@metaplex-foundation/umi';

// Add console logs for debugging
console.log('Script starting...');

async function main() {
    try {
        console.log('Creating UMI instance...');
        const umi = createUmi('https://api.mainnet-beta.solana.com')
            .use(mplTokenMetadata());

        // If you're using a browser wallet like Phantom
        if (!window.solana) {
            throw new Error('Please install Phantom or another Solana wallet');
        }

        console.log('Requesting wallet connection...');
        await window.solana.connect();

        umi.use({
            install(umi) {
                umi.signers.add(window.solana);
            },
        });

        const tokenMint = publicKey('3MoLKvvL9ZqfGSxzCMr18XDZcTzRnZwS22R3qhAAPKEJ');

        console.log('Creating metadata transaction...');
        const tx = await createMetadataAccountV3(umi, {
            mint: tokenMint,
            mintAuthority: window.solana,
            data: {
                name: "WrappedRedgold",
                symbol: "wRDG",
                uri: "https://redgold-public.s3.us-west-1.amazonaws.com/token-metadata.json",
                sellerFeeBasisPoints: 0,
                creators: null,
                collection: null,
                uses: null
            },
            isMutable: true,
        });

        console.log('Sending transaction...');
        const result = await tx.sendAndConfirm();
        console.log('Transaction signature:', result.signature);

    } catch (error) {
        console.error('Error:', error);
        alert(`Error: ${error.message}`);
    }
}

// Add click handler to button
document.getElementById('createMetadata').addEventListener('click', main);