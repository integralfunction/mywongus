use core::panic;
use gtk::{
    self,
    prelude::{ContainerExt, GtkWindowExt, WidgetExt},
    PadController,
};
use gtk_layer_shell::LayerShell;
use std::{collections::HashMap, ops::Deref};
use tao::{
    error::OsError,
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder, EventLoopProxy, EventLoopWindowTarget},
    platform::unix::{EventLoopWindowTargetExtUnix, WindowExtUnix},
    window::{Window, WindowBuilder, WindowId},
};
use wry::{http::Request, WebView, WebViewBuilder};
enum UserEvent {
    CloseWindow(WindowId),
    NewTitle(WindowId, String),
    NewWindow,
}

fn main() -> wry::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();
    let mut webviews = HashMap::new();
    let proxy = event_loop.create_proxy();

    let new_window = create_new_window(
        format!("Window {}", webviews.len() + 1),
        &event_loop,
        proxy.clone(),
    );
    webviews.insert(new_window.0.id(), (new_window.0, new_window.1));

    event_loop.run(move |event, event_loop, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } => {
                webviews.remove(&window_id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }
            Event::UserEvent(UserEvent::NewWindow) => {
                let new_window = create_new_window(
                    format!("Window {}", webviews.len() + 1),
                    event_loop,
                    proxy.clone(),
                );
                webviews.insert(new_window.0.id(), (new_window.0, new_window.1));
            }
            Event::UserEvent(UserEvent::CloseWindow(id)) => {
                webviews.remove(&id);
                if webviews.is_empty() {
                    *control_flow = ControlFlow::Exit
                }
            }

            Event::UserEvent(UserEvent::NewTitle(id, title)) => {
                webviews.get(&id).unwrap().0.set_title(&title);
            }
            _ => (),
        }
    });
}

fn create_new_window(
    title: String,
    event_loop: &EventLoopWindowTarget<UserEvent>,
    proxy: EventLoopProxy<UserEvent>,
) -> (Window, WebView) {
    // Create a gtk window
    let gtkwindow = gtk::ApplicationWindow::new(event_loop.gtk_app());
    // Initialize layer shell and set some properties. Layer shell is responsible for controlling the z-index of this bar. Apps in layer shell are usually bars, notifications, etc, etc.
    gtkwindow.init_layer_shell();
    gtkwindow.set_layer(gtk_layer_shell::Layer::Bottom);
    gtkwindow.set_keyboard_interactivity(true);
    gtkwindow.set_resizable(false);
    gtkwindow.set_app_paintable(true);
    gtkwindow.set_decorated(false);
    gtkwindow.stick();
    gtkwindow.set_title(&title);
    let default_vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    gtkwindow.add(&default_vbox);
    gtkwindow.show_all();
    // Create a tao window from the gtk window
    let window =
        Window::new_from_gtk_window(event_loop, gtkwindow).unwrap_or_else(|_error: OsError| {
            panic!("Could not create tao window from gtk window");
        });
    let window_id = window.id();
    let handler = move |req: Request<String>| {
        let body = req.body();
        match body.as_str() {
            "new-window" => {
                let _ = proxy.send_event(UserEvent::NewWindow);
            }
            "close" => {
                let _ = proxy.send_event(UserEvent::CloseWindow(window_id));
            }
            _ if body.starts_with("change-title") => {
                let title = body.replace("change-title:", "");
                let _ = proxy.send_event(UserEvent::NewTitle(window_id, title));
            }
            _ => {}
        }
    };

    let builder = {
        use tao::platform::unix::WindowExtUnix;
        use wry::WebViewBuilderExtUnix;
        WebViewBuilder::new_gtk(&default_vbox)
    };
    let webview = builder
        .with_transparent(true)
        .with_back_forward_navigation_gestures(false)
        .with_devtools(true)
        .with_ipc_handler(handler)
        //TODO don't hard code paths
        .with_url("file:///home/river/Documents/mywongus/src/index.html")
        //     .with_html(
        //         r#"
        //     <button onclick="window.ipc.postMessage('new-window')">Open a new window</button>
        //     <button onclick="window.ipc.postMessage('close')">Close current window</button>
        //     <input oninput="window.ipc.postMessage(`change-title:${this.value}`)" />
        // "#,
        //     )
        .build()
        .unwrap();
    (window, webview)
}
