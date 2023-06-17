<template>
  <div id="app">
    <router-view name="header"></router-view>
    <router-view />
  </div>
</template>

<script>
import 'bootstrap/dist/css/bootstrap.css'
import 'bootstrap-vue/dist/bootstrap-vue.css'

export default {
  name: 'App',
}
</script>

<style>
</style>
