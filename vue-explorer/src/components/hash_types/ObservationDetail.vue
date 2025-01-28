<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
       <div class="col-10">


        <div class="grid-container">

        <div><strong>Hash</strong></div>
        <div><HashLink :data="hashDataInitial.hash" :shorten="false" /></div>

        <div><strong>Public Key</strong></div>
        <div><HashLink :data="hashDataInitial.public_key" :shorten="false" /></div>

        <div><strong>Time</strong></div>
        <div><RenderTime :timestamp="hashDataInitial.time" /></div>

        <div><strong>Height</strong></div>
        <div><strong>{{ hashDataInitial.height }}</strong></div>

        <div><strong>Parent Hash</strong></div>
        <div><HashLink :data="hashDataInitial.parent_hash" :shorten="false" /></div>

        <div><strong>Salt</strong></div>
        <div><strong>{{ hashDataInitial.salt }}</strong></div>

        <div><strong>Signable Hash</strong></div>
        <div class="hash-container">
          {{ hashDataInitial.signable_hash }}
          <div><CopyClipboard :data="hashDataInitial.signable_hash" /></div>
        </div>

        <div><strong>Signature</strong></div>
        <div class="signature">{{hashDataInitial.signature}}</div>

        <div><strong>Observations</strong></div>
        <div><ObservationMetadata :data="hashDataInitial.observations" /></div>
      </div>
    </div>
      <!-- Buffer div -->
      <div class="col-1"></div>


    </div>
  </div>
</template>
<script>
import HashLink from "@/components/util/HashLink.vue";
import CopyClipboard from "@/components/util/CopyClipboard.vue";
// import PeerNodeInfo from "@/components/PeerNodeInfo.vue";
import ObservationMetadata from "@/components/hash_types/ObservationMetadata.vue";
import RenderTime from "@/components/RenderTime.vue";

export default {
  name: "ObservationDetail",
  components: {
    RenderTime,
    CopyClipboard,
    HashLink,
    // PeerNodeInfo,
    ObservationMetadata
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