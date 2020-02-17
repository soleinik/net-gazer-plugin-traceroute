#[macro_use] extern crate log;
extern crate net_gazer_core as core;

use core::*;
use pnet::packet::ethernet::EthernetPacket;

const ID:u8=1;
const NAME:&str="Traceroute plugin";

#[derive(Default)]
pub struct DemoPlugin;

impl Plugin for DemoPlugin{

    fn get_name(&self)->&str{NAME}

    fn get_id(&self) -> u8 {ID}
 
    fn on_load(&self){
        env_logger::init();
        info!("Hello from \"{}\"(message_id:{}), ! ", NAME, ID);
    }

    fn on_unload(&self){
        info!("Good bye from \"{}\"(message_id:{})! ", NAME, ID);
    }

    fn process(&self, _tx:CoreSender, _pkt:&EthernetPacket){
        info!("Processing with \"{}\"(message_id:{})", NAME,ID);
    }
}

#[no_mangle]
pub extern "C" fn net_gazer_plugin_new () -> * mut dyn Plugin{
     let boxed:Box<DemoPlugin> = Box::new(DemoPlugin::default());
     Box::into_raw(boxed)
}



