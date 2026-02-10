// Remove fork-ts-checker-webpack-plugin to avoid ajv/schema-utils build errors (JS-only app).
module.exports = {
  webpack: {
    configure: (config) => {
      config.plugins = config.plugins.filter(
        (p) => p.constructor.name !== 'ForkTsCheckerWebpackPlugin'
      );
      return config;
    },
  },
};
