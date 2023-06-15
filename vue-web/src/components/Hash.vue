// Hash.vue

<template>
  <div>
    <!-- Display loading state while the data is being fetched -->
    <div v-if="loading">
      Loading...
    </div>

    <!-- Render the component dynamically once the data is fetched -->
<!--    <component v-else :is="componentToRender" :hashData="hashData"></component>-->
  </div>
</template>

<script>
// import Transaction from './hash_types/Transaction.vue';
// import HashTypeB from './HashTypeB.vue';
// import HashTypeC from './HashTypeC.vue';

export default {
  data() {
    return {
      loading: true,
      hashData: null,
      componentToRender: null,
    };
  },
  components: {
    // Transaction,
  },
  // async created() {
  //   const hash = this.$route.params.hash; // get the hash from the route parameter
  //
  //   const response = await fetch(`http://localhost:16481/explorer/hash/${hash}`);
  //   const data = await response.json();
  //
  //   this.hashData = data;
  //
  //   // determine which component to render based on the data
  //   if ("transaction" in data) {
  //     this.componentToRender = 'Transaction';
  //   }
  //
  //   this.loading = false;
  // },
};
</script>
