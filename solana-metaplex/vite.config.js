import {defineConfig} from 'vite';
import path from 'path';

export default defineConfig({
    resolve: {
        alias: {
            'rpc-websockets': path.resolve(__dirname, 'node_modules/rpc-websockets/dist/lib/client/websocket.browser.js')
        }
    },
    optimizeDeps: {
        include: [
            '@metaplex-foundation/mpl-token-metadata',
            '@metaplex-foundation/umi',
            '@metaplex-foundation/umi-bundle-defaults',
            '@solana/web3.js'
        ],
        esbuildOptions: {
            define: {
                global: 'globalThis'
            }
        }
    },
    define: {
        'process.env': {},
        global: 'globalThis'
    },
    build: {
        rollupOptions: {
            external: ['rpc-websockets'],
            output: {
                globals: {
                    'rpc-websockets': 'RpcWebsockets'
                }
            }
        }
    }
});