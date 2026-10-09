//! Passive packet capture through Npcap/libpcap, loaded at runtime (no SDK at
//! build time). Only reads copies of the frames the network card sees; nothing
//! is injected into the game's connection.

use std::ffi::{c_char, c_int, c_long, c_uint, CStr, CString};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use libloading::Library;

pub struct Segment {
    pub src_ip: [u8; 4],
    pub src_port: u16,
    pub dst_ip: [u8; 4],
    pub dst_port: u16,
    pub seq: u32,
    pub payload: Vec<u8>,
}

#[derive(Debug)]
pub enum CaptureError {
    /// Npcap (wpcap.dll) is not installed or could not be loaded.
    LibraryMissing(String),
    NoDevices,
    Other(String),
}

type PcapT = *mut std::ffi::c_void;

#[repr(C)]
struct PcapIf {
    next: *mut PcapIf,
    name: *const c_char,
    description: *const c_char,
    addresses: *mut PcapAddr,
    flags: c_uint,
}

#[repr(C)]
struct PcapAddr {
    next: *mut PcapAddr,
    addr: *mut SockAddr,
    netmask: *mut SockAddr,
    broadaddr: *mut SockAddr,
    dstaddr: *mut SockAddr,
}

#[repr(C)]
struct SockAddr {
    sa_family: u16,
    sa_data: [u8; 14],
}

/// `struct pcap_pkthdr`; its timestamp is a C `struct timeval` of two `long`s.
#[repr(C)]
struct PcapPkthdr {
    ts_sec: c_long,
    ts_usec: c_long,
    caplen: c_uint,
    len: c_uint,
}

const PCAP_IF_LOOPBACK: c_uint = 1;
const AF_INET: u16 = 2;

struct PcapLib {
    _lib: Library,
    findalldevs: unsafe extern "C" fn(*mut *mut PcapIf, *mut c_char) -> c_int,
    freealldevs: unsafe extern "C" fn(*mut PcapIf),
    open_live: unsafe extern "C" fn(*const c_char, c_int, c_int, c_int, *mut c_char) -> PcapT,
    close: unsafe extern "C" fn(PcapT),
    next_ex: unsafe extern "C" fn(PcapT, *mut *mut PcapPkthdr, *mut *const u8) -> c_int,
    datalink: unsafe extern "C" fn(PcapT) -> c_int,
}

// The function pointers are plain C functions; each thread uses its own handle.
unsafe impl Send for PcapLib {}
unsafe impl Sync for PcapLib {}

#[cfg(windows)]
fn load_library() -> Result<Library, String> {
    use libloading::os::windows::{Library as WinLibrary, LOAD_WITH_ALTERED_SEARCH_PATH};
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| r"C:\Windows".into());
    let npcap = format!(r"{root}\System32\Npcap\wpcap.dll");
    // The altered search path lets wpcap.dll find packet.dll next to it.
    if let Ok(lib) = unsafe { WinLibrary::load_with_flags(&npcap, LOAD_WITH_ALTERED_SEARCH_PATH) } {
        return Ok(lib.into());
    }
    unsafe { Library::new("wpcap.dll") }.map_err(|e| e.to_string())
}

#[cfg(not(windows))]
fn load_library() -> Result<Library, String> {
    unsafe { Library::new("libpcap.so.0.8").or_else(|_| Library::new("libpcap.so")) }
        .map_err(|e| e.to_string())
}

impl PcapLib {
    fn load() -> Result<Self, String> {
        let lib = load_library()?;
        unsafe {
            macro_rules! sym {
                ($name:literal) => {
                    *lib.get(concat!($name, "\0").as_bytes()).map_err(|e| format!("{}: {e}", $name))?
                };
            }
            Ok(Self {
                findalldevs: sym!("pcap_findalldevs"),
                freealldevs: sym!("pcap_freealldevs"),
                open_live: sym!("pcap_open_live"),
                close: sym!("pcap_close"),
                next_ex: sym!("pcap_next_ex"),
                datalink: sym!("pcap_datalink"),
                _lib: lib,
            })
        }
    }

    /// `(name, has an IPv4 address)` of every non-loopback device.
    fn devices(&self) -> Result<Vec<String>, String> {
        let mut head: *mut PcapIf = ptr::null_mut();
        let mut err = [0 as c_char; 256];
        if unsafe { (self.findalldevs)(&mut head, err.as_mut_ptr()) } != 0 {
            let msg = unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy().into_owned();
            return Err(msg);
        }
        let mut out = Vec::new();
        let mut dev = head;
        while !dev.is_null() {
            let d = unsafe { &*dev };
            let mut has_ipv4 = false;
            let mut a = d.addresses;
            while !a.is_null() {
                let ad = unsafe { &*a };
                if !ad.addr.is_null() && unsafe { (*ad.addr).sa_family } == AF_INET {
                    let ip = unsafe { (*ad.addr).sa_data };
                    // Skip link-local 169.254.x.x
                    has_ipv4 |= !(ip[2] == 169 && ip[3] == 254);
                }
                a = ad.next;
            }
            if has_ipv4 && d.flags & PCAP_IF_LOOPBACK == 0 && !d.name.is_null() {
                out.push(unsafe { CStr::from_ptr(d.name) }.to_string_lossy().into_owned());
            }
            dev = d.next;
        }
        unsafe { (self.freealldevs)(head) };
        Ok(out)
    }
}

