<template>

  <div class="container-fluid">

    <div class="row no-gutters">
      <!-- Buffer div -->
      <div class="col-1"></div>

      <!-- Main content div -->
      <div class="col-10">
        <div>
          <h3 class="detail-group">Hash not found</h3>
          <div class="hash-container">
            <div>Hash: {{ hashData }}</div>
            <div><CopyClipboard :data="hashData" :shorten="false" /></div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>


<script>


import CopyClipboard from "@/components/util/CopyClipboard.vue";

export default {
  name: 'NotFound',
  props: ['hashDataInitial'],
  components: {
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

.hash-container {
  display: flex;
  align-items: center;
  padding-left: 10px;
}

</style>