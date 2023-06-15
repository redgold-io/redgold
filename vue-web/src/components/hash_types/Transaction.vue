<template>
  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">
        <div class="row">

          <!-- Transactions Table -->
          <div class="col-sm-8">
            <h4>Recent Transactions</h4>
          </div>

          <!-- Peers Table -->
          <div class="col-sm-4">
            <h4>Active Peers</h4>
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

// import axios from 'axios'
// import HashLink from './HashLink';

export default {
  name: 'Transaction',
  components: {
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
      transactionFields: [
        { key: 'hash' },
        { key: 'from' },
        { key: 'to' },
        { key: 'amount' },
        { key: 'fee' },
        { key: 'bytes' },
        { key: 'timestamp', formatter: (value) => {
            return new Date(value).toLocaleString(); // This line is assuming the timestamp is in milliseconds
          }},
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
}
</script>

<style scoped>

/*.table-container .table-hover .tbody .tr:hover .td .table {
  color: #fff;
//}

 */

.table {
  color: #ffffff;
}


</style>

<style>
.table-container .table-hover tbody tr:hover td {
  color: #FFFFFF;
  background-color: #191a19;
}
</style>
