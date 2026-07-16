/** @type {import('@commitlint/types').UserConfig} */
module.exports = {
  extends: ['@commitlint/config-conventional'],
  rules: {
    'scope-enum': [
      2,
      'always',
      [
        'error',
        'config',
        'pymanager',
        'version-file',
        'shim',
        'shell',
        'cli',
        'commands',
        'ci',
        'docs',
        'openspec',
      ],
    ],
  },
};