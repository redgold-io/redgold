<template>
  <div class="hash-container">
    <a :href="`/hash/${data}`">{{ displayedHash }}</a>
    <CopyClipboard :data="data" />
  </div>
</template>

<script>
import CopyClipboard from "@/components/util/CopyClipboard.vue";

export default {
  components: {CopyClipboard},
  props: {
    data: String,
    shorten: {
      type: Boolean,
      default: true
    }
  },
  computed: {
    displayedHash() {
      if (this.shorten) {
        return this.data.substring(0, 4) + '...' + this.data.substring(this.data.length - 4);
      } else {
        return this.data;
      }
    }
  },
  methods: {
    copyToClipboard(text) {
      navigator.clipboard.writeText(text).then(() => {
        // Success feedback here
        console.log('Copying to clipboard was successful!');
      }, (err) => {
        // Error feedback here
        console.error('Could not copy text: ', err);
      });
    }
  }
}
</script>

<style scoped>

.hash-container {
  display: flex;
  align-items: center;
}

</style>