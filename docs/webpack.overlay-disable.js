module.exports = function (context, options) {
  return {
    name: 'webpack-dev-overlay-disable',
    configureWebpack(config, isServer, utils) {
      return {
        devServer: {
          client: {
            overlay: false,
          },
        },
      };
    },
  };
};
