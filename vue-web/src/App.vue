  
<template>
  <div id="app">
    <Header />
    <Dashboard />
  </div>
</template>

<script>
import 'bootstrap/dist/css/bootstrap.css'
import 'bootstrap-vue/dist/bootstrap-vue.css'

import Header from './components/Header.vue'
import Dashboard from './components/Dashboard.vue'

export default {
  name: 'App',
  components: {
    Header,
    Dashboard
  }
}
</script>

<style>

</style>
