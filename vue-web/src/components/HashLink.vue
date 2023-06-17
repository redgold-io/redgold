<template>
  <a :href="`/hash/${data}`">{{ shortHash }}</a>
</template>

<script>
export default {
  props: ['data'],
  computed: {
    shortHash() {
      return this.data.substring(0, 4) + '...' + this.data.substring(this.data.length - 4);
    }
  }
}
</script>