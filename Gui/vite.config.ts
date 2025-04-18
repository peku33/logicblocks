import react from "@vitejs/plugin-react-swc";
import path from "path";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

const directory_normalized = path.posix.basename(__dirname).replaceAll(path.sep, "/");

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), tsconfigPaths()],
  build: {
    chunkSizeWarningLimit: 10 * 1024 * 1024, // 10MB
    rollupOptions: {
      output: {
        manualChunks(id: string) {
          if (id.startsWith("\x00")) {
            return null;
          }

          const module_path = path.posix.normalize(id);
          const relative_path = path.posix.relative(directory_normalized, module_path);

          const path_items = relative_path.split("/");

          if (path_items[0] == "src") {
            return "src";
          }
          if (path_items[0] == "node_modules") {
            if (["@fortawesome"].includes(path_items[1])) {
              return path_items[1];
            }

            return "vendor";
          }

          return null;
        },
      },
    },
  },
});
