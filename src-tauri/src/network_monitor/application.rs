use super::dto::{ApplicationTrafficSnapshot, AttributionQuality};

pub trait ApplicationTrafficCollector: Send {
    fn quality(&self) -> AttributionQuality;
    fn collect(&mut self) -> Vec<ApplicationTrafficSnapshot>;
}

#[derive(Default)]
pub struct UnavailableApplicationTrafficCollector;

impl ApplicationTrafficCollector for UnavailableApplicationTrafficCollector {
    fn quality(&self) -> AttributionQuality {
        AttributionQuality::Unavailable
    }

    fn collect(&mut self) -> Vec<ApplicationTrafficSnapshot> {
        Vec::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_collector_never_emits_synthetic_application_data() {
        let mut collector = UnavailableApplicationTrafficCollector;
        assert_eq!(collector.quality(), AttributionQuality::Unavailable);
        assert!(collector.collect().is_empty());
    }
}
