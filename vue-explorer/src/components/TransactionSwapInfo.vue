<template>
  <div class="grid-container">
    <div><strong>Swap Status</strong></div>
    <div>{{ swapInfo.swap_status }}</div>

    <div><strong>Input Currency</strong></div>
    <div>{{ swapInfo.input_currency }}</div>

    <div><strong>Input Amount</strong></div>
    <div>{{ swapInfo.swap_input_amount }}</div>

    <div><strong>Input USD</strong></div>
    <div>{{ swapInfo.swap_input_amount_usd }}</div>

    <div><strong>Input USD Now</strong></div>
    <div>{{ swapInfo.swap_input_amount_usd_now }}</div>

    <div><strong>Output Currency</strong></div>
    <div>{{ swapInfo.output_currency }}</div>

    <div><strong>Output Amount</strong></div>
    <div>{{ swapInfo.swap_output_amount }}</div>

    <div><strong>Output USD</strong></div>
    <div>{{ swapInfo.swap_output_amount_usd }}</div>

    <div><strong>Output USD Now</strong></div>
    <div>{{ swapInfo.swap_output_amount_usd_now }}</div>

    <div><strong>Exchange Rate</strong></div>
    <div>{{ swapInfo.swap_exchange_rate }}</div>

    <div><strong>Party Address</strong></div>
    <div>{{ swapInfo.party_address }}</div>

    <div><strong>Swap Destination</strong></div>
    <div>{{ swapInfo.exchange_rate }}</div>

    <div><strong>Is Request</strong></div>
    <div>{{ swapInfo.is_request }}</div>

    <div><strong>Is Fulfillment</strong></div>
    <div>{{ swapInfo.is_fulfillment }}</div>

    <div><strong>Request TX Hash</strong></div>
    <div>
      <HashLink v-if="swapInfo.request_tx_hash" :data="swapInfo.request_tx_hash" />
    </div>

    <div><strong>Fulfillment TX Hash</strong></div>
    <div>
      <HashLink v-if="swapInfo.fulfillment_tx_hash" :data="swapInfo.fulfillment_tx_hash" />
    </div>
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
  }
}
</script>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: 1fr 6fr;
  gap: 10px;
  padding-top: 5px;
  padding-bottom: 5px;
  word-wrap: break-word;
}
</style>