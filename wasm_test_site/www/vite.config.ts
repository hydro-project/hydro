import { defineConfig } from "vite";
import wasmPack from "vite-plugin-wasm-pack";
import path from "node:path";

export default defineConfig({
  // pass your local crate path to the plugin
  plugins: [wasmPack(path.resolve("../"))],
});
