<template>
  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div><strong>Total Transactions {{total_transactions}}</strong></div>
        <div><strong>Total Peers {{num_active_peers}}</strong></div>

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
      observations: [],
      total_transactions: 0,
      num_active_peers: 0
    }
  },
  mounted() {

    let url = this.getUrl()
    // let url = process.env.VUE_APP_API_URL;
    // let port = "16486";
    //
    // url += ":" + port

    axios.get(`${url}/explorer`)
        .then(response => {
          let data = response.data;
          console.log(data); // log the response data
          this.transactions = data['recent_transactions'];
          // for (let i = 0; i < peers.length; i++) {
          //   peers[i].nodes = peers[i].nodes.slice(0, 1);
          // }
          this.peers = data['active_peers_abridged'];
          this.observations = data['recent_observations'];
          this.total_transactions = data.total_accepted_transactions
          this.num_active_peers = data.num_active_peers
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


</style>