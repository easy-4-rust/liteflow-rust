use std::sync::Arc;

use crate::spi::SpiFactory;

use super::RequestIdGenerator;

pub struct RequestIdGeneratorHolder {
    request_id_generator: Arc<dyn RequestIdGenerator>,
}

impl RequestIdGeneratorHolder {
    pub fn new() -> Self {
        let request_id_generator = SpiFactory::request_id_generator();
        Self {
            request_id_generator,
        }
    }

    pub fn request_id_generator(&self) -> Arc<dyn RequestIdGenerator> {
        self.request_id_generator.clone()
    }
}
