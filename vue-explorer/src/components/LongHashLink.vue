<template>
  <div class="hash-container">
    <a :href="`/hash/${data}`">{{ data }}</a>
  </div>
</template>

<script>
export default {
  props: ['data'],
}
</script>