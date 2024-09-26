use rand::Rng;
use redgold_gui::dependencies::gui_depends::GuiDepends;
use redgold_schema::conf::node_config::NodeConfig;
use crate::core::relay::Relay;

#[derive(Clone)]
pub struct NativeGuiDepends {
    nc: NodeConfig
}

impl NativeGuiDepends {
    pub fn new(nc: NodeConfig) -> Self {
        Self {
            nc
        }
    }
}

impl GuiDepends for NativeGuiDepends {
    fn get_salt(&self) -> i64 {
        let mut rng = rand::thread_rng();
        rng.gen::<i64>()
    }
}
