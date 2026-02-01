import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// https://vite.dev/config/
export default defineConfig({
  server: {
    port: 5174,
    host: true,
    allowedHosts: ["1961-explorer.atdigitalbank.com.br"],
  },
  plugins: [react()],
});
