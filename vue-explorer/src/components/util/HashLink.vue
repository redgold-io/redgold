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
    },
    trimPrefix: {
      type: Boolean,
      default: true
    }
  },
  computed: {
    postTrim() {
      let excludePrefixes = ['0a220a20', '0a230a2103', '0a230a2102']
      if (this.trimPrefix) {
        for (let pfx of excludePrefixes) {
          if (this.data.startsWith(pfx)) {
            return this.data.substring(pfx.length);
          }
        }
      }
      return this.data;
    },
    shortened() {
      let d = this.postTrim;
      if (this.shorten) {
        return d.substring(0, 4) + '...' + d.substring(d.length - 4);
      } else {
        return d;
      }
    },
    displayedHash() {
      return this.shortened;
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