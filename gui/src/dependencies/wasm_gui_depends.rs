use redgold_schema::util::times::current_time_millis;
use crate::dependencies::gui_depends::GuiDepends;

pub struct WasmGuiDepends {

}

impl GuiDepends for WasmGuiDepends {
    fn get_salt(&self) -> i64 {
        let random = current_time_millis();
        random
    }
}