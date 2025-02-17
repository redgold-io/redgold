import "bootstrap/dist/css/bootstrap.min.css"
import "bootstrap"

import {createApp} from 'vue'
import {createRouter, createWebHistory} from 'vue-router'
import App from './App.vue'
// import HelloWorld from './components/HelloWorld.vue'
// import HelloWorld2 from './components/HelloWorld2.vue'
import Dashboard from './components/HomePage.vue'
// import Header from './components/Header.vue'
// import HashDetails from "./components/HashDetails.vue";
import store from './store'; // Assuming store.js is in the root directory alongside main.js

// Define routes
const routes = [
    // { path: '/hash/:param', component: HashDetails },
    { path: '/', component: Dashboard},
]

// Create router
const router = createRouter({
    history: createWebHistory(),
    routes
})

// Create app
const app = createApp(App)

// Use router
app.use(router)
app.use(store)

// Mount app
app.mount('#app')
