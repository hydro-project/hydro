// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

import { themes } from "prism-react-renderer";

import remarkMath from "remark-math";
import rehypeKatex from "rehype-katex";

/** @type {import('@docusaurus/types').Config} */
const config = {
  title: "Hydro",
  tagline: "A Rust framework for correct and performant distributed systems",
  favicon: "img/favicon.ico",

  // Set the production url of your site here
  url: "https://hydro.run",
  // Set the /<baseUrl>/ pathname under which your site is served
  // For GitHub pages deployment, it is often '/<projectName>/'
  baseUrl: "/",

  // GitHub pages deployment config.
  // If you aren't using GitHub pages, you don't need these.
  organizationName: "hydro-project", // Usually your GitHub org/user name.
  projectName: "hydroflow", // Usually your repo name.

  onBrokenLinks: "throw",
  onBrokenMarkdownLinks: "throw",

  customFields: {
    LOAD_PLAYGROUND: process.env.LOAD_PLAYGROUND || false,
  },

  markdown: {
    mermaid: true,
  },

  themes: ["@docusaurus/theme-mermaid"],

  // Even if you don't use internalization, you can use this field to set useful
  // metadata like html lang. For example, if your site is Chinese, you may want
  // to replace "en" with "zh-Hans".
  i18n: {
    defaultLocale: "en",
    locales: ["en"],
  },

  stylesheets: [
    {
      href: "https://cdn.jsdelivr.net/npm/katex@0.13.24/dist/katex.min.css",
      type: "text/css",
      integrity:
        "sha384-odtC+0UGzzFL/6PNoE8rX/SPcQDXBJ+uRepguP4QkPCm2LBxH3FA3y+fKSiJ+AmM",
      crossorigin: "anonymous",
    },
  ],

  presets: [
    [
      "classic",
      /** @type {import('@docusaurus/preset-classic').Options} */
      ({
        docs: {
          sidebarPath: require.resolve("./sidebars.js"),
          // Please change this to your repo.
          // Remove this to remove the "edit this page" links.
          editUrl: "https://github.com/hydro-project/hydro/tree/main/docs/",
          remarkPlugins: [remarkMath],
          rehypePlugins: [rehypeKatex],
        },
        // blog: {
        //   showReadingTime: true,
        //   // Please change this to your repo.
        //   // Remove this to remove the "edit this page" links.
        //   editUrl:
        //     'https://github.com/hydro-project/hydro/tree/main/docs/',
        // },
        theme: {
          customCss: require.resolve("./src/css/custom.css"),
        },
      }),
    ],
  ],

  plugins: [
    [
      "@docusaurus/plugin-ideal-image",
      {
        quality: 75,
        max: 1080,
        min: 480,
        steps: 6,
        disableInDev: false,
      },
    ],
    require.resolve("./wasm-plugin.js"),
    function (context, options) {
      return {
        name: 'webpack-process-polyfill',
        configureWebpack(config, isServer, utils) {
          const webpack = require('webpack');
          return {
            resolve: {
              fallback: {
                process: require.resolve('process/browser.js'),
              },
            },
            plugins: [
              new webpack.ProvidePlugin({
                process: 'process/browser.js',
              }),
            ],
          };
        },
      };
    },
  ],

  themeConfig:
    /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
    ({
      // Replace with your project's social card
      image: "img/social-card.png",
      colorMode: {
        respectPrefersColorScheme: true,
      },
      docs: {
        sidebar: {
          autoCollapseCategories: true,
        },
      },
      navbar: {
        title: "Hydro",
        logo: {
          alt: "Hydro",
          src: "img/hydro-logo.svg",
        },
        items: [
          {
            to: "docs/hydro/learn/quickstart",
            activeBasePath: "docs/hydro/learn",
            position: "left",
            label: "Learn",
          },
          {
            type: "dropdown",
            label: "Reference",
            items: [
              {
                to: "docs/hydro/reference",
                activeBasePath: "docs/hydro/reference",
                label: "Hydro",
              },
              {
                type: "docSidebar",
                sidebarId: "dfirSidebar",
                label: "DFIR",
              },
              {
                href: "pathname:///rustdoc/hydro_lang/",
                label: "Rustdoc",
              },
            ],
          },
          {
            to: "/research",
            position: "left",
            label: "Publications",
          },
          {
            to: "/people",
            position: "left",
            label: "People",
          },
          // {to: '/blog', label: 'Blog', position: 'left'},
          {
            href: "https://github.com/hydro-project/hydro",
            position: "right",
            className: "header-github-link",
            "aria-label": "GitHub Repository",
          },
          {
            href: "https://discord.gg/QXKwMNA6RS",
            position: "right",
            className: "header-discord-link",
            "aria-label": "Discord server",
          },
        ],
      },
      footer: {
        style: "dark",
        links: [
          {
            title: "Reference",
            items: [
              {
                label: "Hydro",
                to: "/docs/hydro/reference/",
              },
              {
                label: "DFIR",
                to: "/docs/dfir/",
              },
            ],
          },
          {
            title: "Research Group",
            items: [
              {
                label: "Publications",
                to: "/research",
              },
              {
                label: "People",
                to: "/people",
              },
            ],
          },
          {
            title: "More",
            items: [
              // {
              //   label: 'Blog',
              //   to: '/blog',
              // },
              {
                label: "GitHub",
                href: "https://github.com/hydro-project/hydro",
              },
            ],
          },
        ],
        copyright: `Hydro is co-led by open-source developers from the <a href="https://sky.cs.berkeley.edu">Sky Computing Lab</a> at UC Berkeley, Amazon Web Services, and various participating institutions.`,
      },
      prism: {
        theme: themes.github,
        darkTheme: themes.dracula,
        additionalLanguages: ["rust", "bash"],
        magicComments: [
          {
            className: "theme-code-block-highlighted-line",
            line: "highlight-next-line",
            block: { start: "highlight-start", end: "highlight-end" },
          },
          {
            className: "shell-command-line",
            line: "shell-command-next-line",
          },
        ],
      },
      algolia: {
        appId: "C2TSTQAKIC",
        apiKey: "38cef87035f42759bc1dd871e91e06ba",
        indexName: "hydro",
      },
    }),
  scripts: [
    // {
    //   id: "runllm-widget-script",
    //   type: "module",
    //   src: "https://widget.runllm.com",
    //   "runllm-server-address": "https://api.runllm.com",
    //   "runllm-assistant-id": "136",
    //   "runllm-position": "BOTTOM_RIGHT",
    //   "runllm-keyboard-shortcut": "Mod+j",
    //   "runllm-preset": "docusaurus",
    //   "runllm-slack-community-url": "",
    //   "runllm-name": "Hydro",
    //   "runllm-theme-color": "#005EEC",
    //   async: true,
    // },
  ],
};

module.exports = config;
