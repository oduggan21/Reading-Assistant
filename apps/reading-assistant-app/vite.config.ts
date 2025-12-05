import { reactRouter } from "@react-router/dev/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import tsconfigPaths from "vite-tsconfig-paths";

export default defineConfig({
  plugins: [
    // Use the tailwindcss plugin for Vite
    tailwindcss(),
    // Use the React Router plugin for file-based routing
    reactRouter(),
    // Use tsconfigPaths for clean module imports
    tsconfigPaths()
  ],
  resolve: {
    // Prevent duplicate instances from linked workspace packages
    dedupe: ["react", "react-dom", "@tanstack/react-query"],
  },
  server: {
    port: 3002,
  },
});

