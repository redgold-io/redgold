<template>
  <div class="time">
    <div class="elapsed">{{getTimeElapsed()}}</div>
    <div class="full-date" :data="timestamp">
    {{ new Date(timestamp).toLocaleString() }}
    </div>
  </div>
</template>

<script>
export default {
  props: {
    timestamp: Number,
  },
  methods: {
    getTimeElapsed() {
      const timestamp = this.timestamp;
      const now = Date.now();
      const diff = now - timestamp; // Difference in milliseconds

      const seconds = Math.floor(diff / 1000);
      if (seconds < 60) return `${seconds} seconds ago`;

      const minutes = Math.floor(seconds / 60);
      if (minutes < 60) return `${minutes} minutes ago`;

      const hours = Math.floor(minutes / 60);
      if (hours < 24) return `${hours} hours ago`;

      const days = Math.floor(hours / 24);
      return `${days} days ago`;
    }
  }
}
</script>

<style scoped>


.time {
  display: flex;
  align-items: center;
}

.elapsed {
  padding-right: 10px;
}

</style>