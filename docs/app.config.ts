export default defineAppConfig({
  docus: {
    title: 'Redgold',
    description: 'Redgold documentation hub for applications, wallet & network support',
    image: 'https://redgold-public.s3.us-west-1.amazonaws.com/cover.png',
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
