import { defineConfig } from 'vitepress'

export default defineConfig({
  title: "SkyHetu",
  description: "A causality-first programming language",
  base: '/skyhetu/',
  themeConfig: {
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
    ]
  }
})
