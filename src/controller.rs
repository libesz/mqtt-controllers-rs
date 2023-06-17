use bytes::Bytes;
use rumqttc::{Client};

pub trait Controller {
    fn init(&mut self, client: &mut Client);
    fn notification(&mut self, topic: &String, payload: &Bytes, client: &mut Client);
}