ctx.tool.register({
  name: "get_version",
  description: "Get the current Nota version",
  parameters: {
    type: "object",
    properties: {},
  },
  run: () => {
    return "0.0.1";
  }
});
