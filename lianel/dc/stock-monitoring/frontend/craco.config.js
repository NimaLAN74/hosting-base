// Build uses patch-fork-ts-checker.js via NODE_OPTIONS so fork-ts-checker never loads (avoids ajv errors).
module.exports = {
  webpack: {
    configure: (config) => config,
  },
};
