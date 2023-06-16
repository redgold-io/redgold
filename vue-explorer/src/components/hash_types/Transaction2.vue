<template>
  <div>
    <h1>Transaction Details</h1>
    <div class="grid-container">
      <div>
        <strong>Transaction ID: </strong>
        <span>{{ hashData.hash }}</span>
      </div>
      <div>
        <strong>Timestamp: </strong>
        <span>{{ hashData.transaction.timestamp }}</span>
      </div>
      <div>
        <strong>Value: </strong>
        <span>{{ hashData.transaction.value }}</span>
      </div>
      <!-- Include more fields as needed -->
    </div>
  </div>
</template>

<script>
export default {
  props: ['hashData'],
};
</script>

<style scoped>
.grid-container {
  display: grid;
  grid-template-columns: 1fr 1fr; /* Adjust as needed */
  gap: 10px; /* Adjust as needed */
}
</style>