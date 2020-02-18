use lib_data::{AppTraceRoute, AppHop};
use lib_fbuffers::Builder;

const PLUGIN_ID:u8 = lib_data::PLUGIN_ID;

use async_std::task;

pub fn consume_route(bldr:&mut Builder, route:& AppTraceRoute,tx:&super::CommTxChannel){
    let data = bldr.create_route_message(&[route.clone()]);
    tx.send((PLUGIN_ID, data)).unwrap();
    crate::traceroute::process(route.request.clone().unwrap());
}

pub fn consume_hop(bldr:&mut Builder, hop:&AppHop, tx:&super::CommTxChannel){

    let data = bldr.create_hop_message(&[hop.clone()]);
    let _tx = tx.clone();

    task::spawn(async move {
        _tx.send((PLUGIN_ID, data)).unwrap();
    });
}
