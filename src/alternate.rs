use std::collections::{HashMap, HashSet};

use bytes::Bytes;
use json::object;
use rumqttc::{Client, QoS};

use crate::controller::Controller;

pub struct AlternateGroup {
    state: bool,
    topics: Vec<String>,
    initial_responses_count: usize,
    change_requests: HashMap<String, (bool, HashSet<String>)>
}

impl AlternateGroup {
    pub fn new(topics: Vec<String>) -> Self {
        AlternateGroup{topics, state: false, initial_responses_count: 0, change_requests: HashMap::new()}
    }
    fn broadcast(&self, get: bool, exception_topic: &String, client: &mut Client) {
        self.topics.iter().for_each(|topic| {
            if topic != exception_topic {
                let state_string = match self.state {
                    true => "ON",
                    false => "OFF"
                };
                let final_topic = match get {
                    true => topic.to_string()+"/get",
                    false => topic.to_string()+"/set",
                };
                _ = client.publish(final_topic, QoS::AtMostOnce, false, object!("state": state_string).dump());
            }
        });
    }
}
impl Controller for AlternateGroup {
    fn init(&mut self, client: &mut Client) {
        self.topics.iter_mut().for_each(|topic| {
            client.subscribe(topic.to_string(), QoS::AtMostOnce).unwrap();
        });
        // Sending out a state get query to learn the current output state. Initial answers will be collected and synced before normal operation.
        self.broadcast(true, &String::new(), client);
    }
    fn notification(&mut self, topic: &String, payload: &Bytes, client: &mut Client) {
        // are we interested in this topic?
        if self.topics.iter_mut().find(|item| item == &topic) == None {
            return
        }
        
        let stringed = std::str::from_utf8(payload).unwrap();
        if let json::JsonValue::Object(object) = json::parse(stringed).unwrap() {
            
            if self.initial_responses_count < self.topics.len() {
                // collecting all initial responses to avoid flappy outputs
                self.initial_responses_count += 1;
                if self.initial_responses_count == self.topics.len() {
                    let input_state = match object["state"].to_string().as_str() {
                        "ON" => true,
                        _ => false,
                    };
                    self.state = input_state;
                    println!("Initial states collected, syncing all output devices to the last one: {}", input_state);
                    self.broadcast(false, topic, client);
                }
            } else {
                let input_state = match object["state"].to_string().as_str() {
                    "ON" => true,
                    _ => false,
                };

                let mut found = false;
                for (key, value) in self.change_requests.iter_mut() {
                    if key == topic {
                        if value.0 != input_state {
                            value.0 = input_state;
                            value.1 = HashSet::new();
                        }// else {
                            found = true;
                        //}
                    } else {
                        if value.0 == input_state {
                            value.1.insert(topic.to_string());
                        }
                    }
                }
                if self.state != input_state {
                    if !found {
                        self.change_requests.insert(topic.to_string(), (input_state, HashSet::new()));
                    }
                    self.state = input_state;
                    println!("State changed to: {}, broadcasting to all output devices", input_state);
                    self.broadcast(false, topic, client);
                }

                println!("change_requests before retain: {:?}", self.change_requests);

                self.change_requests.retain(|_, value| value.1.len() != (self.topics.len() - 1));

                println!("change_requests after retain: {:?}", self.change_requests);

                // if self.state != input_state {
                //     // normal state change operation
                //     println!("State changed to: {}, broadcasting to all output devices", input_state);
                //     self.state = input_state;
                //     self.broadcast(false, topic, client);
                // }
            }
        } else {
            println!("Expected JSON object");
        }
    }
}