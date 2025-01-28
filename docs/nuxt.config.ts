export default defineNuxtConfig({
  // https://github.com/nuxt-themes/docus
  extends: '@nuxt-themes/docus',

  modules: [
    // https://github.com/nuxt-modules/plausible
    '@nuxtjs/plausible',
    // https://github.com/nuxt/devtools
    '@nuxt/devtools'
  ],

  // default is 'server'
  target: 'static',

  ssr: true,

  head: { // this doesn't seem to work
    link: [{ rel: 'icon', type: 'image/x-icon', href: '/favicon.png' }]
  },

  css: [
    '~/assets/css/custom.css',
    'katex/dist/katex.min.css'
  ],

  app: {
    head: {
      link: [{
        rel: 'stylesheet',
        href: 'https://cdn.jsdelivr.net/npm/katex@0.11.0/dist/katex.min.css'
      }]
    }
  },

  compatibilityDate: '2024-08-09',

  content: {
    markdown: {
      remarkPlugins: [
        'remark-math'
      ],
      rehypePlugins: [
        'rehype-katex'
      ]
    }
  },

  vue: {
    compilerOptions: {
      isCustomElement: tag => {
        const arrTags = ['semantics', 'mrow', 'msup', 'mi', 'math']
        const answ = arrTags.indexOf(tag.toLowerCase()) !== -1
        console.log(tag+' :: '+ answ)
        return answ
      }
    }
  },
  hooks: {
    'content:file:beforeParse': (file) => {
      console.log('Processing file:', file.path)
      console.log('File content:', file.body)
    }
  }
})