// Stub for fork-ts-checker-webpack-plugin to avoid loading ajv/schema-utils (JS-only app).
function StubForkTsCheckerWebpackPlugin() {}
StubForkTsCheckerWebpackPlugin.prototype.apply = function () {};
module.exports = StubForkTsCheckerWebpackPlugin;
