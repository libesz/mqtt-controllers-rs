use std::sync::atomic::AtomicUsize;

use rumqttc::QoS;

use crate::controller::Controller;

pub struct TemperatureReader {
    topic: String,
    target: &'static AtomicUsize
}

impl TemperatureReader {
    pub fn new(topic: String, target: & 'static AtomicUsize) -> Self {
        TemperatureReader { topic, target }
    }
}

impl Controller for TemperatureReader {
    fn init(&mut self, client: &mut rumqttc::Client) {
        client.subscribe(self.topic.to_string(), QoS::AtMostOnce).unwrap();
    }

    fn notification(&mut self, topic: &String, payload: &bytes::Bytes, _: &mut rumqttc::Client) {
        if *topic == self.topic {
            let stringed = std::str::from_utf8(payload).unwrap();
            if let json::JsonValue::Object(object) = json::parse(stringed).unwrap() {
                let temp = (object["temperature"].as_f64().unwrap() * 10.0) as usize;
                self.target.swap(temp, std::sync::atomic::Ordering::Relaxed);
            }    
        }
    }
}