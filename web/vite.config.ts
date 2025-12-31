import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'
import { vanillaExtractPlugin } from '@vanilla-extract/vite-plugin'
import wasm from 'vite-plugin-wasm'

export default defineConfig({
  plugins: [react(), vanillaExtractPlugin(), wasm()],
  base: '/jpp/',
  build: {
    target: 'esnext',
  },
})
