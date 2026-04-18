import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],
  // Tauri expects a fixed port
  server: {
    port: 1420,
    strictPort: true,
  },
  // Prevent Vite from clearing the screen
  clearScreen: false,
})
