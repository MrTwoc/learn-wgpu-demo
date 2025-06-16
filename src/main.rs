/*
    学习文章
    https://jinleili.github.io/learn-wgpu-zh/beginner/tutorial1-window#%E6%B7%BB%E5%8A%A0%E5%AF%B9-web-%E7%9A%84%E6%94%AF%E6%8C%81
*/
use std::sync::{Arc, Mutex};

use winit::{application::ApplicationHandler, event::WindowEvent, event_loop::{ActiveEventLoop, EventLoop}, window::{Window, WindowId}};

struct WgpuApp {
    /// 避免窗口被释放
    #[allow(unused)]
    window: Arc<Window>,
}

impl WgpuApp {
    async fn new(window: Arc<Window>) -> Self {
        // ...
        Self { window }
    }
}

#[derive(Default)]
struct WgpuAppHandler {
    app: Arc<Mutex<Option<WgpuApp>>>,
}

impl ApplicationHandler for WgpuAppHandler {fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // 修复点：正确检查 Option<WgpuApp> 的存在性
        let mut app_guard = self.app.lock().unwrap();
        if app_guard.is_some() {
            return; // 已初始化则跳过
        }

        let window_attributes = Window::default_attributes().with_title("tutorial1-window");
        let window = Arc::new(event_loop.create_window(window_attributes).unwrap());

        // 使用 pollster 阻塞运行异步初始化
        let wgpu_app = pollster::block_on(WgpuApp::new(window));
        *app_guard = Some(wgpu_app); // 存入 Mutex
    }

    fn suspended(&mut self, _event_loop: &ActiveEventLoop) {
        // 暂停事件
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        // 窗口事件
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::Resized(_size) => {
                // 窗口大小改变
            }
            WindowEvent::KeyboardInput { .. } => {
                // 键盘事件
            }
            WindowEvent::RedrawRequested => {
                // surface重绘事件
            }
            _ => (),
        }
    }
}

fn main() -> Result<(), impl std::error::Error> {
    env_logger::init();

    let events_loop = EventLoop::new().unwrap();
    let mut app = WgpuAppHandler::default();
    events_loop.run_app(&mut app)
}