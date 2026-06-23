import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  build: {
    target: 'es2018',
    cssCodeSplit: true,
    rollupOptions: {
      output: {
        manualChunks: {
          'tauri-api': ['@tauri-apps/api'],
        },
      },
    },
  },
  server: {
    watch: {
      ignored: ['**/src-tauri/target/**'],
    },
  },
  clearScreen: false,
})
