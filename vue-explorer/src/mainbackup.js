import { createApp } from 'vue'
import { createRouter, createWebHistory } from 'vue-router'
import App from './App.vue'
import HelloWorld from './components/HelloWorld.vue'
import Hash from './components/HashDetails.vue'

// Define routes
const routes = [
    { path: '/hash', component: Hash },
    { path: '/', component: HelloWorld }
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

// Mount app
app.mount('#app')
