<template>
  <div>
    <!-- Display loading state while the data is being fetched -->
    <div v-if="loading">
      Loading...
    </div>

    <!-- Render the component dynamically once the data is fetched -->
    <component v-else :is="componentToRender" :hashDataInitial="hashData"></component>
  </div>
</template>

<script>
import TransactionDetail from './hash_types/TransactionDetail.vue';
import AddressDetail from './hash_types/AddressDetail.vue';
import NotFound from './hash_types/NotFound.vue';
// import HashTypeB from './HashTypeB.vue';
// import HashTypeC from './HashTypeC.vue';
import fetchHashInfo from './mixins/fetchHashInfo.js';
export default {
  name: 'HashDetails',
  data() {
    return {
      loading: true,
      hashData: "not loaded",
      componentToRender: null,
    };
  },
  components: {
    TransactionDetail,
    AddressDetail,
    NotFound,
  },
  mixins: [fetchHashInfo],
  async created() {

    await this.fetchData();
    // const hash = this.$route.params.param; // get the hash from the route parameter
    //
    // const response = await fetch(`http://localhost:16481/explorer/hash/${hash}`);
    // const data = await response.json();
    // console.log(data)
    // // console.log(JSON.stringify(data))
    // // console.log(Object.keys(data)); // Output: ["a", "b", "c"]
    // // determine which component to render based on the data
    // if (data.transaction != null) {
    //   this.hashData = data.transaction;
    //   this.componentToRender = 'TransactionDetail';
    // } else if (data.address != null) {
    //   this.hashData = data.address;
    //   this.componentToRender = 'AddressDetail';
    //   console.log("Loading address detail");
    // } else {
    //   this.hashData = hash;
    //   this.componentToRender = 'NotFound';
    // }
    //
    // this.loading = false;
    // // console.log("Logging address data directly");
    // // console.log(data.address);
    // console.log(JSON.stringify(this.hashData));

  },
};
</script>
