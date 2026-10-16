import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig(({ mode }) => ({
  plugins: [react()],
  define: mode === "sds" ? {
    "import.meta.env.VITE_APP_PROFILE": JSON.stringify("sds"),
    "import.meta.env.VITE_SDS_ENVIRONMENT": JSON.stringify(process.env.SDS_ENVIRONMENT ?? "internal"),
  } : {},
  server: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/target/**"],
    },
  },
  preview: {
    host: "127.0.0.1",
    port: 1420,
    strictPort: true,
  },
}));
