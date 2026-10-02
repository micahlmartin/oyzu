import { defineConfig } from "vite";

export default defineConfig(({ mode }) => ({
  build: { outDir: `web/${mode}` },
  plugins: [
    {
      name: "example-native-asset",
      generateBundle() {
        this.emitFile({
          type: "asset",
          fileName: "native-mode.txt",
          source: mode,
        });
      },
    },
  ],
}));
