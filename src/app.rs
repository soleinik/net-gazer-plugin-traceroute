extern crate lib_data;
use lib_data::*;

use core::CoreSender;

use pnet::packet::ethernet::{EtherTypes, EthernetPacket};
use pnet::packet::icmp::{
    destination_unreachable::DestinationUnreachablePacket, echo_reply::EchoReplyPacket,
    echo_request::EchoRequestPacket, time_exceeded::TimeExceededPacket, IcmpPacket, IcmpTypes,
};
use pnet::packet::ip::IpNextHeaderProtocols;
use pnet::packet::ipv4::Ipv4Packet;
use pnet::packet::tcp::{TcpOptionNumbers, TcpPacket};
use pnet::packet::Packet;

use ipnetwork::Ipv4Network;
use std::net::Ipv4Addr;
use std::time::Instant;

use std::sync::mpsc;


pub struct App {
    net:Ipv4Network,
    ip:Ipv4Addr,
    data_sender:lib_data::SenderChannel
}



impl App {
    pub fn new(net:Ipv4Network, tx:CoreSender) -> Self{

        let ip = net.ip();

        //communication via async channels - unbounded queue, watch for OOM. 
        // 1:1 producer:consumer
        let (data_sender, data_receiver): (lib_data::SenderChannel,lib_data::ReceiverChannel) = mpsc::channel();

        lib_tracer::start(data_receiver, ip, tx);
        lib_tracer::timer_start(data_sender.clone());

        App{
            net,
            ip,
            data_sender
        }
    }


