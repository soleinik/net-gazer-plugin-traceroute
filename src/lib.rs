#[macro_use] extern crate log;
extern crate net_gazer_core as core;
#[macro_use] extern crate bitflags;

mod app;

use core::*;
use pnet::packet::ethernet::EthernetPacket;
use pnet::datalink::NetworkInterface;



const ID:u8=lib_data::PLUGIN_ID;
const NAME:&str="Traceroute plugin";

#[derive(Default)]
pub struct TraceRoutePlugin{
    app:Option<app::App>
}

impl Plugin for TraceRoutePlugin{

    fn get_name(&self)->&str{NAME}
    fn get_id(&self) -> u8 {ID}
 
    fn on_load(&mut self, iface:&NetworkInterface, tx:CoreSender){
        env_logger::init();

        let net = iface.ips.iter()
            .map(|net| {
                match net{
                    ipnetwork::IpNetwork::V4(net)=> Some(net),
                    _ => None
                }
            })
            .find(|net| net.is_some()).flatten().unwrap();
        
        self.app = Some(app::App::new(*net, tx));

        info!("Hello from \"{}\"(message_id:{}), ! ", NAME, ID);
    }

    fn on_unload(&mut self){
        info!("Good bye from \"{}\"(message_id:{})! ", NAME, ID);
    }

    fn process(&self, pkt:&EthernetPacket){
        trace!("Processing with \"{}\"(message_id:{})", NAME,ID);
        self.app.as_ref().unwrap().process(pkt);
    }
}

#[no_mangle]
pub extern "C" fn net_gazer_plugin_new () -> * mut dyn Plugin{
     let boxed:Box<TraceRoutePlugin> = Box::new(TraceRoutePlugin::default());
     Box::into_raw(boxed)
}