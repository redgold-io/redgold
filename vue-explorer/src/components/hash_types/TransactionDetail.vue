<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div>
          <h3 class="detail-group">Transaction Details</h3>
          <div class="grid-container">
            <div><strong>Link</strong></div>
            <div><HashLink :data="hashData.info.hash" /></div>

            <div><strong>Hash</strong></div>
            <div class="hash-container">
              {{ hashData.info.hash }}
              <div><CopyClipboard :data="hashData.info.hash" /></div>
            </div>

            <div><strong>Accepted</strong></div>
            <div>{{ hashData.accepted }}</div>

            <div><strong>From</strong></div>
            <div><HashLink :data="hashData.info.from" :shorten="false" /></div>

            <div><strong>To</strong></div>
            <div><HashLink :data="hashData.info.to" :shorten="false" /></div>

            <div><strong>First Amount</strong></div>
            <div>{{ hashData.info.first_amount }} RDG</div>

            <div><strong>Total Amount</strong></div>
            <div>{{ hashData.info.amount }} RDG</div>

            <div><strong>Fee</strong></div>
            <div>{{ hashData.info.fee }} RDG</div>

            <div><strong>Bytes</strong></div>
            <div>{{ hashData.info.bytes }}</div>

            <div><strong>Time</strong></div>
            <div><RenderTime :timestamp="hashData.info.timestamp" /></div>

            <div><strong>Timestamp</strong></div>
            <div>{{ hashData.info.timestamp }}</div>

            <div><strong>Confirmation Score</strong></div>
            <div>{{ hashData.confirmation_score }}</div>

            <div><strong>Acceptance Score</strong></div>
            <div>{{ hashData.acceptance_score }}</div>

            <div><strong>Message</strong></div>
            <div>{{ hashData.message }}</div>

            <div><strong>Rejection Reason</strong></div>
            <div>{{ hashData.rejection_reason }}</div>

            <div><strong>Signable Hash</strong></div>
            <div>{{ hashData.signable_hash }}</div>

          </div>
          <h3 class="detail-group">Inputs</h3>
          <div v-for="(input, index) in hashData.inputs" :key="index">
            <div class="grid-container">
              <div><strong>Input {{ index }}</strong></div>
              <div class="grid-container">
                <div><strong>Transaction Hash</strong></div>
                <div><HashLink :data="input.transaction_hash" :shorten="false" /></div>
                <div><strong>Output Index</strong></div>
                <div><strong>{{ input.output_index }}</strong></div>
                <div><strong>Address</strong></div>
                <div><HashLink :data="input.address" :shorten="false" /></div>
              </div>
            </div>
          </div>

          <h3 class="detail-group">Outputs</h3>
          <div v-for="(output, index) in hashData.outputs" :key="index">
            <div class="grid-container">
              <div><strong>Output {{ index }}</strong></div>
              <div class="grid-container">
                <div><strong>Address</strong></div>
                <div><HashLink :data="output.address" :shorten="false" /></div>
                <div><strong>Amount</strong></div>
                <div><strong>{{ output.amount }}</strong></div>
                <div><strong>Available</strong></div>
                <div><strong>{{ output.available }}</strong></div>
              </div>
            </div>
          </div>


          <h3 class="detail-group">Signers</h3>
          <div v-for="(signer, index) in hashData.signers" :key="index">
            <div class="grid-container">
              <div>
                <div><strong>Signer {{ index }}</strong></div>
                <div><HashLink :data="signer.peer_id" /></div>
                <div><strong>Rating {{ signer.trust }} / 10</strong></div>
              </div>
              <div v-for="(signer, index) in signer.nodes" :key="index">
                <div class="grid-container">
                  <div><strong>Node Public Key</strong></div>
                  <div><HashLink :data="signer.node_id" :shorten="false" /></div>
                  <div><strong>Signature</strong></div>
                  <div class="signature">{{signer.signature}}</div>

                  <div><strong>Pending Time</strong></div>
                  <div><RenderTime :timestamp="signer.signed_pending_time" /></div>

                  <div><strong>Finalized Time</strong></div>
                  <div><RenderTime :timestamp="signer.signed_finalized_time" /></div>

                  <div><strong>Observation Hash</strong></div>
                  <div><HashLink :data="signer.observation_hash" :shorten="false" /></div>

                  <div><strong>Observation Type</strong></div>
                  <div><strong>{{ signer.observation_type }}</strong></div>


<!--                  <div><strong>Time</strong></div>-->
<!--                  <div><RenderTime :timestamp="signer.observation_timestamp" /></div>-->

                  <div><strong>Validation Confidence</strong></div>
                  <div><strong>{{ signer.validation_confidence_score }} / 10</strong></div>

                </div>
              </div>
            </div>
          </div>


<!--            <div><strong>Signers</strong></div>-->
<!--            <div>-->
<!--              <ul>-->
<!--                <li v-for="(signer, index) in hashData.signers" :key="index">-->
<!--                  Signature: {{ signer.signature }} <br>-->
<!--                  Node ID: {{ signer.node_id }} <br>-->
<!--                  Trust: {{ signer.trust }}-->
<!--                </li>-->
<!--              </ul>-->
<!--            </div>-->

<!--            <div><strong>Outputs</strong></div>-->
<!--            <div>-->
<!--              <ul>-->
<!--                <li v-for="(output, index) in hashData.outputs" :key="index">-->
<!--                  Output Index: {{ output.output_index }} <br>-->
<!--                  Address: {{ output.address }} <br>-->
<!--                  Available: {{ output.available }} <br>-->
<!--                  Amount: {{ output.amount }}-->
<!--                </li>-->
<!--              </ul>-->
<!--            </div>-->

          </div>

        </div>
      </div>
    </div>
</template>


<script>

import HashLink from "@/components/util/HashLink.vue";
import CopyClipboard from "@/components/util/CopyClipboard.vue";
import RenderTime from "@/components/RenderTime.vue";

export default {
  name: 'TransactionDetail',
  props: ['hashDataInitial'],
  components: {
    RenderTime,
    HashLink,
    CopyClipboard
  },
  data() {
    return {
      hashData: this.hashDataInitial
    }
  },
}
</script>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: 1fr 6fr; /* Adjust as needed */
  gap: 10px; /* Adjust as needed */
  padding-top: 5px;
  padding-bottom: 5px;
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

</style>