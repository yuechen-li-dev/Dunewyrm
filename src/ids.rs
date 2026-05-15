#![allow(non_snake_case)]

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct DwFrameId {
    pub Domain: u64,
    pub Local: u32,
}
