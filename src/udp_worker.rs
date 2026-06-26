use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::{Sender, channel};
use chrono::Local;
use eframe::egui;

#[derive(Debug)]
pub enum UdpCommand {
    Bind(String),
    Send { target: String, data: Vec<u8> },
    Unbind,
    JoinMulticast { multi_addr: String, interface_addr: String },
    LeaveMulticast { multi_addr: String, interface_addr: String },
}

#[derive(Debug)]
pub enum UdpEvent {
    Bound(SocketAddr),
    Unbound,
    Sent { to: SocketAddr, data: Vec<u8>, timestamp: chrono::DateTime<Local> },
    Received { from: SocketAddr, data: Vec<u8>, timestamp: chrono::DateTime<Local> },
    Error(String),
    MulticastJoined { multi_addr: String, interface_addr: String },
    MulticastLeft { multi_addr: String, interface_addr: String },
}

#[derive(Clone)]
struct EventSender {
    tx: Sender<UdpEvent>,
    ctx: egui::Context,
}

impl EventSender {
    fn send(&self, event: UdpEvent) {
        let _ = self.tx.send(event);
        self.ctx.request_repaint();
    }
}

pub struct UdpWorker {
    tx_cmd: Sender<UdpCommand>,
}

impl UdpWorker {
    pub fn spawn(tx_event: Sender<UdpEvent>, ctx: egui::Context) -> Self {
        let (tx_cmd, rx_cmd) = channel();
        let event_sender = EventSender { tx: tx_event, ctx };
        
        let es = event_sender.clone();
        thread::spawn(move || {
            let event_sender = es;
            let mut active_socket: Option<Arc<UdpSocket>> = None;
            let mut stop_flag: Option<Arc<AtomicBool>> = None;

            while let Ok(cmd) = rx_cmd.recv() {
                match cmd {
                    UdpCommand::Bind(addr_str) => {
                        // Unbind existing first
                        if let Some(ref flag) = stop_flag {
                            flag.store(true, Ordering::SeqCst);
                        }
                        active_socket = None;
                        stop_flag = None;

                        match addr_str.to_socket_addrs() {
                            Ok(mut addrs) => {
                                if let Some(addr) = addrs.next() {
                                    match UdpSocket::bind(addr) {
                                        Ok(socket) => {
                                            // Set a read timeout so the receive loop checks the stop flag periodically
                                            let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
                                            // Enable UDP broadcasting
                                            let _ = socket.set_broadcast(true);
                                            
                                            // Resolve the actual local address (in case port was 0)
                                            let bound_addr = socket.local_addr().unwrap_or(addr);

                                            let socket_arc = Arc::new(socket);
                                            let flag_arc = Arc::new(AtomicBool::new(false));

                                            active_socket = Some(socket_arc.clone());
                                            stop_flag = Some(flag_arc.clone());

                                            let es_recv = event_sender.clone();
                                            let socket_receiver = socket_arc.clone();
                                            let flag_receiver = flag_arc.clone();

                                            // Spawn receiver thread
                                            thread::spawn(move || {
                                                let event_sender = es_recv;
                                                let mut buf = [0u8; 65535];
                                                while !flag_receiver.load(Ordering::SeqCst) {
                                                    match socket_receiver.recv_from(&mut buf) {
                                                        Ok((size, from)) => {
                                                            let data = buf[..size].to_vec();
                                                            let timestamp = Local::now();
                                                            event_sender.send(UdpEvent::Received {
                                                                from,
                                                                data,
                                                                timestamp,
                                                            });
                                                        }
                                                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock || e.kind() == std::io::ErrorKind::TimedOut => {
                                                            // Just timeout, continue and check stop flag
                                                        }
                                                        Err(e) => {
                                                            // Only send error if we are not shutting down
                                                            if !flag_receiver.load(Ordering::SeqCst) {
                                                                event_sender.send(UdpEvent::Error(format!("Receive error: {}", e)));
                                                            }
                                                            break;
                                                        }
                                                    }
                                                }
                                            });

                                            event_sender.send(UdpEvent::Bound(bound_addr));
                                        }
                                        Err(e) => {
                                            event_sender.send(UdpEvent::Error(format!("Failed to bind: {}", e)));
                                        }
                                    }
                                } else {
                                    event_sender.send(UdpEvent::Error(format!("No addresses resolved for bind: {}", addr_str)));
                                }
                            }
                            Err(e) => {
                                event_sender.send(UdpEvent::Error(format!("Invalid bind address '{}': {}", addr_str, e)));
                            }
                        }
                    }
                    UdpCommand::Send { target, data } => {
                        if let Some(ref socket) = active_socket {
                            match target.to_socket_addrs() {
                                Ok(mut addrs) => {
                                    if let Some(target_addr) = addrs.next() {
                                        match socket.send_to(&data, target_addr) {
                                            Ok(_) => {
                                                event_sender.send(UdpEvent::Sent {
                                                    to: target_addr,
                                                    data,
                                                    timestamp: Local::now(),
                                                });
                                            }
                                            Err(e) => {
                                                event_sender.send(UdpEvent::Error(format!("Send error: {}", e)));
                                            }
                                        }
                                    } else {
                                        event_sender.send(UdpEvent::Error(format!("No addresses resolved for target: {}", target)));
                                    }
                                }
                                Err(e) => {
                                    event_sender.send(UdpEvent::Error(format!("Invalid target address '{}': {}", target, e)));
                                }
                            }
                        } else {
                            event_sender.send(UdpEvent::Error("Not bound. Bind to a local port first.".to_string()));
                        }
                    }
                    UdpCommand::Unbind => {
                        if let Some(ref flag) = stop_flag {
                            flag.store(true, Ordering::SeqCst);
                        }
                        active_socket = None;
                        stop_flag = None;
                        event_sender.send(UdpEvent::Unbound);
                    }
                    UdpCommand::JoinMulticast { multi_addr, interface_addr } => {
                        if let Some(ref socket) = active_socket {
                            match (multi_addr.parse::<std::net::IpAddr>(), interface_addr.parse::<std::net::IpAddr>()) {
                                (Ok(std::net::IpAddr::V4(m)), Ok(std::net::IpAddr::V4(i))) => {
                                    match socket.join_multicast_v4(&m, &i) {
                                        Ok(_) => {
                                            event_sender.send(UdpEvent::MulticastJoined { multi_addr, interface_addr });
                                        }
                                        Err(e) => {
                                            event_sender.send(UdpEvent::Error(format!("Failed to join IPv4 multicast: {}", e)));
                                        }
                                    }
                                }
                                (Ok(std::net::IpAddr::V6(m)), _) => {
                                    let interface_idx = interface_addr.parse::<u32>().unwrap_or(0);
                                    match socket.join_multicast_v6(&m, interface_idx) {
                                        Ok(_) => {
                                            event_sender.send(UdpEvent::MulticastJoined { multi_addr, interface_addr });
                                        }
                                        Err(e) => {
                                            event_sender.send(UdpEvent::Error(format!("Failed to join IPv6 multicast: {}", e)));
                                        }
                                    }
                                }
                                _ => {
                                    event_sender.send(UdpEvent::Error("Invalid multicast or interface address format".to_string()));
                                }
                            }
                        } else {
                            event_sender.send(UdpEvent::Error("Not bound. Bind to a local port first.".to_string()));
                        }
                    }
                    UdpCommand::LeaveMulticast { multi_addr, interface_addr } => {
                        if let Some(ref socket) = active_socket {
                            match (multi_addr.parse::<std::net::IpAddr>(), interface_addr.parse::<std::net::IpAddr>()) {
                                (Ok(std::net::IpAddr::V4(m)), Ok(std::net::IpAddr::V4(i))) => {
                                    match socket.leave_multicast_v4(&m, &i) {
                                        Ok(_) => {
                                            event_sender.send(UdpEvent::MulticastLeft { multi_addr, interface_addr });
                                        }
                                        Err(e) => {
                                            event_sender.send(UdpEvent::Error(format!("Failed to leave IPv4 multicast: {}", e)));
                                        }
                                    }
                                }
                                (Ok(std::net::IpAddr::V6(m)), _) => {
                                    let interface_idx = interface_addr.parse::<u32>().unwrap_or(0);
                                    match socket.leave_multicast_v6(&m, interface_idx) {
                                        Ok(_) => {
                                            event_sender.send(UdpEvent::MulticastLeft { multi_addr, interface_addr });
                                        }
                                        Err(e) => {
                                            event_sender.send(UdpEvent::Error(format!("Failed to leave IPv6 multicast: {}", e)));
                                        }
                                    }
                                }
                                _ => {
                                    event_sender.send(UdpEvent::Error("Invalid multicast or interface address format".to_string()));
                                }
                            }
                        } else {
                            event_sender.send(UdpEvent::Error("Not bound. Bind to a local port first.".to_string()));
                        }
                    }
                }
            }
        });

        UdpWorker { tx_cmd }
    }

    pub fn send(&self, cmd: UdpCommand) {
        let _ = self.tx_cmd.send(cmd);
    }
}


