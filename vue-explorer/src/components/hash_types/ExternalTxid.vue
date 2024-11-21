<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div>
          <h3 class="detail-group">External Transaction Details</h3>

          <div class="grid-container">

          </div>



<!--            <div><strong>Link</strong></div>-->
<!--            <div><HashLink :data="hashData.info.hash" /></div>-->

<!--            <div><strong>Hash</strong></div>-->
<!--            <div class="hash-container">-->
<!--              {{ hashData.info.hash }}-->
<!--              <div><CopyClipboard :data="hashData.info.hash" /></div>-->
<!--            </div>-->

<!--            <div><strong>Accepted</strong></div>-->
<!--            <div>{{ hashData.accepted }}</div>-->

<!--            <div><strong>From</strong></div>-->
<!--            <div><HashLink :data="hashData.info.from" :shorten="false" /></div>-->

<!--            <div><strong>To</strong></div>-->
<!--            <div><HashLink :data="hashData.info.to" :shorten="false" /></div>-->

<!--            <div><strong>First Amount</strong></div>-->
<!--            <div>{{ hashData.info.first_amount }} RDG</div>-->

<!--            <div><strong>Remainder Amount</strong></div>-->
<!--            <div>{{ hashData.remainder_amount }} RDG</div>-->

<!--            <div><strong>Total Amount</strong></div>-->
<!--            <div>{{ hashData.info.amount }} RDG</div>-->

<!--            <div><strong>Fee</strong></div>-->
<!--            <div>{{ hashData.info.fee }} sats RDG</div>-->

<!--            <div><strong>Bytes</strong></div>-->
<!--            <div>{{ hashData.info.bytes }}</div>-->

<!--            <div><strong>Time</strong></div>-->
<!--            <div><RenderTime :timestamp="hashData.info.timestamp" /></div>-->

<!--            <div><strong>Timestamp</strong></div>-->
<!--            <div>{{ hashData.info.timestamp }}</div>-->

<!--            <div><strong>Confirmation Score</strong></div>-->
<!--            <div>{{ hashData.confirmation_score }}</div>-->

<!--            <div><strong>Acceptance Score</strong></div>-->
<!--            <div>{{ hashData.acceptance_score }}</div>-->

<!--            <div><strong>Num Signers</strong></div>-->
<!--            <div>{{ hashData.signers.length }}</div>-->

<!--            <div v-if="hashData.message"><strong>Message</strong></div>-->
<!--            <div v-if="hashData.message">{{ hashData.message }}</div>-->

<!--            <div v-if="hashData.rejection_reason"><strong>Rejection Reason</strong></div>-->
<!--            <div v-if="hashData.rejection_reason">{{ hashData.rejection_reason }}</div>-->

<!--            <div><strong>Signable Hash</strong></div>-->
<!--            <div>{{ hashData.signable_hash }}</div>-->

<!--            <div><strong>Num Inputs</strong></div>-->
<!--            <div>{{ hashData.inputs.length }}</div>-->

<!--            <div><strong>Num Outputs</strong></div>-->
<!--            <div>{{ hashData.outputs.length }}</div>-->

<!--          </div>-->


          <div v-if="hashData.swap_info">
            <h3 class="detail-group">Swap Info</h3>
            <TransactionSwapInfo :swapInfo="hashData.swap_info" />
          </div>


<!--          <h3 class="detail-group">Transaction Data</h3>-->
<!--          <div class="grid-container">-->
<!--            <div><strong>Compact JSON</strong></div>-->
<!--            <div class="json-container">{{ hashData.raw_transaction }}</div>-->
<!--            <div><strong>Pretty JSON</strong></div>-->
<!--            <div><pre class="json-container">{{ prettyTransactionData }}</pre></div>-->

<!--          </div>-->

<!--          </div>-->

        </div>
        </div>
      </div>
    </div>
</template>


<script>

// import HashLink from "@/components/util/HashLink.vue";
// import CopyClipboard from "@/components/util/CopyClipboard.vue";
// import RenderTime from "@/components/RenderTime.vue";
import TransactionSwapInfo from "@/components/TransactionSwapInfo.vue";

export default {
  name: 'ExternalTxid',
  props: ['data'],
  components: {
    TransactionSwapInfo,
    // RenderTime,
    // HashLink,
    // CopyClipboard
  },
  data() {
    return {
      hashData: this.data
    }
  },
  computed: {
  }
}
</script>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: 1fr 6fr; /* Adjust as needed */
  gap: 10px; /* Adjust as needed */
  padding-top: 5px;
  padding-bottom: 5px;
  word-wrap: break-word; /* allows long words to be able to be broken and wrap onto the next line */
}

.hash-container {
  display: flex;
  align-items: center;
}
.detail-group {
  padding-top: 15px;
  padding-bottom: 15px;
  padding-left: 10px;
  background-color: #191a19 !important;
}
.signature {
  word-break: break-word;
  overflow-wrap: break-word;
}

.json-container {
  width: 80%; /* or any other appropriate width */
  word-break: break-word;
  overflow-wrap: break-word;
  font-size: smaller;
}

</style>