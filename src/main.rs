#![feature(proc_macro_hygiene, decl_macro)]

mod alternate;
mod controller;
mod temperature;

use std::thread;
use controller::Controller;
use rumqttc::{MqttOptions, Client, Outgoing, Event::*};
use std::time::Duration;

use std::sync::atomic::AtomicUsize;

static G_TEMPERATURE_VALUE: AtomicUsize = AtomicUsize::new(0);

#[macro_use] extern crate rocket;

#[get("/")]
fn hello() -> String {
    let raw = G_TEMPERATURE_VALUE.load(std::sync::atomic::Ordering::Relaxed);
    format!("The temperature is: {}.{}", raw / 10, raw % 10)
}

fn main() {
    thread::spawn(move || {
        rocket::ignite().mount("/", routes![hello]).launch();
    });


    let mut mqttoptions = MqttOptions::new("rumqtt-sync", "localhost", 1883);
    mqttoptions.set_keep_alive(Duration::from_secs(5));
    
    let (mut client, mut connection) = Client::new(mqttoptions, 10);
    //client.subscribe("zigbee2mqtt/0x00124b002911333d", QoS::AtMostOnce).unwrap();
    let mut temperature_reader = temperature::TemperatureReader::new("zigbee2mqtt/0x00124b002911333d".to_string(), &G_TEMPERATURE_VALUE);
    temperature_reader.init(&mut client);

    let mut alternate_switches = alternate::AlternateGroup::new(vec!("zigbee2mqtt/stairs1".to_string(), "zigbee2mqtt/stairs2".to_string()));
    alternate_switches.init(&mut client);

    let mut controllers: Vec<Box<dyn Controller>> = vec![Box::new(alternate_switches), Box::new(temperature_reader)];

    for (_, notification) in connection.iter().enumerate() {
        let notif_ok = notification.unwrap();
        match notif_ok.clone() {
            Incoming(incoming) => {
                match incoming {
                    rumqttc::Packet::PingReq => {},
                    rumqttc::Packet::PingResp => {},
                    _ => {
                        println!("Notification = {:?}", notif_ok);
                    },
                }
                match incoming {
                    rumqttc::Packet::Publish(publish) => {
                        println!("Publish payload = {:?}", publish.payload);
                        for controller in controllers.iter_mut() {
                            controller.notification(&publish.topic, &publish.payload, &mut client);
                        }
                    },
                    _ => {},
                }
            },
            Outgoing(outgoing) => {
                match outgoing {
                    Outgoing::PingReq => {},
                    Outgoing::PingResp => {},
                    _ => {
                        println!("Notification = {:?}", notif_ok);
                    }
                }
            },
        }
    }
}