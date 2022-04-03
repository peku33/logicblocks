module.exports = {
  "stories": [
    "../src/**/*.stories.tsx"
  ],
  "core": {
    "builder": 'webpack5',
  },
  "staticDirs": ['../public'],
  "addons": [
    "@storybook/addon-links",
    "@storybook/addon-essentials",
    "@storybook/preset-create-react-app"
  ]
}
