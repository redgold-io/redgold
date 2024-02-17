<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">

        <div class="grid-container">

          <div><strong>Peer Id</strong></div>
          <div><HashLink :data="hashDataInitial.peer_id" :shorten="false" /></div>

<!--          <div><strong>Public Key</strong></div>-->
<!--          <div class="hash-container">-->
<!--            {{ hashDataInitial.public_key }}-->
<!--            <div><CopyClipboard :data="hashDataInitial.public_key" /></div>-->
<!--          </div>-->

<!--          <div><strong>Signature</strong></div>-->
<!--          <div class="signature">{{hashDataInitial.signature}}</div>-->

          <div><strong>Nodes</strong></div>
          <div><PeerNodeInfo :hashDataInitial="hashDataInitial.nodes" /></div>


        </div>
      </div>
      <!-- Buffer div -->
      <div class="col-1"></div>
    </div>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
// import CopyClipboard from "@/components/util/CopyClipboard.vue";
import PeerNodeInfo from "@/components/hash_types/PeerNodeInfo.vue";
// import BriefObservation from "@/components/BriefObservation.vue";
export default {
  name: "PeerInfo",
  components: {
    // BriefObservation,
    // CopyClipboard,
    HashLink,
    PeerNodeInfo
  },
  props: {
    hashDataInitial: Object,
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