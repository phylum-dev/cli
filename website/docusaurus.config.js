// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require('prism-react-renderer/themes/github');
const darkCodeTheme = require('prism-react-renderer/themes/dracula');

/** @type {import('@docusaurus/types').Config} */
const config = {
    title: 'Phylum CLI',
    tagline: 'Command line interface for the Phylum API',
    favicon: 'img/favicon.ico',

    // Set the production url of your site here
    url: 'https://cli.phylum.io',
    // Set the /<baseUrl>/ pathname under which your site is served
    // For GitHub pages deployment, it is often '/<projectName>/'
    baseUrl: '/',

    // GitHub pages deployment config.
    // If you aren't using GitHub pages, you don't need these.
    organizationName: 'phylum-dev', // Usually your GitHub org/user name.
    projectName: 'cli', // Usually your repo name.
    deploymentBranch: 'gh-pages',
    trailingSlash: false,

    onBrokenLinks: 'throw',
    onBrokenMarkdownLinks: 'throw',
    onDuplicateRoutes: 'throw',

    // Even if you don't use internalization, you can use this field to set useful
    // metadata like html lang. For example, if your site is Chinese, you may want
    // to replace "en" with "zh-Hans".
    i18n: {
        defaultLocale: 'en',
        locales: ['en'],
    },

    presets: [
        [
            'classic',
            /** @type {import('@docusaurus/preset-classic').Options} */
            ({
                docs: {
                    path: '../docs',
                    // Make this a "Docs-only" site
                    routeBasePath: '/',
                    sidebarPath: require.resolve('./sidebars.js'),
                    // Remove this to remove the "edit this page" links.
                    editUrl: 'https://github.com/phylum-dev/cli/tree/main/website/',
                    showLastUpdateAuthor: true,
                    showLastUpdateTime: true,
                },
                blog: false,
                theme: {
                    customCss: require.resolve('./src/css/custom.css'),
                },
            }),
        ],
    ],

    themeConfig:
        /** @type {import('@docusaurus/preset-classic').ThemeConfig} */
        ({
            colorMode: {
                defaultMode: 'dark',
                disableSwitch: false,
                respectPrefersColorScheme: true,
            },
            // The announcement bar can be used to highlight big changes
            announcementBar: {
                content: 'Welcome to the new Phylum CLI documentation!',
                textColor: '#fff',
                backgroundColor: '#3480eb',
            },
            navbar: {
                title: 'Phylum CLI',
                logo: {
                    alt: 'Phylum Logo',
                    src: 'img/phylum_logo.svg',
                },
                hideOnScroll: false,
                items: [
                    {
                        type: 'docSidebar',
                        sidebarId: 'cliSidebar',
                        position: 'left',
                        label: 'Docs',
                    },
                    {
                        href: 'https://github.com/phylum-dev/cli',
                        position: 'right',
                        className: 'header-github-link',
                        'aria-label': 'GitHub repository',
                    },
                ],
            },
            footer: {
                style: 'dark',
                logo: {
                    alt: 'Phylum Logo',
                    src: 'img/phylum_logo.svg',
                    height: 100,
                    width: 100,
                    href: 'https://phylum.io',
                },
                copyright: `Copyright © 2020-${new Date().getFullYear()} Phylum, Inc.`,
                links: [
                    {
                        title: 'Docs',
                        items: [
                            {
                                label: 'Quickstart',
                                to: '/',
                            },
                            {
                                label: 'Commands',
                                to: 'commands/phylum',
                            },
                            {
                                label: 'Extensions',
                                to: 'extensions/extension_overview',
                            },
                        ],
                    },
                    {
                        title: 'Community',
                        items: [
                            {
                                label: 'Discord',
                                href: 'https://discord.gg/c9QnknWxm3',
                            },
                            {
                                label: 'Twitter',
                                href: 'https://twitter.com/Phylum_IO',
                            },
                            {
                                label: 'YouTube',
                                href: 'https://www.youtube.com/@phylum_io',
                            },
                            {
                                label: 'DEV',
                                href: 'https://dev.to/phylum',
                            },
                            {
                                label: 'LinkedIn',
                                href: 'https://www.linkedin.com/company/phylum-io',
                            },
                        ],
                    },
                    {
                        title: 'More',
                        items: [
                            {
                                label: 'GitHub',
                                href: 'https://github.com/phylum-dev/cli',
                            },
                        ],
                    },
                ],
            },
            prism: {
                theme: lightCodeTheme,
                darkTheme: darkCodeTheme,
                additionalLanguages: [
                    'toml',
                ],
            },
            docs: {
                sidebar: {
                    hideable: true,
                    autoCollapseCategories: true,
                }
            },
        }),
};

module.exports = config;
