<template>
  <div class="hash-container">
    <div v-if="this.useLink"><a :href=this.toLink>{{ displayedHash }}</a></div>
    <div v-if="!this.useLink">{{ displayedHash }}</div>
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
    isAddress: {
      type: Boolean,
      default: true
    },
    useLink: {
      type: Boolean,
      default: true
    },
    useExternalLink: {
      type: Boolean,
      default: false
    },
    link: {
      type: String,
      default: ''
    },
    trimPrefix: {
      type: Boolean,
      default: true
    }
  },
  computed: {
    toLink() {
      if (this.link) {
        return this.link;
      } else {
        let hashValue = this.data;
        let url = "explorer.redgold.io"
        const hostname = window.location.hostname;
        let main = hostname === url
        if (this.useExternalLink) {
          if (hashValue.startsWith('0x')) {
            let urlType = this.isAddress ? "address" : "tx"
            let prefix = main ? "" : "sepolia."
            return `https://${prefix}etherscan.io/${urlType}/${hashValue}`;
          }
          if (hashValue.startsWith('tb') || hashValue.startsWith('bc')) {
            let urlType = this.isAddress ? "address" : "tx"
            let prefix = main ? "" : "testnet/"
            return `https://blockstream.info/${prefix}${urlType}/${hashValue}`;
          }
        }
        return `/hash/${hashValue}`;
      }
    },
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
  }
}
</script>

<style scoped>

.hash-container {
  display: flex;
  align-items: center;
}

</style>