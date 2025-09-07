//! Central button construction helpers ensuring consistent padding, style, and width.
use crate::ui::style::{pad_narrow, pad_primary, pad_std};
use serenity::builder::CreateButton;
use serenity::model::application::ButtonStyle;

pub struct Btn;
impl Btn {
    pub fn primary(id: &str, label: &str) -> CreateButton {
        CreateButton::new(id)
            .label(pad_primary(label))
            .style(ButtonStyle::Primary)
    }
    pub fn success(id: &str, label: &str) -> CreateButton {
        CreateButton::new(id)
            .label(pad_primary(label))
            .style(ButtonStyle::Success)
    }
    pub fn secondary(id: &str, label: &str) -> CreateButton {
        CreateButton::new(id)
            .label(pad_std(label))
            .style(ButtonStyle::Secondary)
    }
    pub fn danger(id: &str, label: &str) -> CreateButton {
        CreateButton::new(id)
            .label(pad_std(label))
            .style(ButtonStyle::Danger)
    }
    pub fn narrow(id: &str, label: &str) -> CreateButton {
        CreateButton::new(id)
            .label(pad_narrow(label))
            .style(ButtonStyle::Secondary)
    }
}
