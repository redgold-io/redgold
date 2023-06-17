<template>
  <button @click="copyToClipboard(data)" class="copy-button">
    <i class="copy-icon">📋</i> <!-- Replace this with an actual icon if you have one -->
  </button>
</template>

<script>
export default {
  props: {
    data: String,
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

.copy-button {
  padding: 0; /* Remove padding */
  border: none; /* Remove border */
  background: none; /* Remove default background */
  cursor: pointer; /* Optional: change cursor to hand icon on hover */
  margin-left: 10px;  /* Adjust as needed */
}


.copy-icon {
  font-size: 16px;  /* Adjust the size as needed */
}

</style>