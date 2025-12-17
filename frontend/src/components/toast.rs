/// Toast notification component for displaying transient messages to users
///
/// Supports error, warning, success, and info types with distinct styles.
use std::time::Instant;

/// Toast notification type
#[derive(Clone, Debug, PartialEq)]
pub enum ToastType {
    /// Error notification - red with alert icon
    Error,
    /// Warning notification - yellow with warning icon
    Warning,
    /// Success notification - green with checkmark icon
    Success,
    /// Info notification - blue with info icon
    Info,
}

impl ToastType {
    /// Get the icon for this toast type
    fn icon(&self) -> &str {
        match self {
            ToastType::Error => "❌",
            ToastType::Warning => "!",
            ToastType::Success => "✅",
            ToastType::Info => "ℹ",
        }
    }

    /// Get the icon color for this toast type
    fn icon_color(&self) -> egui::Color32 {
        match self {
            ToastType::Error => egui::Color32::from_rgb(255, 100, 100),
            ToastType::Warning => egui::Color32::from_rgb(255, 200, 100),
            ToastType::Success => egui::Color32::from_rgb(100, 255, 100),
            ToastType::Info => egui::Color32::from_rgb(100, 150, 255),
        }
    }

    /// Get the background color for this toast type
    fn background_color(&self) -> egui::Color32 {
        match self {
            ToastType::Error => egui::Color32::from_rgba_premultiplied(80, 30, 30, 230),
            ToastType::Warning => egui::Color32::from_rgba_premultiplied(80, 70, 30, 230),
            ToastType::Success => egui::Color32::from_rgba_premultiplied(30, 80, 30, 230),
            ToastType::Info => egui::Color32::from_rgba_premultiplied(30, 50, 80, 230),
        }
    }
}

/// Toast notification for displaying messages to users
#[derive(Clone)]
pub struct Toast {
    pub message: String,
    pub toast_type: ToastType,
    pub created_at: Instant,
    pub duration_secs: f32,
}

impl Toast {
    /// Creates a new toast with the specified type and default 5-second duration
    pub fn new(message: String, toast_type: ToastType) -> Self {
        Self {
            message,
            toast_type,
            created_at: Instant::now(),
            duration_secs: 5.0,
        }
    }

    /// Creates an error toast (red with alert icon)
    pub fn error(message: String) -> Self {
        Self::new(message, ToastType::Error)
    }

    /// Creates a warning toast (yellow with warning icon)
    pub fn warning(message: String) -> Self {
        Self::new(message, ToastType::Warning)
    }

    /// Creates a success toast (green with checkmark icon)
    pub fn success(message: String) -> Self {
        Self::new(message, ToastType::Success)
    }

    /// Creates an info toast (blue with info icon)
    pub fn info(message: String) -> Self {
        Self::new(message, ToastType::Info)
    }

    /// Checks if the toast has expired based on creation time
    pub fn is_expired(&self) -> bool {
        self.created_at.elapsed().as_secs_f32() > self.duration_secs
    }

    /// Renders the toast notification and returns true if it should be dismissed
    pub fn show(&self, ctx: &egui::Context) -> bool {
        // Check if expired first - don't render if expired
        if self.is_expired() {
            return true; // Signal to dismiss
        }

        let mut should_dismiss = false;

        egui::Window::new("notification")
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-20.0, -20.0))
            .fixed_size(egui::vec2(350.0, 80.0))
            .frame(
                egui::Frame::window(&ctx.style())
                    .fill(self.toast_type.background_color())
                    .stroke(egui::Stroke::new(
                        1.5,
                        self.toast_type.icon_color().linear_multiply(0.7),
                    ))
                    .corner_radius(8.0)
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 4],
                        blur: 16,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(100),
                    }),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    // Icon
                    ui.label(
                        egui::RichText::new(self.toast_type.icon())
                            .size(28.0)
                            .color(self.toast_type.icon_color()),
                    );

                    ui.add_space(10.0);

                    // Message and button
                    ui.vertical(|ui| {
                        ui.label(
                            egui::RichText::new(&self.message)
                                .size(14.0)
                                .color(egui::Color32::WHITE),
                        );

                        ui.add_space(5.0);

                        // Close button - manual dismiss
                        let button_color = self.toast_type.icon_color().linear_multiply(0.8);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Dismiss").color(egui::Color32::WHITE),
                                )
                                .fill(button_color)
                                .stroke(egui::Stroke::NONE)
                                .corner_radius(4.0),
                            )
                            .clicked()
                        {
                            should_dismiss = true;
                        }
                    });
                });
            });

        // Request repaint for continuous expiration check
        ctx.request_repaint();

        should_dismiss
    }
}
