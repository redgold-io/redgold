export default defineAppConfig({
  docus: {
    title: 'Redgold',
    description: 'Redgold documentation hub for applications, wallet & network support',
    image: 'https://user-images.githubusercontent.com/904724/185365452-87b7ca7b-6030-4813-a2db-5e65c785bf88.png',
    socials: {
      twitter: 'redgold_io',
      github: 'redgold-io/redgold'
    },
    github: {
      dir: 'docs/content',
      branch: 'dev',
      repo: 'redgold',
      owner: 'redgold-io',
      edit: true
    },
    aside: {
      level: 0,
      collapsed: false,
      exclude: []
    },
    main: {
      padded: true,
      fluid: true
    },
    header: {
      logo: true,
      showLinkIcon: true,
      exclude: [],
      fluid: true
    },
    footer: {
      textLinks: [
        {
          href: 'https://redgold.io',
          text: 'Main Website'
        },
        {
          href: 'https://dev.explorer.redgold.io',
          text: 'Explorer'
        },
        {
          href: 'https://discord.gg/86fzxJg8ce',
          text: 'Discord'
        },
      ],
      iconLinks: [
        {
          href: 'https://redgold.io',
          icon: 'simple-icons:r'
        }
      ]
    }
  }
})
