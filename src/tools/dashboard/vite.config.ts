import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react-swc';
import svgr from 'vite-plugin-svgr';
//import svgLoader from 'vite-svg-loader';

// https://vite.dev/config/
export default defineConfig({
  plugins: [react(), svgr()],
  cacheDir: '/tmp/.vite',
  server: {
    host: '0.0.0.0',
    port: 5173,
    proxy: {      
      '/api/v1/metrics': {
        target: 'http://10.0.0.30:8080',
        changeOrigin: true,
        secure: false,
      },
      '/api/containers': {
        target: 'http://localhost:5000',
        secure: false,
        changeOrigin: true,
        ws: true,
        configure: (proxy) => {
          proxy.on('proxyReq', (proxyReq) => {
            proxyReq.setHeader('Connection', 'keep-alive');
          });
        },
        timeout: 60000,

        proxyTimeout: 60000,
      },
    },
  },
});

