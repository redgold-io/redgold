export default defineNuxtConfig({
  // https://github.com/nuxt-themes/docus
  extends: '@nuxt-themes/docus',

  modules: [
    // https://github.com/nuxt-modules/plausible
    '@nuxtjs/plausible',
    // https://github.com/nuxt/devtools
    '@nuxt/devtools'
  ],
  target: 'static', // default is 'server'
  ssr: true,
  head: { // this doesn't seem to work
    link: [{ rel: 'icon', type: 'image/x-icon', href: '/favicon.png' }]
  }
})
