export default {
    methods: {
        getUrl() {
            let url = "https://api.redgold.io"
            const hostname = window.location.hostname;

            if (hostname.includes('dev') || hostname.includes('localhost')) {
                url = "https://dev.api.redgold.io"
            } else if (hostname.includes('staging')) {
                url = "https://staging.api.redgold.io"
            } else if (hostname.includes('test')) {
                url = "https://test.api.redgold.io"
            }

            return url
        },
        isMainnet() {
            const hostname = window.location.hostname;
            return hostname.includes("explorer.redgold.io")
        },
        async btcUsdPrice() {
            const url = "https://api.coinbase.com/v2/exchange-rates?currency=BTC"
            const response = await fetch(url);
            const data = await response.json();
            console.log(data);
            return Number(data.data.rates.USD)
        },
        async ethUsdPrice() {
            const url = "https://api.coinbase.com/v2/exchange-rates?currency=ETH"
            const response = await fetch(url);
            const data = await response.json();
            console.log(data);
            return Number(data.data.rates.USD)
        },
        async fetchSwapInfo() {
            try {
                let url = this.getUrl()
                let input = `${url}/explorer/swap`;
                const response = await fetch(input);

                if (!response.ok) {
                    throw new Error(`HTTP error! status: ${response.status}`);
                }

                const data = await response.json();
                console.log(data)
                return data
            } catch (error) {
                console.error('An error occurred:', error);
                return null;
            }
        },
        async fetchData(offset = null, limit = null) {
            const hash = this.$route.params.param; // get the hash from the route parameter

            let url = this.getUrl()


            let input = `${url}/explorer/hash/${hash}`;

            // Add offset and limit as query parameters if they are present
            let params = new URLSearchParams();
            if (offset == null) {
                offset = 0;
            }
            params.append('offset', offset);
            if (limit == null) {
                limit = 25;
            }
            params.append('limit', limit);
            if (params.toString()) {
                input += `?${params.toString()}`;
            }

            const response = await fetch(input);
            const data = await response.json();
            console.log(data)
            // console.log(JSON.stringify(data))
            // console.log(Object.keys(data)); // Output: ["a", "b", "c"]
            // determine which component to render based on the data
            if (data.transaction != null) {
                this.hashData = data.transaction;
                this.componentToRender = 'TransactionDetail';
            } else if (data.address != null) {
                this.hashData = data.address;
                this.componentToRender = 'AddressDetail';
                console.log("Loading address detail");
            } else if (data.observation != null) {
                this.hashData = data.observation;
                this.componentToRender = 'ObservationDetail';
                console.log("Loading observation detail");
            } else if (data.peer != null) {
                this.hashData = data.peer;
                this.componentToRender = 'PeerInfo';
                console.log("Loading PeerInfo detail");
            } else if (data.peer_node != null) {
                this.hashData = data.peer_node;
                this.componentToRender = 'PeerNodeDetail';
                console.log("Loading PeerNodeDetail detail");
            } else if (data.external_txid_info != null) {
                this.hashData = data.peer_node;
                this.componentToRender = 'ExternalTxid';
                console.log("Loading ExternalTxid detail");
            } else {
                this.hashData = hash;
                this.componentToRender = 'NotFound';
            }

            this.loading = false;
            // console.log(JSON.stringify(this.hashData));
        },
    },
};