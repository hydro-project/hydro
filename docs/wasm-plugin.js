const path = require('path');

module.exports = function (context, options) {
  return {
    name: 'wasm-docusuarus-plugin',
    // eslint-disable-next-line
    configureWebpack(config, isServer, utils) {
      const useRealHydroscope = process.env.LOAD_HYDROSCOPE === '1';
      return {
        experiments: {
          asyncWebAssembly: !isServer,
        },
        module: {
          rules: isServer ? [
            {
              test: /\.wasm$/,
              type: "asset/inline",
            },
          ] : []
        },
        resolve: {
          alias: {
            ...(process.env.LOAD_PLAYGROUND !== "1" ? {
              "website_playground/website_playground_bg.wasm": false,
              "website_playground/website_playground_bg.js": false
            } : {}),
            // When LOAD_HYDROSCOPE is not set, alias the package to a local shim
            ...(!useRealHydroscope ? {
              "@hydro-project/hydroscope": path.resolve(__dirname, 'src/shims/hydroscope.tsx')
            } : {})
          }
        }
      };
    },
  };
};
