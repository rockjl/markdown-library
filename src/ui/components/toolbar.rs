use egui::RichText;

use crate::app::MarkdownApp;
use crate::editor_actions::EditorAction;

impl MarkdownApp {
    pub fn toolbar_button(&mut self, ui: &mut egui::Ui, label: &str, tooltip: &str, action: EditorAction) {
        let c = self.colors();
        let resp = ui.add(
            egui::Button::new(RichText::new(label).size(13.0).color(c.text_normal))
                .min_size(egui::vec2(32.0, 28.0))
                .fill(c.button_bg),
        );
        if resp.on_hover_text(tooltip).clicked() {
            self.pending_action = Some(action);
        }
    }

    pub fn draw_toolbar(&mut self, ui: &mut egui::Ui) {
        let c = self.colors();
        egui::Frame::none()
            .fill(c.toolbar_bg)
            .inner_margin(egui::Margin::symmetric(12, 8))
            .show(ui, |ui| {
                if self.toolbar_collapsed {
                    ui.horizontal(|ui| {
                        self.toolbar_button(ui, "B", "Bold (Ctrl+B)", EditorAction::Wrap { prefix: "**", suffix: "**" });
                        self.toolbar_button(ui, "I", "Italic (Ctrl+I)", EditorAction::Wrap { prefix: "*", suffix: "*" });
                        self.toolbar_button(ui, "</>", "Inline code (Ctrl+E)", EditorAction::Wrap { prefix: "`", suffix: "`" });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("⤢").on_hover_text("Expand toolbar").clicked() {
                                self.toolbar_collapsed = false;
                                self.save_settings();
                            }
                        });
                    });
                } else {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        ui.spacing_mut().item_spacing.y = 4.0;
                        self.toolbar_button(ui, "B", "Bold (Ctrl+B)", EditorAction::Wrap { prefix: "**", suffix: "**" });
                        self.toolbar_button(ui, "I", "Italic (Ctrl+I)", EditorAction::Wrap { prefix: "*", suffix: "*" });
                        self.toolbar_button(ui, "S", "Strikethrough", EditorAction::Wrap { prefix: "~~", suffix: "~~" });
                        ui.separator();
                        self.toolbar_button(ui, "H1", "Heading 1", EditorAction::LinePrefix("# "));
                        self.toolbar_button(ui, "H2", "Heading 2", EditorAction::LinePrefix("## "));
                        self.toolbar_button(ui, "H3", "Heading 3", EditorAction::LinePrefix("### "));
                        ui.separator();
                        self.toolbar_button(ui, "</>", "Inline code (Ctrl+E)", EditorAction::Wrap { prefix: "`", suffix: "`" });
                        self.toolbar_button(ui, "{ }", "Code block", EditorAction::CodeBlock(""));
                        self.toolbar_button(ui, "🔗", "Link (Ctrl+K)", EditorAction::Insert("[link](https://)"));
                        self.toolbar_button(ui, "🖼", "Image", EditorAction::Insert("![alt](path/to/image.png)"));
                        ui.separator();
                        self.toolbar_button(ui, "•", "Bullet list (Ctrl+Shift+L)", EditorAction::LinePrefix("- "));
                        self.toolbar_button(ui, "1.", "Numbered list", EditorAction::LinePrefix("1. "));
                        self.toolbar_button(ui, "☑", "Todo (Ctrl+Shift+T)", EditorAction::LinePrefix("- [ ] "));
                        self.toolbar_button(ui, "❝", "Quote (Ctrl+Shift+Q)", EditorAction::LinePrefix("> "));
                        self.toolbar_button(ui, "—", "Horizontal rule", EditorAction::Insert("\n---\n"));
                        self.toolbar_button(ui, "⊞", "Table", EditorAction::Table { rows: 2, cols: 3 });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("⤡").on_hover_text("Collapse toolbar").clicked() {
                                self.toolbar_collapsed = true;
                                self.save_settings();
                            }
                        });
                    });
                }
            });
    }
}
