// Stub fork-ts-checker so react-scripts never loads the real plugin (avoids ajv/schema-utils errors).
const path = require('path');
const Module = require('module');
const stubPath = path.resolve(__dirname, 'craco-stub-fork-ts-checker.js');
const origResolve = Module._resolveFilename;
Module._resolveFilename = function (request, parent, isMain, options) {
  if (request === 'fork-ts-checker-webpack-plugin') return stubPath;
  return origResolve.call(this, request, parent, isMain, options);
};

module.exports = {
  webpack: {
    configure: (config) => config,
  },
};
