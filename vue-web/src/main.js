// import Vue from 'vue'
// import App from './App.vue'
// import { BootstrapVue } from 'bootstrap-vue'
// import './assets/custom.scss'  // adjust path if necessary
//
// Vue.use(BootstrapVue)
// Vue.config.productionTip = false
//
// new Vue({
//   render: h => h(App),
// }).$mount('#app')

// main.js

import Vue from 'vue'
import App from './App.vue'
import { BootstrapVue } from 'bootstrap-vue'
import './assets/custom.scss'  // adjust path if necessary
import VueRouter from 'vue-router'
import Header from './components/Header.vue'
import Dashboard from './components/Dashboard.vue'
import Hash from "./components/Hash.vue";

Vue.use(BootstrapVue)
Vue.use(VueRouter) // Use VueRouter

Vue.config.productionTip = false

// Define your routes
const routes = [
  { path: '/hash/:hashParam', components: { default: Hash, header: Header } },
  { path: '/', components: { default: Dashboard, header: Header } },
]

// Create the router instance and pass the `routes` option
const router = new VueRouter({
  routes
})

new Vue({
  router,  // Inject the router to make the whole app router-aware.
  render: h => h(App),
}).$mount('#app')