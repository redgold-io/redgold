<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div class="grid-container">

          <div><strong>Public Key</strong></div>
          <div><HashLink :data="hashDataInitial.public_key" :shorten="false" /></div>

          <div><strong>Host</strong></div>
          <div>{{hashDataInitial.external_address}}</div>

          <div><strong>Port</strong></div>
          <div>{{hashDataInitial.port_offset}}</div>
          <div><strong>Exe Checksum</strong></div>

          <div>{{shortenExeChecksum(hashDataInitial.executable_checksum)}}</div>

          <div><strong>XOR Distance</strong></div>
          <div>{{hashDataInitial.utxo_distance}}</div>

          <div><strong>Node Name</strong></div>
          <div>{{hashDataInitial.node_name}}</div>

          <div><strong>Peer Id</strong></div>
          <div><HashLink :data="hashDataInitial.peer_id" :shorten="false" /></div>

          <!-- TODO: Observations from this PK paginated / latest observation-->
        </div>

        <h4>Recent Observations</h4>
        <BriefObservation :data="hashDataInitial.recent_observations"/>


      </div>
      <!-- Buffer div -->
      <div class="col-1"></div>
    </div>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
// import CopyClipboard from "@/components/util/CopyClipboard.vue";
// import PeerNodeInfo from "@/components/hash_types/PeerNodeInfo.vue";
import {toTitleCase} from "@/utils";
import BriefObservation from "@/components/BriefObservation.vue";
export default {
  name: "PeerInfo",
  components: {
    BriefObservation,
    // CopyClipboard,
    HashLink,
    // PeerNodeInfo
  },
  props: {
    hashDataInitial: Object,
  },
  methods: {
    // Now you can use toTitleCase in this component
    formatTitle(key) {
      return toTitleCase(key);
    },
    shortenExeChecksum(h) {
      return h.substring(h.length - 8)
    }
  },
}

</script>

<style>
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