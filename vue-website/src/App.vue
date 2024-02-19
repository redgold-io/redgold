<template>
  <div id="app">
    <!-- <img alt="Vue logo" src="./assets/logo.png"> -->
    <div class="header-container">
      <HeaderBox />
    </div>
    <router-view></router-view>
  </div>
</template>

<script>
import HeaderBox from './components/Header.vue';
// import store from './store'; // Import the store

export default {
  name: 'App',
  components: {
    HeaderBox,
  },
}
</script>

<style lang="scss">
$body-bg: #1c1b1b;
$body-color: #fff;
$medium-grey: #191a19;

// Apply styles to the HTML page
html {
  background-color: $body-bg;
  height: 100%; /* Set the HTML height to 100% */
}

body {
  color: $body-color;
  margin: 0; /* Remove default margin */
  min-height: 100%; /* Set the body min-height to 100% */
  display: flex; /* Use flexbox layout */
  flex-direction: column; /* Arrange elements in a column */
}

#app {
  font-family: Avenir, Helvetica, Arial, sans-serif;
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
  flex-grow: 1; /* Grow to fill the remaining vertical space */
  background-color: $body-bg;
  color: $body-color;
}

.header-container {
  margin-top: 0; /* Remove top margin to make it start at the top */
}

a {
  color: #ADD8E6;
}

a:hover {
  background-color: $medium-grey;
}

</style>
