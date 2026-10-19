import { defineConfig, type Plugin } from "vite";
import react from "@vitejs/plugin-react";

function sdsHtmlPlugin(mode: string): Plugin {
  return {
    name: "sds-html-transform",
    transformIndexHtml(html) {
      if (mode === "sds") {
        return html
          .replace(/<title>.*?<\/title>/, "<title>AX Portal</title>")
          .replace(
            /<link\s+rel="icon"[^>]*href="\/vite\.svg"[^>]*\/?>/,
            '<link rel="icon" type="image/png" href="/ax-portal/favicon.png" />'
          );
      }
      return html;
    },
  };
}

export default defineConfig(({ mode }) => ({
  plugins: [react(), sdsHtmlPlugin(mode)],
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