    pub fn process(&self, ethernet_packet: &EthernetPacket) {
        match ethernet_packet.get_ethertype() {
            EtherTypes::Ipv4 => {
                if let Some(ip4pkt) = Ipv4Packet::new(ethernet_packet.payload()) {
                    match ip4pkt.get_next_level_protocol() {
                        IpNextHeaderProtocols::Tcp => {
                            if let Some(tcp) = TcpPacket::new(ip4pkt.payload()) {
                                let flags = tcp.get_flags();

                                // this can be used to calculate bandwith use between endpoints
                                // let size = ip4pkt.get_total_length();
                                // println!("======>packet size:{}", size);

                                if 0 == tcp.get_window() {
                                    let src_port = tcp.get_source();
                                    let dst_port = tcp.get_destination();
                                    let src = ip4pkt.get_source();
                                    let dst = ip4pkt.get_destination();
                                    trace!("source overloaded {}:{} -> {}:{}. Application performance issues?", src,src_port, dst, dst_port);
                                }

                                if tcp
                                    .get_options_iter()
                                    .any(|o| o.get_number() == TcpOptionNumbers::SACK)
                                {
                                    //let mss = tcp.get_options_iter().any(|o| o.get_number() == TcpOptionNumbers::MSS);

                                    let src_port = tcp.get_source();
                                    let dst_port = tcp.get_destination();

                                    let src = ip4pkt.get_source();
                                    let dst = ip4pkt.get_destination();
                                    trace!("re-transmission request detected: {}:{} -> {}:{}. Connection quality issues?", src,src_port, dst, dst_port);
                                }

                                //SYN, SYN-ACK, ACK, u16
                                if !has_bit(flags, Flags::SYN) {
                                    //ignore all but SYN+ flag
                                    return;
                                }

                                let src = ip4pkt.get_source();
                                let dst = ip4pkt.get_destination();
                                let outbound = self.net.contains(src);

                                if !has_bit(flags, Flags::ACK) {
                                    //SYN flag
                                    self.data_sender
                                        .send(AppData::Syn(AppTcp::new(
                                            src,
                                            dst,
                                            outbound,
                                            Some(Instant::now()),
                                            None,
                                        )))
                                        .unwrap();
                                } else {
                                    //SYN-ACK
                                    self.data_sender
                                        .send(AppData::SynAck(AppTcp::new(
                                            src,
                                            dst,
                                            outbound,
                                            None,
                                            Some(Instant::now()),
                                        )))
                                        .unwrap();
                                }

                                return;
                            }
                        }
                        IpNextHeaderProtocols::Udp => return,
                        IpNextHeaderProtocols::Icmp => {
                            if let Some(icmp) = IcmpPacket::new(ip4pkt.payload()) {
                                //TODO: add ours EchoRequest for elapsed time

                                let dst = ip4pkt.get_destination();
                                if self.ip != dst {
                                    //only replies back to us
                                    return;
                                }

                                match icmp.get_icmp_type() {
                                    // IcmpTypes::EchoRequest => {
                                    //     let src = ip4pkt.get_source();
                                    //     info!("ICMP-Request {} -> {} [id:{},seq:{},ttl:{}]", src, dst, pkt_id, pkt_seq, ip4pkt.get_ttl());
                                    // }
                                    IcmpTypes::EchoReply => {
                                        if let Some(echo) = EchoReplyPacket::new(ip4pkt.payload()) {
                                            let src = ip4pkt.get_destination();
                                            let dst = ip4pkt.get_source();
                                            let ttl = ip4pkt.get_ttl();

                                            let pkt_id = echo.get_identifier();
                                            let pkt_seq = echo.get_sequence_number();

                                            self.data_sender
                                                .send(AppData::IcmpReply(AppIcmp {
                                                    src,
                                                    dst,
                                                    hop: dst,
                                                    pkt_id,
                                                    pkt_seq,
                                                    ttl,
                                                    ts: Instant::now(),
                                                }))
                                                .unwrap();
                                        }
                                    }
                                    IcmpTypes::TimeExceeded => {
                                        if let Some(timeex_pkt) =
                                            TimeExceededPacket::new(ip4pkt.payload())
                                        {
                                            let hop = ip4pkt.get_source();
                                            let src = ip4pkt.get_destination(); //this ip

                                            if let Some(ip4_hdr) =
                                                Ipv4Packet::new(timeex_pkt.payload())
                                            {
                                                let dst = ip4_hdr.get_destination(); //intended
                                                let ttl = ip4_hdr.get_ttl(); //this is not reliable... we will use seq

                                                if let Some(echoreq_pkt) =
                                                    EchoRequestPacket::new(ip4_hdr.payload())
                                                {
                                                    let pkt_id = echoreq_pkt.get_identifier();
                                                    let pkt_seq = echoreq_pkt.get_sequence_number();

                                                    self.data_sender
                                                        .send(AppData::IcmpExceeded(AppIcmp {
                                                            src,
                                                            dst,
                                                            hop,
                                                            pkt_id,
                                                            pkt_seq,
                                                            ttl,
                                                            ts: Instant::now(),
                                                        }))
                                                        .unwrap();
                                                }
                                            }
                                        }
                                    }
                                    IcmpTypes::DestinationUnreachable => {
                                        if let Some(unreach_pkt) =
                                            DestinationUnreachablePacket::new(ip4pkt.payload())
                                        {
                                            let hop = ip4pkt.get_source();
                                            let src = ip4pkt.get_destination(); //this ip
                                            if let Some(ip4_hdr) =
                                                Ipv4Packet::new(unreach_pkt.payload())
                                            {
                                                let dst = ip4_hdr.get_destination(); //intended
                                                let ttl = ip4_hdr.get_ttl();
                                                if let Some(echoreq_pkt) =
                                                    EchoRequestPacket::new(ip4_hdr.payload())
                                                {
                                                    let pkt_id = echoreq_pkt.get_identifier();
                                                    let pkt_seq = echoreq_pkt.get_sequence_number();

                                                    self.data_sender
                                                        .send(AppData::IcmpUnreachable(AppIcmp {
                                                            src,
                                                            dst,
                                                            hop,
                                                            pkt_id,
                                                            pkt_seq,
                                                            ttl,
                                                            ts: Instant::now(),
                                                        }))
                                                        .unwrap();
                                                }
                                            }
                                        }
                                    }
                                    // IcmpTypes::Traceroute => {
                                    //     println!("=============> IcmpTypes::Traceroute <=========================")
                                    // }
                                    _ => {
                                        println!("icmp type:{:?}", icmp.get_icmp_type());
                                        return;
                                    }
                                }
                            }
                        }
                        _ => return,
                    }
                }
            }
            EtherTypes::Ipv6 => {}

            _ => {}
        }
    }
}

use pnet::packet::tcp::TcpFlags;
bitflags! {
    struct Flags: u16 {
        const SYN = TcpFlags::SYN; //2
        const URG = TcpFlags::URG; //32
        const ACK = TcpFlags::ACK; //16
        const PSH = TcpFlags::PSH; //8
        const RST = TcpFlags::RST; //4
        const FIN = TcpFlags::FIN; //1

        const CWR = TcpFlags::CWR; //
        const ECE = TcpFlags::ECE; //
    }
}

fn has_bit(flags: u16, bit: Flags) -> bool {
    if let Some(s) = Flags::from_bits(flags) {
        return s.contains(bit);
    }
    false
}
