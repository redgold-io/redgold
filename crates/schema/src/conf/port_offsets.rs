pub trait PortOffsetHelpers {
    fn udp_port(&self) -> u16;
}

impl PortOffsetHelpers for i64 {
    fn udp_port(&self) -> u16 {
        (self + 5) as u16
    }
}
