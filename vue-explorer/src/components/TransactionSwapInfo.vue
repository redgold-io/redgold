<template>
  <div class="table-container">
    <table class="table table-striped table-hover">
      <thead>
      <tr>
        <th>Property</th>
        <th>Value</th>
      </tr>
      </thead>
      <tbody>
      <tr>
        <td>Swap Status</td>
        <td>{{ swapInfo.swap_status }}</td>
      </tr>
      <tr>
        <td>Input Currency</td>
        <td>{{ swapInfo.input_currency }}</td>
      </tr>
      <tr>
        <td>Input Amount</td>
        <td>{{ swapInfo.swap_input_amount }}</td>
      </tr>
      <tr>
        <td>Input USD</td>
        <td>{{ swapInfo.swap_input_amount_usd }}</td>
      </tr>
      <tr>
        <td>Input USD Now</td>
        <td>{{ swapInfo.swap_input_amount_usd_now }}</td>
      </tr>
      <tr>
        <td>Output Currency</td>
        <td>{{ swapInfo.output_currency }}</td>
      </tr>
      <tr>
        <td>Output Amount</td>
        <td>{{ swapInfo.swap_output_amount }}</td>
      </tr>
      <tr>
        <td>Output USD</td>
        <td>{{ swapInfo.swap_output_amount_usd }}</td>
      </tr>
      <tr>
        <td>Output USD Now</td>
        <td>{{ swapInfo.swap_output_amount_usd_now }}</td>
      </tr>
      <tr>
        <td>Exchange Rate</td>
        <td>{{ swapInfo.swap_exchange_rate }}</td>
      </tr>
      <tr>
        <td>Party Address</td>
        <td>{{ swapInfo.party_address }}</td>
      </tr>
      <tr>
        <td>Swap Destination</td>
        <td>{{ swapInfo.exchange_rate }}</td>
      </tr>
      <tr>
        <td>Is Request</td>
        <td>{{ swapInfo.is_request }}</td>
      </tr>
      <tr>
        <td>Is Fulfillment</td>
        <td>{{ swapInfo.is_fulfillment }}</td>
      </tr>
      <tr>
        <td>Request TX Hash</td>
        <td>
          <HashLink v-if="swapInfo.request_tx_hash" :data="swapInfo.request_tx_hash" />
        </td>
      </tr>
      <tr>
        <td>Fulfillment TX Hash</td>
        <td>
          <HashLink v-if="swapInfo.fulfillment_tx_hash" :data="swapInfo.fulfillment_tx_hash" />
        </td>
      </tr>
      </tbody>
    </table>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
export default {
  name: "TransactionSwapInfo",
  components: {
    HashLink,
  },
  props: {
    swapInfo: Object,
  },
  data() {
    return {
    }
  }
}

</script>

<style>
.table .tr .td {
  color: #ffffff;
  background-color: #000000 !important;
}

.table-hover tbody tr:hover {
  color: #FFFFFF !important;
  background-color: #191a19 !important;
}

.table-striped>tbody>tr:nth-child(odd)>td,
.table-striped>tbody>tr:nth-child(odd)>th {
  background-color: #000000 !important;
  color: #ffffff;
}

.table-striped>tbody>tr:nth-child(even)>td,
.table-striped>tbody>tr:nth-child(even)>th {
  background-color: #191a19 !important;
  color: #ffffff;
}

.table-striped>tbody>tr:hover>td,
.table-striped>tbody>tr:hover>th {
  background-color: #291a00 !important; /* Set the desired color on hover */
  color: #ffffff;
}

th {
  background-color: #191a19 !important;
  color: #ffffff !important;
}
.table-container {
  width: auto;
  min-width: 300px;
  max-width: fit-content;
}

.table {
  width: auto;
  table-layout: fixed;
}

.table td:first-child {
  white-space: nowrap;
  padding-right: 2rem;
}

.table td:last-child {
  min-width: 200px;
  max-width: 400px;
  word-break: break-all;
}


</style>