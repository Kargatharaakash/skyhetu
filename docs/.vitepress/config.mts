import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "SkyHetu",
  description: "A causality-first programming language with explicit state tracking and logical time.",
  base: '/skyhetu/',

  head: [
    ['link', { rel: 'icon', href: '/skyhetu/skyhetu_dark.png' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'SkyHetu - Causality-First Language' }],
    ['meta', { property: 'og:description', content: 'A programming language where every state mutation is tracked. Debug by asking "why?"' }],
    ['meta', { property: 'og:image', content: 'https://kargatharaakash.github.io/skyhetu/skyhetu_dark.png' }],
    ['meta', { property: 'og:url', content: 'https://kargatharaakash.github.io/skyhetu/' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:title', content: 'SkyHetu - Causality-First Language' }],
    ['meta', { name: 'twitter:description', content: 'Debug by asking "why?" - Every state mutation is tracked.' }],
    ['meta', { name: 'twitter:image', content: 'https://kargatharaakash.github.io/skyhetu/skyhetu_dark.png' }],
    ['meta', { name: 'keywords', content: 'programming language, causality, rust, compiler, debugging, state management' }],
    ['meta', { name: 'author', content: 'Aakash Kargathara' }],
  ],

  themeConfig: {
    logo: {
      light: '/skyhetu_light.png',
      dark: '/skyhetu_dark.png'
    },

    siteTitle: 'SkyHetu',

    nav: [
      { text: 'Home', link: '/' },
      { text: 'Guide', link: '/guide/introduction' },
      { text: 'Reference', link: '/reference/builtins' }
    ],

    sidebar: {
      '/guide/': [
        {
          text: 'Introduction',
          items: [
            { text: 'What is SkyHetu?', link: '/guide/introduction' },
            { text: 'Installation', link: '/guide/installation' }
          ]
        },
        {
          text: 'Core Concepts',
          items: [
            { text: 'The Causality Engine', link: '/guide/causality' },
            { text: 'Modules', link: '/guide/modules' }
          ]
        }
      ],
      '/reference/': [
        {
          text: 'Reference',
          items: [
            { text: 'Built-in Functions', link: '/reference/builtins' },
            { text: 'Grammar', link: '/reference/grammar' }
          ]
        }
      ]
    },

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Kargatharaakash/skyhetu' }
    ],

    footer: {
      message: 'Released under the MIT License.',
      copyright: 'Copyright © 2026 Aakash Kargathara'
    }
  }
})
