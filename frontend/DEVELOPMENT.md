# Development Guide

This guide provides step-by-step instructions for common development tasks in the Roome frontend.

## Adding a New Page

### 1. Create the page module

```rust
// frontend/src/pages/new_page/mod.rs
use crate::events::UiCommand;

pub struct NewPage;

impl NewPage {
    /// Pure view function - does NOT mutate state
    /// Returns UiCommand for controller to handle
    pub fn show(ui: &mut egui::Ui, data: &Data) -> Option<UiCommand> {
        let mut command = None;

        ui.vertical(|ui| {
            ui.heading("New Page");
            
            if ui.button("Do something").clicked() {
                command = Some(UiCommand::NewAction);
            }
        });

        command
    }
}
```

### 2. Add to Page enum

```rust
// frontend/src/pages/mod.rs
pub enum Page {
    Login,
    Home,
    NewPage,  // ← Add here
}
```

### 3. Implement rendering

```rust
// frontend/src/app/state.rs
impl App {
    fn render_page(&mut self, ctx: &egui::Context) -> Option<UiCommand> {
        match self.current_page {
            Page::NewPage => self.render_new_page(ctx),
            // ...
        }
    }
}
```

## Adding a New Action Flow

### 1. Define UiCommand

```rust
// frontend/src/events/ui_command.rs
pub enum UiCommand {
    ToggleScreenShare,
}
```

### 2. Emit from view

```rust
if ui.button("📺 Share Screen").clicked() {
    command = Some(UiCommand::ToggleScreenShare);
}
```

### 3. Handle in controller

```rust
// frontend/src/app/ui_handler.rs
impl App {
    pub(super) fn handle_ui_command(&mut self, command: UiCommand) {
        match command {
            UiCommand::ToggleScreenShare => {
                self.screen_share_enabled = !self.screen_share_enabled;
                // If heavy, send to logic thread
                self.logic_cmd_tx.send(LogicCommand::StartScreenShare).unwrap();
            }
        }
    }
}
```

### 4. Add LogicCommand for heavy operations

```rust
// frontend/src/events/logic_command.rs
pub enum LogicCommand {
    StartScreenShare,
}

// frontend/src/logic/mod.rs
pub fn run_logic_thread(cmd_rx: Receiver<LogicCommand>) {
    for command in cmd_rx {
        match command {
            LogicCommand::StartScreenShare => {
                // Heavy work here
                match capture_screen() {
                    Ok(frame) => evt_tx.send(LogicEvent::ScreenShareStarted(frame)).unwrap(),
                    Err(e) => evt_tx.send(LogicEvent::Error(e.to_string())).unwrap(),
                }
            }
        }
    }
}
```

### 5. Handle the response

```rust
// frontend/src/events/logic_event.rs
pub enum LogicEvent {
    ScreenShareStarted(VideoFrame),
}

// frontend/src/app/logic_handler.rs
impl App {
    pub(super) fn handle_logic_event(&mut self, event: LogicEvent) {
        match event {
            LogicEvent::ScreenShareStarted(frame) => {
                self.screen_share_frame = Some(frame);
            }
        }
    }
}
```

## Best Practices

### Views Should Be Pure
- Never mutate state directly in views
- Always return `UiCommand` for state changes
- Accept only `&data` (not `&mut data`)

### Controller Responsibilities
- Process all `UiCommand`s
- Update application state
- Send `LogicCommand`s for heavy work
- Handle `LogicEvent`s from background threads

### Logic Thread Usage
- Use for WebRTC operations
- Use for camera/video processing
- Use for network requests
- Use for file I/O
- Communicate via events only

## Common Patterns

### Navigation Between Pages

```rust
// In view
if ui.button("Go to Settings").clicked() {
    return Some(UiCommand::NavigateToSettings);
}

// In controller
UiCommand::NavigateToSettings => {
    self.current_page = Page::Settings;
}
```

### Showing Toasts/Notifications

```rust
// In logic handler after event
LogicEvent::Success(msg) => {
    self.show_toast(msg);
}
```
