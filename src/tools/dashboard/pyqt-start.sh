#!/bin/bash

# Wayland/Weston 환경 설정
export QT_QPA_PLATFORM=wayland
export QT_PLUGIN_PATH=/usr/lib/aarch64-linux-gnu/qt5/plugins
export QT_QPA_PLATFORM_PLUGIN_PATH=/usr/lib/aarch64-linux-gnu/qt5/plugins/platforms
export LD_LIBRARY_PATH=/usr/lib/aarch64-linux-gnu:/usr/lib/aarch64-linux-gnu/qt5/plugins/platforms
export QT_WAYLAND_DISABLE_WINDOWDECORATION=1
export QT_DEBUG_PLUGINS=1

# node_modules 디렉토리 권한 설정
cd /app
if [ -d "node_modules" ]; then
    chown -R $(id -u):$(id -g) node_modules
fi

# Vite 개발 서버 시작 (백그라운드로)
npm install
npm run dev -- --host 0.0.0.0 &

# Vite 서버가 완전히 시작될 때까지 대기
sleep 5

# PyQt WebView 애플리케이션 실행
python3 qt_dashboard_main.py --no-sandbox
