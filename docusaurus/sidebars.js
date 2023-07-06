/**
 * Creating a sidebar enables you to:
 - create an ordered group of docs
 - render a sidebar for each doc of that group
 - provide next/previous navigation

 The sidebars can be generated from the filesystem, or explicitly defined here.

 Create as many sidebars as you want.
 */

// @ts-check

/** @type {import('@docusaurus/plugin-content-docs').SidebarsConfig} */
const sidebars = {
  // Reference: https://docusaurus.io/docs/sidebar
  cliSidebar: [
    'quickstart',
    'alternate_install',
    'supported_lockfiles',
    'lockfile_generation',
    'analyzing_dependencies',
    {
      type: 'category',
      label: 'Extensions',
      link: {
        type: 'doc',
        id: 'extensions/extension_overview',
      },
      items: [
        'extensions/extension_quickstart',
        'extensions/extension_manifest',
        'extensions/extension_api',
        'extensions/extension_example',
        'extensions/extension_sandboxing',
        'extensions/extension_rest_api',
      ],
    },
  ],
};

module.exports = sidebars;
