// Run with node -r ./patch-fork-ts-checker.js so any require('fork-ts-checker-webpack-plugin') gets our stub.
const path = require('path');
const Module = require('module');
const stubPath = path.resolve(__dirname, 'craco-stub-fork-ts-checker.js');
const origResolve = Module._resolveFilename;
Module._resolveFilename = function (request, parent, isMain, options) {
  if (request === 'fork-ts-checker-webpack-plugin' || (request && String(request).includes('fork-ts-checker-webpack-plugin'))) return stubPath;
  let resolved = origResolve.call(this, request, parent, isMain, options);
  if (resolved && String(resolved).includes('fork-ts-checker-webpack-plugin') && !String(resolved).includes('craco-stub')) return stubPath;
  return resolved;
};
