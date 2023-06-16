export default {
    methods: {
        async fetchData(url) {
            const hash = this.$route.params.param; // get the hash from the route parameter

            const response = await fetch(`${url}/explorer/hash/${hash}`);
            const data = await response.json();
            // console.log(data)
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
            } else {
                this.hashData = hash;
                this.componentToRender = 'NotFound';
            }

            this.loading = false;
            // console.log(JSON.stringify(this.hashData));
        },
    },
};