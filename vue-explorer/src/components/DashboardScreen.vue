<template>
  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <!-- Columns for transaction and peers -->
        <div class="row">

          <!-- Transactions Table -->
          <div class="col-sm-9">
            <h4>Recent Transactions</h4>
            <BriefTransaction :transactions="transactions"/>
          </div>

          <!-- Peers Table -->
          <div class="col-sm-3">
            <h4>Active Peers</h4>
<!--            <b-table striped hover :items="peers">-->
<!--              <template v-slot:cell(peerId)="data">-->
<!--                <a :href="`https://example.com/peers/${data.value}`">{{ data.value }}</a>-->
<!--              </template>-->
<!--            </b-table>-->
          </div>

        </div>


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

export default {
  name: 'DashboardScreen',
  components: {
    BriefTransaction,
    // BJumbotron,
    // BButton,
    // BForm,
    // BFormInput,
    // HashLink
    // BTable
  },
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
        },
      ],
      //   { key: 'transactionId', label: 'Transaction ID', formatter: value => `<a href="https://example.com/transactions/${value}">${value}</a>` },
      peers: [
        { peerId: 'P123', address: '192.168.1.1' },
      ],
      // peerFields: [
      //   { key: 'peerId', label: 'Peer ID', formatter: value => `<a href="https://example.com/peers/${value}">${value}</a>` },
      //   { key: 'address', label: 'Address' },
      // ]
    }
  },
  mounted() {

    let url = process.env.VUE_APP_API_URL;
    let port = "16486";

    url += ":" + port


    axios.get(`${url}/explorer`)
        .then(response => {
          console.log(response.data); // log the response data
          this.transactions = response.data['recent_transactions'];
        })
        .catch(error => {
          console.error(error);
        });
  }
}
</script>

<style>

</style>