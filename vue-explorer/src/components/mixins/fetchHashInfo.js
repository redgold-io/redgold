export default {
    methods: {
        getUrl() {
            let url = process.env.VUE_APP_API_URL
            let port = "16186"
            const hostname = window.location.hostname;

            if (hostname.includes('staging')) {
                port = "16386";
            } else if (hostname.includes('test')) {
                port = "16286";
            } else if (hostname.includes('main')) {
                port = "16186";
            } else { // default dev
                port = "16486"
            }
            url += ":" + port
            return url
        },
        async btcUsdPrice() {
            const url = "https://api.coinbase.com/v2/exchange-rates?currency=BTC"
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
        async fetchData(url= null, port= null, offset = null, limit = null) {
            const hash = this.$route.params.param; // get the hash from the route parameter

            if (url == null) {
                url = process.env.VUE_APP_API_URL
            }

            if (port == null) {
                const hostname = window.location.hostname;
                if (hostname.includes('staging')) {
                    port = "16386";
                } else {
                    port = "16486"
                }
            }

            url += ":" + port


            let input = `${url}/explorer/hash/${hash}`;

            // Add offset and limit as query parameters if they are present
            let params = new URLSearchParams();
            if (offset !== null) {
                params.append('offset', offset);
            }
            if (limit !== null) {
                params.append('limit', limit);
            }
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
            } else {
                this.hashData = hash;
                this.componentToRender = 'NotFound';
            }

            this.loading = false;
            // console.log(JSON.stringify(this.hashData));
        },
    },
};