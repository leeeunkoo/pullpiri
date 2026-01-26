import sys
from PyQt5.QtWidgets import QApplication
from PyQt5.QtCore import QUrl, Qt
from PyQt5.QtWebEngineWidgets import QWebEngineView
from PyQt5.QtWidgets import QMainWindow

class MainWindow(QMainWindow):
    def __init__(self):
        super().__init__()
        
        # 타이틀바 제거 및 전체 화면 설정
        self.setWindowFlags(self.windowFlags() | Qt.FramelessWindowHint)
        self.showFullScreen()
        
        # WebView 생성
        self.web_view = QWebEngineView(self)
        self.setCentralWidget(self.web_view)
        
        # Vite 개발 서버 URL 로드
        self.web_view.setUrl(QUrl("http://localhost:5173"))

def main():
    # QApplication 인스턴스 생성
    app = QApplication(sys.argv)
    app.setDesktopFileName("com.piccolo.dashboard")
    
    # MainWindow 인스턴스 생성
    window = MainWindow()
    window.show()
    
    # 이벤트 루프 시작
    sys.exit(app.exec_())

if __name__ == "__main__":
    main()
