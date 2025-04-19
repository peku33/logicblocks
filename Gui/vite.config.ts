import react from "@vitejs/plugin-react-swc";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

// const directory_normalized = __dirname.replaceAll(path.sep, "/");

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), tsconfigPaths()],
  build: {
    sourcemap: true,
    chunkSizeWarningLimit: 10 * 1024 * 1024, // 10MB
    // broken cyclic dependnecies: https://github.com/vitejs/vite/issues/12209
    // rollupOptions: {
    //   output: {
    //     manualChunks(id: string) {
    //       if (id.startsWith("\x00")) {
    //         return "builtin";
    //       }

    //       const module_path = path.posix.normalize(id);
    //       const relative_path = path.posix.relative(directory_normalized, module_path);

    //       const path_items = relative_path.split("/");

    //       if (path_items[0] == "node_modules") {
    //         if (["@fortawesome", "react"].includes(path_items[1])) {
    //           return path_items[1].replaceAll("@", "");
    //         }

    //         return "vendor";
    //       }

    //       console.log({ id });
    //       return "src";
    //     },
    //   },
    // },
  },
});