/// Where the IPv4 header starts for a pcap link type.
fn ip_offset(link: c_int, frame: &[u8]) -> Option<usize> {
    let be16 = |at: usize| frame.get(at..at + 2).map(|b| u16::from_be_bytes([b[0], b[1]]));
    match link {
        1 => {
            // Ethernet, up to two VLAN tags.
            let mut at = 12;
            for _ in 0..2 {
                if matches!(be16(at)?, 0x8100 | 0x88A8) {
                    at += 4;
                }
            }
            (be16(at)? == 0x0800).then_some(at + 2)
        }
        0 => (frame.get(..4)? == [2, 0, 0, 0]).then_some(4), // Npcap loopback
        12 | 14 | 101 | 228 => Some(0),                       // raw IP (VPN, tunnels)
        _ => None,
    }
}

fn parse_segment(frame: &[u8], link: c_int) -> Option<Segment> {
    let ip = ip_offset(link, frame)?;
    let h = frame.get(ip..)?;
    if h.len() < 20 || h[0] >> 4 != 4 || h[9] != 6 {
        return None;
    }
    let ihl = (h[0] & 0x0F) as usize * 4;
    let tcp = h.get(ihl..)?;
    if tcp.len() < 20 {
        return None;
    }
    let thl = (tcp[12] >> 4) as usize * 4;
    // The payload ends where the IP packet does: Ethernet pads short frames and
    // some cards pad with leftovers, not zeros.
    let total = u16::from_be_bytes([h[2], h[3]]) as usize;
    let end = if total >= ihl + 20 && total <= h.len() { total } else { h.len() };
    let payload = h.get(ihl + thl..end)?;
    if payload.is_empty() {
        return None;
    }
    Some(Segment {
        src_ip: [h[12], h[13], h[14], h[15]],
        src_port: u16::from_be_bytes([tcp[0], tcp[1]]),
        dst_ip: [h[16], h[17], h[18], h[19]],
        dst_port: u16::from_be_bytes([tcp[2], tcp[3]]),
        seq: u32::from_be_bytes([tcp[4], tcp[5], tcp[6], tcp[7]]),
        payload: payload.to_vec(),
    })
}

pub fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// Open every usable device and stream TCP segments into `tx` from background
/// threads until `running` is cleared. Returns how many devices were opened.
pub fn start(tx: Sender<Segment>, running: Arc<AtomicBool>) -> Result<usize, CaptureError> {
    let lib = Arc::new(PcapLib::load().map_err(CaptureError::LibraryMissing)?);
    let devices = lib.devices().map_err(CaptureError::Other)?;
    if devices.is_empty() {
        return Err(CaptureError::NoDevices);
    }

    let opened = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
    for name in &devices {
        let (lib, tx, running, name) = (lib.clone(), tx.clone(), running.clone(), name.clone());
        let (opened, ready_tx) = (opened.clone(), ready_tx.clone());
        std::thread::spawn(move || {
            let Ok(c_name) = CString::new(name.clone()) else {
                let _ = ready_tx.send(Err("bad device name".into()));
                return;
            };
            let mut err = [0 as c_char; 256];
            // snaplen 65535, not promiscuous (only this PC's traffic), 100 ms timeout
            let h = unsafe { (lib.open_live)(c_name.as_ptr(), 65535, 0, 100, err.as_mut_ptr()) };
            if h.is_null() {
                let msg = unsafe { CStr::from_ptr(err.as_ptr()) }.to_string_lossy().into_owned();
                let _ = ready_tx.send(Err(msg));
                return;
            }
            opened.fetch_add(1, Ordering::SeqCst);
            let _ = ready_tx.send(Ok(()));
            let link = unsafe { (lib.datalink)(h) };
            while running.load(Ordering::SeqCst) {
                let mut hdr: *mut PcapPkthdr = ptr::null_mut();
                let mut data: *const u8 = ptr::null();
                match unsafe { (lib.next_ex)(h, &mut hdr, &mut data) } {
                    1 => {
                        let n = unsafe { (*hdr).caplen } as usize;
                        let frame = unsafe { std::slice::from_raw_parts(data, n) };
                        if let Some(seg) = parse_segment(frame, link) {
                            if tx.send(seg).is_err() {
                                break;
                            }
                        }
                    }
                    0 => continue, // timeout
                    _ => break,
                }
            }
            unsafe { (lib.close)(h) };
        });
    }
    drop(ready_tx);

    let mut last_err = String::new();
    for _ in 0..devices.len() {
        if let Ok(Err(e)) = ready_rx.recv() {
            last_err = e;
        }
    }
    match opened.load(Ordering::SeqCst) {
        0 => Err(CaptureError::Other(if last_err.is_empty() { "no device could be opened".into() } else { last_err })),
        n => Ok(n),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_ethernet_tcp_frame_and_cuts_padding() {
        // Ethernet II + IPv4 + TCP, payload "hi", then 4 junk padding bytes.
        let mut f = vec![0u8; 12];
        f.extend([0x08, 0x00]);
        f.extend([0x45, 0, 0, 42, 0, 0, 0, 0, 64, 6, 0, 0, 193, 202, 112, 126, 192, 168, 1, 100]);
        f.extend([0x34, 0x10, 0xC9, 0x45, 0, 0, 0, 7, 0, 0, 0, 0, 0x50, 0x18, 0, 0, 0, 0, 0, 0]);
        f.extend(b"hi");
        f.extend([0xBE, 0xDE, 0, 3]);
        let s = parse_segment(&f, 1).unwrap();
        assert_eq!((s.src_port, s.dst_port, s.seq), (13328, 51525, 7));
        assert_eq!(s.src_ip, [193, 202, 112, 126]);
        assert_eq!(s.payload, b"hi");
    }
}
