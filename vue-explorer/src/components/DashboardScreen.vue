<template>
  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div class="stats-container">
          <div><strong>Peers: {{this.getFieldValue('num_active_peers')}}</strong></div>
          <div><strong>Transactions: {{this.getFieldValue('total_accepted_transactions')}}</strong></div>
          <div><strong>UTXOs: {{this.getFieldValue('total_accepted_utxos')}}</strong></div>
          <div><strong>Observations: {{this.getFieldValue('total_accepted_observations')}}</strong></div>
          <div><strong>Distinct UTXO Address: {{this.getFieldValue('total_distinct_utxo_addresses')}}</strong></div>
          <div><strong>Transaction Size: {{this.getFieldValue('size_transactions_gb')}} GB</strong></div>
          <div><strong>UTXOs Size: {{this.getFieldValue('size_utxos_gb')}} GB</strong></div>
          <div><strong>Observation Size: {{this.getFieldValue('size_observations_gb')}} GB</strong></div>
        </div>
        <h4>Recent Transactions</h4>
        <BriefTransaction :transactions="transactions"/>

        <h4>Recent Observations</h4>
        <BriefObservation :data="observations"/>

        <h4>Active Peers</h4>
        <BriefPeer :data="peers"/>

      </div>

      <!-- Buffer div -->
      <div class="col-1"></div>

    </div>
  </div>
</template>


<script>
// import { BJumbotron, BButton, BForm, BFormInput, BTable } from 'bootstrap-vue'
// import { BJumbotron, BButton, BForm, BFormInput } from 'bootstrap-vue'

import axios from 'axios'
// import HashLink from './HashLink';
import BriefTransaction from "@/components/BriefTransaction.vue";
// import PeerInfo from "@/components/hash_types/PeerInfo.vue";
// import ObservationDetail from "@/components/ObservationDetail.vue";
import BriefObservation from "@/components/BriefObservation.vue";
import BriefPeer from "@/components/BriefPeer.vue";
import fetchHashInfo from "@/components/mixins/fetchHashInfo";

export default {
  name: 'DashboardScreen',
  components: {
    BriefObservation,
    BriefTransaction,
    BriefPeer,
    // ObservationDetail
    // BJumbotron,
    // BButton,
    // BForm,
    // BFormInput,
    // HashLink
    // BTable
  },
  mixins: [fetchHashInfo],
  data() {
    return {
      data: {
        transactions: [
          {
            hash: 'fc5159bd8626cf2b19c4d3ed6395fef2ba04a5fb062b64d333224afeabd3b9f9',
            from: 'fc5159bd8626cf2b19c4d3ed6395fef2ba04a5fb062b64d333224afeabd3b9f9',
            to: 'fc5159bd8626cf2b19c4d3ed6395fef2ba04a5fb062b64d333224afeabd3b9f9',
            amount: 50,
            fee: 0.0,
            bytes: 400,
            timestamp: 1580000000,
            first_amount: 1,
          },
        ],
        peers: [],
        observations: []
      }
    }
  },
  methods: {
    getFieldValue(field) {
      const value = this.data && this.data.data ? this.data.data[field] : '';
      // Check if the value is a number and not an integer (thus, a float)
      if (typeof value === 'number' && !Number.isInteger(value)) {
        // Format to a maximum of 5 decimal places and remove trailing zeros
        return parseFloat(value.toFixed(5));
      }
      return value;
    }
  },
  mounted() {

    let url = this.getUrl()
    axios.get(`${url}/explorer`)
        .then(response => {
          let data = response.data;
          this.data.data = data;
          console.log(data); // log the response data
          this.transactions = data['recent_transactions'];
          this.peers = data['active_peers_abridged'];
          this.observations = data['recent_observations'];
        })
        .catch(error => {
          console.error(error);
        });
  }
}
</script>

<style>

h4 {
  margin-top: 20px;
  margin-bottom: 20px;
}

.stats-container {
  display: grid;
  grid-template-columns: 1fr 1fr 1fr 1fr; /* Adjust as needed */
  gap: 10px; /* Adjust as needed */
  padding-top: 5px;
  padding-bottom: 5px;
  word-wrap: break-word; /* allows long words to be able to be broken and wrap onto the next line */
}


</style>