<template>
  <div class="hash-container">
    {{ data }}
    <div><CopyClipboard :data="data" /></div>
  </div>
</template>

<script>
import CopyClipboard from "@/components/util/CopyClipboard.vue";

export default {
  components: {CopyClipboard},
  props: {
    data: String
  }
}
</script>

<style scoped>

.hash-container {
  display: flex;
  align-items: center;
}

</style>