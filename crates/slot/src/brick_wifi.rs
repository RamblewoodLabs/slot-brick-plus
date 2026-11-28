//! Brick Wi-Fi settings. Uses the firmware supplicant; no shell interpolation of credentials.
use std::{collections::HashSet,fs,os::unix::{net::UnixDatagram,fs::PermissionsExt},path::{Path,PathBuf},process::{Command,Stdio},sync::{mpsc,Arc,atomic::{AtomicBool,AtomicU64,Ordering}},thread,time::{Duration,Instant}};
use slot_input::{Action,Btn};
use slot_store::Platform;
use slot_ui::{TexId,UndoFace};
use crate::cart_art::{ArtJob,Fetched,Progress};
use crate::ra;
type Result<T> = std::result::Result<T,String>;
static SERIAL:AtomicU64=AtomicU64::new(0);
#[derive(Clone,Copy,PartialEq,Eq,Debug)]
enum Security {Open,Personal,Unsupported}
#[derive(Clone,Debug)]
struct Network {ssid:Vec<u8>,signal:i32,security:Security,saved:Option<u32>}
fn decode(s:&str)->Vec<u8> {
    let b=s.as_bytes();let mut v=Vec::new();let mut i=0;
    while i<b.len(){
        if b[i]==b'\\' && i+1<b.len(){
            if b[i+1]==b'x' && i+3<b.len(){if let Some(n)=std::str::from_utf8(&b[i+2..i+4]).ok().and_then(|s|u8::from_str_radix(s,16).ok()){v.push(n);i+=4;continue}}
            let c=match b[i+1]{b'\\'=>Some(b'\\'),b'"'=>Some(b'"'),b'n'=>Some(b'\n'),b't'=>Some(b'\t'),b'r'=>Some(b'\r'),_=>None};
            if let Some(c)=c {v.push(c);i+=2;continue}
        }v.push(b[i]);i+=1;
    }v
}
fn name(b:&[u8])->String{String::from_utf8_lossy(b).chars().map(|c|if c.is_control(){' '}else{c}).collect()}
fn hex(b:&[u8])->String{b.iter().map(|b|format!("{b:02x}")).collect()}
fn psk(s:&str)->Result<String>{
    if s.len()==64 && s.bytes().all(|b|b.is_ascii_hexdigit()){return Ok(s.into())}
    if !(8..=63).contains(&s.len()) || !s.bytes().all(|b|(32..=126).contains(&b)){return Err("Use 8-63 characters or 64 hex digits".into())}
    // wpa_config_parse_psk takes the literal bytes between the first and last quote.
    // Backslashes and interior quotes must NOT be C-escaped here.
    Ok(format!("\"{s}\""))
}
fn scan_rows(scan:&str,saved:&str)->Vec<Network>{
    let saved:Vec<_>=saved.lines().skip(1).filter_map(|s|{let v:Vec<_>=s.split('\t').collect();Some((v.first()?.parse::<u32>().ok()?,decode(v.get(1)?)))}).collect();
    let mut out:Vec<Network>=Vec::new();
    for s in scan.lines().skip(1){let v:Vec<_>=s.splitn(5,'\t').collect();if v.len()!=5{continue}let ssid=decode(v[4]);if ssid.is_empty() || ssid.len()>32{continue}
        let security=if v[3].contains("PSK"){Security::Personal}else if ["WPA","WEP","EAP","SAE","OWE","RSN"].iter().any(|s|v[3].contains(s)){Security::Unsupported}else{Security::Open};
        let n=Network{saved:saved.iter().find(|(_,s)|s==&ssid).map(|(id,_)|*id),ssid,security,signal:v[2].parse().unwrap_or(-100)};
        if let Some(old)=out.iter_mut().find(|a|a.ssid==n.ssid && a.security==n.security){if old.signal<n.signal{*old=n}}else{out.push(n)}
    }out.sort_by(|a,b|b.signal.cmp(&a.signal).then(a.ssid.cmp(&b.ssid)));out
}
pub(crate) trait Supplicant {
    /// The supplicant's answer as it is, FAIL... included.
    fn raw(&self,s:&str)->Result<String>;
    /// Its answer, with FAIL... and UNKNOWN... as errors.
    fn command(&self,s:&str)->Result<String>{
        let s=self.raw(s)?;
        if s.starts_with("FAIL") || s.starts_with("UNKNOWN"){Err("Wi-Fi service rejected the request".into())}else{Ok(s)}
    }
    fn ok(&self,s:&str)->Result<()>{if self.command(s)?=="OK"{Ok(())}else{Err("Wi-Fi service rejected the request".into())}}
}
pub(crate) struct Control{socket:UnixDatagram,local:PathBuf}
impl Drop for Control{fn drop(&mut self){let _=fs::remove_file(&self.local);}}
impl Control{
    pub(crate) fn at(remote:&Path)->Result<Self>{
        let local=PathBuf::from(format!("/tmp/slot-wifi-{}-{}",std::process::id(),SERIAL.fetch_add(1,Ordering::Relaxed)));
        let socket=UnixDatagram::bind(&local).map_err(|_|"Cannot open Wi-Fi control socket")?;
        let c=Self{socket,local};fs::set_permissions(&c.local,fs::Permissions::from_mode(0o600)).map_err(|_|"Cannot secure Wi-Fi socket")?;
        c.socket.set_read_timeout(Some(Duration::from_secs(2))).map_err(|_|"Cannot set Wi-Fi timeout")?;
        c.socket.set_write_timeout(Some(Duration::from_secs(2))).map_err(|_|"Cannot set Wi-Fi timeout")?;
        c.socket.connect(remote).map_err(|_|"Wi-Fi service unavailable")?;Ok(c)
    }
    fn open()->Result<Self>{SOCKETS.iter().find_map(|s|Self::at(&slot_power::hw(s)).ok()).ok_or("Wi-Fi service unavailable".into())}
}
impl Supplicant for Control{
    fn raw(&self,s:&str)->Result<String>{
        self.socket.send(s.as_bytes()).map_err(|_|"Wi-Fi command failed")?;let mut b=[0;32768];let n=self.socket.recv(&mut b).map_err(|_|"Wi-Fi service did not respond")?;
        Ok(String::from_utf8_lossy(&b[..n]).trim_end_matches('\n').to_string())
    }
}
/// Starts a scan. A supplicant just started is already scanning by itself (FAIL-BUSY), or
/// joining a saved network, or not yet ready (FAIL): the first two leave results all the
/// same, and the last is asked again for a few seconds. Only then is it a refusal, and the
/// caller restarts Wi-Fi, which would cut a join short.
fn scan(c:&impl Supplicant,pause:impl Fn(u64)->Result<()>)->Result<()>{
    for _ in 0..SCAN_TRIES{
        let reply=c.raw("SCAN")?;
        if matches!(reply.as_str(),"OK"|"FAIL-BUSY"){return Ok(())}
        let status=c.command("STATUS")?;let state=field(&status,"wpa_state").unwrap_or("?");
        eprintln!("wifi: SCAN answered {reply} (wpa_state {state})");
        if matches!(state,"SCANNING"|"AUTHENTICATING"|"ASSOCIATING"|"ASSOCIATED"|"4WAY_HANDSHAKE"|"GROUP_HANDSHAKE"){return Ok(())}
        pause(SCAN_RETRY_MS)?;
    }
    Err("Wi-Fi service rejected the request".into())
}
const SCAN_TRIES:u32=10;
const SCAN_RETRY_MS:u64=500;
fn field<'a>(s:&'a str,key:&str)->Option<&'a str>{s.lines().find_map(|s|s.strip_prefix(key).and_then(|s|s.strip_prefix('=')))}
fn pause(cancel:&AtomicBool,ms:u64)->Result<()>{for _ in 0..(ms+99)/100{if cancel.load(Ordering::Relaxed){return Err("Cancelled".into())}thread::sleep(Duration::from_millis(100));}Ok(())}
/// Slot's own System/brick_wifi.sh always wins: it writes Logs/wifi.log and tries the
/// firmware's /etc/wifi/wifi_init.sh itself, falling back to its own start when that fails.
/// The firmware helper is only run directly when the card copy is missing.
fn helper(firmware:&Path,root:&Path)->Option<PathBuf>{
    let own=root.join("System/brick_wifi.sh");if own.is_file(){return Some(own)}
    firmware.is_file().then(||firmware.into())
}
fn card_root()->PathBuf{PathBuf::from(std::env::var_os("SLOT_ROOT").unwrap_or_else(||"/mnt/SDCARD/Slot".into()))}
fn service(on:bool,cancel:&AtomicBool)->Result<()>{helper_run(if on{"start"}else{"stop"},cancel)}
/// Control sockets slot can reach a supplicant on, in its search order.
const SOCKETS:[&str;3]=["/etc/wifi/sockets/wlan0","/var/run/wpa_supplicant/wlan0","/run/wpa_supplicant/wlan0"];
/// Wi-Fi is switched on, whether or not it has reached anything. Cheap: it only looks for the
/// supplicant's socket. The shelf asks a few times a second, so it must stay cheap.
pub fn radio_on()->bool{supplicant_up()}
/// A supplicant is running: what the menu reports as Wi-Fi on, known without asking it.
fn supplicant_up()->bool{SOCKETS.iter().any(|s|slot_power::hw(s).exists())}
/// A supplicant that answers: the firmware's socket appears a while before it does.
fn supplicant_answers()->bool{Control::open().and_then(|c|c.command("PING")).is_ok_and(|r|r=="PONG")}
/// One helper at a time: the boot repair and the menu may both ask, and two helpers
/// stopping and starting the same supplicant would undo each other.
static HELPER:std::sync::Mutex<()>=std::sync::Mutex::new(());
/// At startup: Wi-Fi as it was left. The firmware may start a supplicant with the radio still
/// blocked (saved networks are then never joined), or, with slot in place of its menu, start
/// none at all. So wait a little for its own, repair that, or else start Wi-Fi here, as
/// opening the Wi-Fi menu and turning it on would.
/// Never in the way of startup: everything, even reading whether Wi-Fi is wanted, happens on
/// its own thread, and nothing waits for it.
pub fn repair_at_boot(){
    let spawned=thread::Builder::new().name("slot-wifi-boot".into()).spawn(||{
        if !wanted(&card_root()){eprintln!("wifi: boot: Wi-Fi was turned off; leaving it off");return}
        let quiet=AtomicBool::new(false);
        // Slot OS starts no supplicant of its own: nothing to wait for.
        let wait=if slot_power::slot_os(){0}else{FIRMWARE_WAIT_MS};
        STARTING.store(true,Ordering::Relaxed);
        boot_repair(wait,supplicant_answers,online,|up|{
            // Turned off in the menu meanwhile (asked again once the helper is ours, as the
            // menu's own off may have run while this waited): the player's choice stands.
            let r=helper_run_if(if up{"ensure"}else{"start"},&quiet,||wanted(&card_root()));STARTING.store(false,Ordering::Relaxed);r},|ms|thread::sleep(Duration::from_millis(ms)));
        STARTING.store(false,Ordering::Relaxed);
    });
    if let Err(e)=spawned{eprintln!("wifi: boot repair: {e}");}
}
/// Joined a network with no route out of it: after waking from sleep the DHCP client can be
/// left without an address, and Wi-Fi then says connected while nothing online works. Asked
/// every few seconds while Wi-Fi is on but offline; on its own thread, as the supplicant is.
pub fn keep_lease(){
    if STARTING.load(Ordering::Relaxed) || LEASE_CHECKING.swap(true,Ordering::AcqRel){return}
    let spawned=thread::Builder::new().name("slot-wifi-lease".into()).spawn(||{
        let joined=Control::open().and_then(|c|c.command("STATUS")).is_ok_and(|s|field(&s,"wpa_state")==Some("COMPLETED"));
        let stuck=LEASE.lock().unwrap_or_else(|e|e.into_inner()).stuck(joined,online(),Instant::now());
        if stuck{
            eprintln!("wifi: joined but offline for {} s; asking for an address again",LEASE_STUCK.as_secs());
            if let Err(e)=helper_run("renew",&AtomicBool::new(false)){eprintln!("wifi: renew: {e}");}
        }
        LEASE_CHECKING.store(false,Ordering::Release);
    });
    if let Err(e)=spawned{eprintln!("wifi: lease check: {e}");LEASE_CHECKING.store(false,Ordering::Release);}
}
static LEASE_CHECKING:AtomicBool=AtomicBool::new(false);
static LEASE:std::sync::Mutex<LeaseWatch>=std::sync::Mutex::new(LeaseWatch{since:None,renewed:None});
/// How long joined-but-offline lasts before the address is asked for again (a DHCP client that
/// works answers within a few seconds of joining), and the least time between two asks.
const LEASE_STUCK:Duration=Duration::from_secs(15);
const LEASE_RENEW_GAP:Duration=Duration::from_secs(60);
struct LeaseWatch{since:Option<Instant>,renewed:Option<Instant>}
impl LeaseWatch{
    /// Whether to ask for an address again: joined, with no route, for LEASE_STUCK, and not
    /// asked within LEASE_RENEW_GAP.
    fn stuck(&mut self,joined:bool,routed:bool,now:Instant)->bool{
        if !joined || routed {self.since=None;return false}
        let since=*self.since.get_or_insert(now);
        if now.duration_since(since)<LEASE_STUCK || self.renewed.is_some_and(|r|now.duration_since(r)<LEASE_RENEW_GAP){return false}
        self.renewed=Some(now);true
    }
}
/// Startup is bringing Wi-Fi back on: until it has, the menu says so rather than "off".
static STARTING:AtomicBool=AtomicBool::new(false);
/// Where the Wi-Fi menu's on/off is kept, so startup brings Wi-Fi back as it was.
const CHOICE_FILE:&str="Config/wifi.txt";
/// Wi-Fi was left on: as last turned on or off in the menu, else whenever a network is saved.
fn wanted(root:&Path)->bool{
    match fs::read_to_string(root.join(CHOICE_FILE)).map(|s|s.trim().to_string()).as_deref(){
        Ok("off")=>false,
        Ok("on")=>true,
        _=>[slot_power::hw("/etc/wifi/wpa_supplicant.conf"),root.join("Config/wpa_supplicant.conf")].iter()
            .any(|p|fs::read_to_string(p).is_ok_and(|s|s.lines().any(|l|l.trim_start().starts_with("network="))))
    }
}
fn remember(on:bool){
    let path=card_root().join(CHOICE_FILE);
    if let Err(e)=fs::write(&path,if on{"on\n"}else{"off\n"}){eprintln!("wifi: {}: {e}",path.display());}
}
/// How long startup waits for the firmware's own supplicant, and for a saved network to join.
const FIRMWARE_WAIT_MS:u64=10_000;
const JOIN_WAIT_MS:u64=20_000;
const BOOT_POLL_MS:u64=1_000;
/// Give the firmware a moment to start its supplicant; then repair it, or start one
/// (`bring_up(up)`). A repair that fails (the supplicant not ready after all) is tried again
/// shortly; one that works but leaves Wi-Fi offline once more after a while.
fn boot_repair(firmware_wait:u64,up:impl Fn()->bool,online:impl Fn()->bool,mut bring_up:impl FnMut(bool)->Result<()>,sleep:impl Fn(u64)){
    let mut waited=0;
    while !up() && waited<firmware_wait{sleep(BOOT_POLL_MS);waited+=BOOT_POLL_MS;}
    for attempt in 1..=BOOT_TRIES{
        let was_up=up();
        if !was_up{eprintln!("wifi: boot: no supplicant answering; starting Wi-Fi");}
        if let Err(e)=bring_up(was_up){eprintln!("wifi: boot repair {attempt}: {e}");sleep(RETRY_MS);continue}
        let mut waited=0;
        while waited<JOIN_WAIT_MS{
            if online(){eprintln!("wifi: boot repair: online");return}
            sleep(BOOT_POLL_MS);waited+=BOOT_POLL_MS;
        }
    }
    eprintln!("wifi: boot repair: not online (no saved network in range?)");
}
const BOOT_TRIES:u32=3;
const RETRY_MS:u64=3_000;
fn helper_run(verb:&str,cancel:&AtomicBool)->Result<()>{helper_run_if(verb,cancel,||true)}
/// As helper_run, but only if `still` holds once no other helper is running.
fn helper_run_if(verb:&str,cancel:&AtomicBool,still:impl Fn()->bool)->Result<()>{
    let _one=HELPER.lock().unwrap_or_else(|e|e.into_inner());
    if !still(){return Ok(())}
    let root=card_root();
    let Some(script)=helper(&slot_power::hw("/etc/wifi/wifi_init.sh"),&root) else {return Err("Wi-Fi helper missing: System/brick_wifi.sh".into())};
    // Only slot's own helper knows "ensure", "dhcp" and "renew"; a firmware one would just print its usage.
    if matches!(verb,"ensure"|"dhcp"|"renew") && !script.ends_with("System/brick_wifi.sh"){return Ok(())}
    // Inherit slot's stdout/stderr (Logs/slot.log): brick_wifi.sh moves itself to Logs/wifi.log,
    // and anything else, such as a firmware helper run directly, still leaves a trace.
    eprintln!("wifi: {} {verb}",script.display());
    let mut child=Command::new("/bin/sh").arg(&script).arg(verb).env("SLOT_ROOT",&root).stdin(Stdio::null()).spawn().map_err(|_|"Cannot start Wi-Fi helper")?;
    let start=Instant::now();loop{
        if let Some(status)=child.try_wait().map_err(|_|"Cannot check Wi-Fi helper")?{
            eprintln!("wifi: helper {verb} ended: {status}");
            return if status.success(){Ok(())}else{Err("Could not switch Wi-Fi; see Slot/Logs/wifi.log".into())}
        }
        if cancel.load(Ordering::Relaxed){let _=child.kill();let _=child.wait();eprintln!("wifi: helper {verb} cancelled");return Err("Cancelled".into())}
        if start.elapsed()>Duration::from_secs(30){let _=child.kill();let _=child.wait();eprintln!("wifi: helper {verb} timed out");return Err("Wi-Fi helper timed out; see Slot/Logs/wifi.log".into())}
        thread::sleep(Duration::from_millis(100));
    }
}
fn connect(c:&impl Supplicant,n:&Network,password:Option<String>,cancel:&AtomicBool)->Result<String>{
    let old=c.command("LIST_NETWORKS")?;
    let previous:Vec<(u32,bool,bool)>=old.lines().skip(1).filter_map(|s|{let v:Vec<_>=s.split('\t').collect();Some((v.first()?.parse().ok()?,!v.last()?.contains("[DISABLED]"),v.last()?.contains("[CURRENT]")))}).collect();
    let fresh=password.is_some() || n.saved.is_none();
    let secret=if fresh && n.security==Security::Personal{Some(psk(password.as_deref().unwrap_or(""))?)}else{None};
    let id=if fresh{c.command("ADD_NETWORK")?.parse::<u32>().map_err(|_|"Cannot create Wi-Fi profile")?}else{n.saved.unwrap()};
    let result=(||{
        if fresh{c.ok(&format!("SET_NETWORK {id} ssid {}",hex(&n.ssid)))?;c.ok(&format!("SET_NETWORK {id} key_mgmt {}",if n.security==Security::Open{"NONE"}else{"WPA-PSK"}))?;if let Some(secret)=secret{c.ok(&format!("SET_NETWORK {id} psk {secret}"))?;}}
        c.ok(&format!("SELECT_NETWORK {id}"))?;
        let deadline=Instant::now()+Duration::from_secs(20);
        while Instant::now()<deadline{
            pause(cancel,500)?;let s=c.command("STATUS")?;
            if field(&s,"id").and_then(|s|s.parse::<u32>().ok())==Some(id) && field(&s,"wpa_state")==Some("COMPLETED"){
                // Restore eligibility of previously enabled saved networks.
                for &(old,on,_) in &previous{if on{c.ok(&format!("ENABLE_NETWORK {old}"))?;}}
                // Keep the previous profile until a successful replacement has been saved.
                if c.ok("SAVE_CONFIG").is_err(){return Ok("Connected; firmware could not save profile".into())}
                if fresh{if let Some(old)=n.saved{if old!=id && c.ok(&format!("REMOVE_NETWORK {old}")).is_ok(){let _=c.ok("SAVE_CONFIG");}}}
                return Ok(if field(&s,"ip_address").is_some(){"Connected; network saved"}else{"Connected; waiting for IP address"}.into())
            }
        }Err("Connection failed. Check password / signal".into())
    })();
    if result.is_err(){
        if fresh{let _=c.ok(&format!("REMOVE_NETWORK {id}"));}
        if let Some(&(id,_,_))=previous.iter().find(|(_,_,current)|*current){let _=c.ok(&format!("SELECT_NETWORK {id}"));}else{let _=c.ok("DISCONNECT");}
        for &(id,on,_) in &previous{let _=c.ok(&format!("{}_NETWORK {id}",if on{"ENABLE"}else{"DISABLE"}));}
    }result
}
/// Scan(true) restarts Wi-Fi once if the supplicant refuses to scan.
enum Job{Open,Status,Scan(bool),Power(bool),Connect(Network,Option<String>),Disconnect}
struct Reply{on:bool,status:String,networks:Option<Vec<Network>>}
fn run(job:Job,cancel:&AtomicBool)->Result<Reply>{
    // The firmware starts a supplicant at boot with the radio still blocked: it answers, so
    // Wi-Fi looks on, but cannot scan. Repair that once when the menu opens.
    if let Job::Open=job{
        if Control::open().is_ok(){if let Err(e)=helper_run("ensure",cancel){eprintln!("wifi: ensure: {e}");}}
        return run(Job::Status,cancel)
    }
    if let Job::Power(on)=job{
        // The player's own choice: no longer "Turning Wi-Fi on..." while startup waits.
        STARTING.store(false,Ordering::Relaxed);
        service(on,cancel)?;
        remember(on);
        if !on{return Ok(Reply{on:false,status:"Wi-Fi off".into(),networks:Some(Vec::new())})}
        for _ in 0..20{if Control::open().is_ok(){break}pause(cancel,250)?;}
        return run(Job::Scan(false),cancel)
    }
    let c=match Control::open(){Ok(c)=>c,Err(e)=>return match job{
        Job::Status if STARTING.load(Ordering::Relaxed)=>Ok(Reply{on:true,status:"Turning Wi-Fi on...".into(),networks:None}),
        Job::Status=>Ok(Reply{on:false,status:"Wi-Fi off / service unavailable".into(),networks:None}),
        _=>Err(e)}};
    let s=c.command("STATUS")?;let connected=field(&s,"wpa_state")==Some("COMPLETED");
    let mut r=Reply{on:true,status:if connected{format!("{}  {}",name(&decode(field(&s,"ssid").unwrap_or("Connected"))),field(&s,"ip_address").unwrap_or("Waiting for IP"))}else{"Not connected".into()},networks:None};
    match job{
        Job::Status=>{},Job::Open|Job::Power(_)=>unreachable!(),
        Job::Scan(retry)=>{
            if let Err(e)=scan(&c,|ms|pause(cancel,ms)){
                if !retry{return Err(e)}
                // One started under a blocked radio answers but refuses scans until restarted,
                // which is what turning Wi-Fi off and on does.
                eprintln!("wifi: scan refused; restarting Wi-Fi");drop(c);service(false,cancel)?;return run(Job::Power(true),cancel)
            }
            pause(cancel,3500)?;r.networks=Some(scan_rows(&c.command("SCAN_RESULTS")?,&c.command("LIST_NETWORKS")?));if r.networks.as_ref().unwrap().is_empty(){r.status="No networks found. Try scanning again".into();}},
        Job::Connect(n,p)=>{
            r.status=connect(&c,&n,p,cancel)?;
            // Joined but no address yet: make sure a DHCP client is asking, then give the
            // router a moment, so "waiting for IP" is not the last word on a working network.
            if r.status.contains("waiting for IP"){
                if let Err(e)=helper_run("dhcp",cancel){eprintln!("wifi: dhcp: {e}");}
                let deadline=Instant::now()+Duration::from_secs(10);
                while Instant::now()<deadline{
                    pause(cancel,500)?;
                    if let Some(ip)=field(&c.command("STATUS")?,"ip_address"){r.status=format!("Connected as {ip}; network saved");break}
                }
            }
            r.networks=Some(scan_rows(&c.command("SCAN_RESULTS")?,&c.command("LIST_NETWORKS")?));
        },
        Job::Disconnect=>{c.ok("DISCONNECT")?;r.status="Disconnected".into();},
    }Ok(r)
}
struct Worker{rx:mpsc::Receiver<Result<Reply>>,cancel:Arc<AtomicBool>}
/// `typed`: when the last character went in; it shows in clear until SHOW_TYPED has passed.
/// What the keyboard is filling in. `password` holds the text, secret or not.
enum KeyTarget{Wifi(Network),RaUser,RaPassword,LabelQuery,SsUser,SsPassword}
struct Keyboard{target:KeyTarget,secret:bool,password:String,key:usize,page:usize,typed:Option<Instant>}
const SHOW_TYPED:Duration=Duration::from_millis(1200);
/// The password as dots, but for the character just typed.
fn masked(k:&Keyboard)->String{
    let n=k.password.chars().count();
    let shown=k.typed.is_some_and(|t|t.elapsed()<SHOW_TYPED);
    if !k.secret{return k.password.chars().skip(n.saturating_sub(30)).collect()}
    let tail=n.saturating_sub(30);
    k.password.chars().enumerate().skip(tail).map(|(i,c)|if shown && i+1==n{c}else{'*'}).collect()
}
/// Three 10x4 pages, Y cycles them: QWERTY under a number row, its shifted twin (US
/// symbols over the numbers), then the rest of the symbols. The last key is space.
const KEYS:[&str;3]=[
    "1234567890qwertyuiopasdfghjkl-zxcvbnm_. ",
    "!@#$%^&*()QWERTYUIOPASDFGHJKL+ZXCVBNM?, ",
    "1234567890-_=+[]{}\\|;:'\"<>,./?`~!@#$%^& ",
];
const PAGES:[&str;3]=["abc","ABC","#!?"];
/// Rows above the network list: Wi-Fi on/off, scan, disconnect.
const FIXED:usize=3;
/// Wi-Fi is usable for downloads: wlan0 is associated. Cheap enough to check every tick.
/// Online: wlan0 has a way out (a default route, set when DHCP hands out an address). Joined
/// to a network alone is not enough: with no address nothing can be reached.
pub fn online()->bool{fs::read_to_string(slot_power::hw("/proc/net/route")).is_ok_and(|s|routed(&s))}
fn routed(table:&str)->bool{
    table.lines().skip(1).any(|l|{let f:Vec<&str>=l.split_whitespace().collect();f.len()>1&&f[0]=="wlan0"&&f[1]=="00000000"})
}
/// The Cart Art screen: row 0 is the ScreenScraper login, row 1 fetches art, the rest are games
/// with a label, to delete. `games`: platform, stem, label, and whether it is the Famicom label.
/// `user`: the ScreenScraper account saved on the card; `login`: its screen, when open.
struct ArtList{games:Vec<(Platform,String,PathBuf,bool)>,armed:Option<usize>,user:Option<String>,login:Option<SsLogin>}
/// Rows above the games on the Cart Art screen.
const ART_FIXED:usize=2;
/// The ScreenScraper login screen. `password` is only held until the login is tested and saved.
struct SsLogin{user:String,password:String,job:Option<mpsc::Receiver<Result<crate::cart_art_ss::UserInfo>>>,saving:bool}
impl SsLogin{fn new(user:Option<&str>)->Self{Self{user:user.unwrap_or("").to_string(),password:String::new(),job:None,saving:false}}}
impl ArtList{
    fn load()->Self{
        // A NES game's Famicom label is a row of its own, to delete apart from its NES one.
        let mut games:Vec<_>=slot_store::scan(&card_root()).unwrap_or_default().into_iter().flat_map(|c|[c.label.map(|l|(c.platform,c.stem.clone(),l,false)),c.famicom_label.map(|l|(c.platform,c.stem,l,true))]).flatten().collect();
        games.sort_by(|a,b|slot_store::sort_key(&a.1).cmp(&slot_store::sort_key(&b.1)).then((a.0 as u8).cmp(&(b.0 as u8))));
        Self{games,armed:None,user:crate::cart_art_ss::saved_user(&card_root()),login:None}
    }
}
/// What the Cart Art screen says first: how many labels, or what is needed before any are fetched.
fn art_status(list:&ArtList)->String{
    let root=card_root();
    match crate::cart_art::ready(&root){
        Ok(_)=>format!("{} games have a label. A on one twice deletes it",list.games.len()),
        Err(e) if e==crate::cart_art::LOG_IN=>"Log in to ScreenScraper to fetch cart art: A on SCREENSCRAPER".into(),
        Err(e)=>e,
    }
}
/// The RetroAchievements screen: log in, test the connection, show a test unlock, log out.
/// `test`: a pretend unlock waiting for the app to toast it.
const PICK_FIXED:usize=3;
const PICK_GAME_FIXED:usize=2;
/// The Cache achievement sets picker: systems on the card, and which games are ticked.
struct CachePicker{systems:Vec<crate::offline_ra::CacheSystem>,selected:HashSet<crate::offline_ra::CacheKey>,system:usize,avg:Option<u64>}
impl CachePicker{
    fn load(root:&Path)->Self{
        let cat=crate::offline_ra::cache_catalog(root);
        let selected=crate::offline_ra::load_pick(root,&cat.systems);
        Self{systems:cat.systems,selected,system:0,avg:cat.avg_size}
    }
    fn refresh_cached(&mut self,root:&Path){
        let cat=crate::offline_ra::cache_catalog(root);
        self.avg=cat.avg_size;
        self.systems=cat.systems;
    }
    fn selected_count(&self)->usize{self.selected.len()}
    fn game_count(&self)->usize{self.systems.iter().map(|s|s.games.len()).sum()}
    fn select_all(&mut self){
        self.selected=self.systems.iter().flat_map(|s|s.games.iter().map(|g|g.key.clone())).collect();
    }
    fn select_none(&mut self){self.selected.clear();}
    fn system_selected(&self,i:usize)->usize{
        self.systems.get(i).map(|s|s.games.iter().filter(|g|self.selected.contains(&g.key)).count()).unwrap_or(0)
    }
    fn toggle_system(&mut self,i:usize){
        let Some(s)=self.systems.get(i) else{return};
        let keys:Vec<_>=s.games.iter().map(|g|g.key.clone()).collect();
        if keys.is_empty(){return}
        if keys.iter().all(|k|self.selected.contains(k)){
            for k in keys{self.selected.remove(&k);}
        }else{
            self.selected.extend(keys);
        }
    }
    fn toggle_game(&mut self,sys:usize,game:usize){
        let Some(k)=self.systems.get(sys).and_then(|s|s.games.get(game)).map(|g|g.key.clone()) else{return};
        if !self.selected.remove(&k){self.selected.insert(k);}
    }
    fn select_system_all(&mut self,i:usize){
        let Some(s)=self.systems.get(i) else{return};
        self.selected.extend(s.games.iter().map(|g|g.key.clone()));
    }
    fn select_system_none(&mut self,i:usize){
        let Some(s)=self.systems.get(i) else{return};
        for g in &s.games{self.selected.remove(&g.key);}
    }
    fn uncached_keys(&self)->Vec<crate::offline_ra::CacheKey>{
        let mut out=Vec::new();
        for s in &self.systems{
            for g in &s.games{
                if self.selected.contains(&g.key) && !g.cached{out.push(g.key.clone());}
            }
        }
        out
    }
    fn size_label(&self)->String{
        match crate::offline_ra::selection_size(&self.systems,&self.selected,self.avg){
            Some((n,approx)) if n>0=>{
                let s=crate::offline_ra::format_bytes(n);
                if approx{format!("~{s}")}else{s}
            }
            _=>String::new(),
        }
    }
    fn status_line(&self)->String{
        let n=self.selected_count();
        let total=self.game_count();
        let cached=self.systems.iter().flat_map(|s|&s.games).filter(|g|self.selected.contains(&g.key)&&g.cached).count();
        let size=self.size_label();
        let mut s=format!("{n} of {total} game{} selected",if total==1{""}else{"s"});
        if cached>0{s.push_str(&format!(", {cached} already on the card"));}
        if !size.is_empty(){s.push_str(&format!(", {size}"));}
        s
    }
}
struct RaScreen{account:Option<ra::Account>,user:String,password:String,job:Option<mpsc::Receiver<RaDone>>,test:Option<TestUnlock>,page:u8,games:Vec<crate::offline_ra::GameCard>,focus:usize,cache:Option<crate::offline_ra::CacheRun>,pick:Option<CachePicker>}
/// A pretend unlock: title, description, points, and a badge already cached on the card.
pub struct TestUnlock{pub title:String,pub description:String,pub points:u32,pub badge:Option<PathBuf>}
/// Any badge cached on the card, so the test toast looks like the real thing.
fn cached_badge(root:&Path)->Option<PathBuf>{
    let mut v:Vec<PathBuf>=fs::read_dir(root.join("System/Cache/Badges")).ok()?.filter_map(|e|Some(e.ok()?.path())).filter(|p|p.extension().is_some_and(|x|x=="png")).collect();
    v.sort();v.into_iter().next()
}
enum RaDone{LoggedIn(Result<(ra::Account,ra::Profile)>),Checked(Result<ra::Profile>)}
fn profile_line(p:&ra::Profile)->String{format!("Connected in {} ms: {}, {} points ({} softcore)",p.ms,p.user,p.score,p.softcore)}
/// The overlay behind HOME > Wi-Fi, Cart Art and RetroAchievements: `list` is set for
/// Cart Art, `ra` for RetroAchievements.
pub struct WifiMenu{worker:Option<Worker>,pub face:Option<TexId>,pub dirty:bool,on:bool,status:String,networks:Vec<Network>,row:usize,keyboard:Option<Keyboard>,refreshed:Instant,art:Option<ArtJob>,fetched:Vec<Fetched>,list:Option<ArtList>,ra:Option<RaScreen>,query:Option<Option<String>>,update:Option<crate::brick_update::UpdateScreen>}
impl WifiMenu{
    pub fn new()->Self{let mut s=Self{worker:None,face:None,dirty:true,on:supplicant_up(),status:"Checking Wi-Fi...".into(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:None,query:None,update:None};s.start(Job::Open);s}
    /// The keyboard alone, to type what to search labels for: `name` to start from. Closes
    /// when done (START) or left (B); take_label_query then says which.
    pub fn label_query(name:&str)->Self{
        Self{worker:None,face:None,dirty:true,on:true,status:"Type part of a game's name".into(),networks:Vec::new(),row:0,
            keyboard:Some(Keyboard{target:KeyTarget::LabelQuery,secret:false,password:name.chars().take(64).collect(),key:0,page:0,typed:None}),
            refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:None,query:Some(None),update:None}
    }
    /// What was typed to search labels for, once START closed the keyboard.
    pub fn take_label_query(&mut self)->Option<String>{self.query.as_mut()?.take()}
    pub fn cart_art()->Self{
        let list=ArtList::load();
        let status=art_status(&list);
        Self{worker:None,face:None,dirty:true,on:true,status,networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:Some(list),ra:None,query:None,update:None}
    }
    pub fn update(root:&Path,known:Option<crate::update::Manifest>)->Self{
        let mut s=Self::cart_art_free();s.update=Some(crate::brick_update::UpdateScreen::open(root,known));s
    }
    fn cart_art_free()->Self{Self{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:None,query:None,update:None}}
    /// The update is in: Slot should end and start again.
    pub fn take_restart(&mut self)->bool{self.update.as_mut().is_some_and(|u|u.take_restart())}
    pub fn take_update_verdict(&mut self)->Option<Option<crate::update::Manifest>>{self.update.as_mut().and_then(|u|u.take_verdict())}
    pub fn set_battery(&mut self,battery:Option<(u8,bool)>){if let Some(u)=&mut self.update{u.battery=battery;}}
    pub fn achievements()->Self{
        let account=ra::load(&card_root());
        let status=match &account{Some(a)=>format!("Logged in as {}",a.user),None=>"Log in with your RetroAchievements account".into()};
        let games=crate::offline_ra::games(&card_root());
        let waiting=crate::offline_ra::waiting(&card_root());
        let status=if waiting==0{status}else{format!("{status}. {waiting} unlock{} waiting to send",if waiting==1{""}else{"s"})};
        let status=crate::offline_ra::cache_status().unwrap_or(status);
        Self{worker:None,face:None,dirty:true,on:true,status,networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:Some(RaScreen{account,user:String::new(),password:String::new(),job:None,test:None,page:0,games,focus:0,cache:None,pick:None}),query:None,update:None}
    }
    pub fn busy(&self)->bool{self.update.as_ref().is_some_and(|u|u.busy()) || self.worker.is_some() || self.art.is_some() || self.list.as_ref().and_then(|l|l.login.as_ref()).is_some_and(|l|l.job.is_some()) || self.ra.as_ref().is_some_and(|r|r.job.is_some())}
    pub fn cancel(&mut self){
        if let Some(w)=&self.worker{w.cancel.store(true,Ordering::Relaxed);}
        if let Some(a)=&self.art{a.cancel();}
        if let Some(u)=&mut self.update{u.cancel();self.dirty=true;return}
        // A login test that answers after this is dropped unread, so nothing gets saved.
        if let Some(l)=self.list.as_mut().and_then(|l|l.login.as_mut()){if l.job.take().is_some(){self.status="Cancelled".into();self.dirty=true;return}}
        // A reply that comes back after this is dropped unread, so nothing gets saved.
        if let Some(r)=&mut self.ra{
            if r.job.take().is_some(){self.status="Cancelled".into();self.dirty=true;return}
            // Achievement set caching continues in the background if this screen closes.
            r.cache=None;
        }
        if self.busy(){self.status="Cancelling...".into();self.dirty=true;}
    }
    /// Labels saved since the last call, for the library to show straight away.
    pub fn take_fetched(&mut self)->Vec<Fetched>{std::mem::take(&mut self.fetched)}
    /// The test unlock asked for since the last call, for the app to toast.
    pub fn take_test_unlock(&mut self)->Option<TestUnlock>{self.ra.as_mut()?.test.take()}
    fn poll_art(&mut self){
        let Some(job)=&self.art else{return};
        loop{match job.try_recv(){
            Ok(Progress::Step{done,total,name})=>{self.status=format!("Cart art {}/{total}: {name}",done+1);self.dirty=true;}
            Ok(Progress::Got(f))=>self.fetched.push(f),
            Ok(Progress::Done(r))=>{
                self.status=match r{
                    Ok(s) if s.found+s.missing+s.failed==0=>"Every game already has a label".into(),
                    Ok(s)=>format!("Cart art: {} added, {} not found{}",s.found,s.missing,if s.failed>0{format!(", {} failed",s.failed)}else{String::new()}),
                    Err(e)=>e,
                };
                if let Some(l)=&mut self.list{*l=ArtList::load();self.row=self.row.min(l.games.len());}
                self.art=None;self.dirty=true;self.refreshed=Instant::now();return;
            }
            Err(mpsc::TryRecvError::Empty)=>return,
            Err(mpsc::TryRecvError::Disconnected)=>{self.art=None;self.status="Cart art stopped".into();self.dirty=true;return}
        }}
    }
    fn start(&mut self,job:Job){
        if self.busy(){return}let(tx,rx)=mpsc::channel();let cancel=Arc::new(AtomicBool::new(false));let token=cancel.clone();
        if !matches!(job,Job::Status|Job::Open){self.status=match job{Job::Scan(_)=>"Scanning...",Job::Power(true)=>"Turning Wi-Fi on...",Job::Power(false)=>"Turning Wi-Fi off...",Job::Connect(..)=>"Connecting...",Job::Disconnect=>"Disconnecting...",_=>unreachable!()}.into();}
        if !crate::quiet::spawn("slot-wifi", move || { let _ = tx.send(run(job, &token)); }) {
            self.status = "Could not start Wi-Fi".into();
            self.dirty = true;
            return;
        }
        self.worker=Some(Worker{rx,cancel});self.dirty=true;
    }
    pub fn poll(&mut self){
        if let Some(u)=&mut self.update{if u.poll(){self.dirty=true;}return}
        self.poll_art();
        self.poll_login();
        self.poll_ra();
        self.poll_cache();
        if let Some(w)=&self.worker{match w.rx.try_recv(){
            Ok(r)=>{self.worker=None;match r{Ok(r)=>{self.on=r.on;self.status=r.status;if let Some(n)=r.networks{self.networks=n;}},Err(e)=>self.status=e}self.row=self.row.min(self.networks.len()+FIXED-1);self.dirty=true;self.refreshed=Instant::now();},
            Err(mpsc::TryRecvError::Disconnected)=>{self.worker=None;self.status="Wi-Fi worker stopped".into();self.dirty=true;self.refreshed=Instant::now();},_=>{}
        }}
        // Hide the character just typed once its moment is up.
        if let Some(k)=&mut self.keyboard{if k.typed.is_some_and(|t|t.elapsed()>=SHOW_TYPED){k.typed=None;self.dirty=true;}}
        // Poll only while this menu is open; scanning is always explicit. An address still on
        // its way is checked on every couple of seconds, so it shows as soon as it comes.
        let every=if self.status.contains("for IP"){Duration::from_secs(2)}else if self.status.starts_with("Turning Wi-Fi on"){Duration::from_secs(1)}else{Duration::from_secs(15)};
        if self.list.is_none() && self.ra.is_none() && !self.busy() && self.keyboard.is_none() && self.refreshed.elapsed()>every{self.start(Job::Status);}
    }
    pub fn input(&mut self,a:Action)->bool{
        if let Some(u)=&mut self.update{self.dirty=true;return u.input(a)}
        if self.busy(){if matches!(a,Action::GbaDown(Btn::B)){self.cancel();}return false}
        if !matches!(a,Action::GbaDown(_)|Action::ShelfLeft|Action::ShelfRight|Action::Insert){return false}
        self.dirty=true;
        if let Some(k)=&mut self.keyboard{
            match a{
                Action::GbaDown(Btn::B)=>{self.keyboard=None;if self.query.is_some(){return true}},Action::GbaDown(Btn::X)=>{k.password.pop();k.typed=None;},Action::GbaDown(Btn::Y)=>k.page=(k.page+1)%3,
                Action::GbaDown(Btn::Up)=>k.key=(k.key+30)%40,Action::GbaDown(Btn::Down)=>k.key=(k.key+10)%40,
                Action::GbaDown(Btn::Left)|Action::ShelfLeft=>k.key=k.key/10*10+(k.key%10+9)%10,
                Action::GbaDown(Btn::Right)|Action::ShelfRight=>k.key=k.key/10*10+(k.key%10+1)%10,
                Action::GbaDown(Btn::A)|Action::Insert=>{if k.password.len()<64{k.password.push(KEYS[k.page].as_bytes()[k.key] as char);k.typed=Some(Instant::now());}},
                Action::GbaDown(Btn::Start)=>{
                    if let KeyTarget::Wifi(_)=k.target{if let Err(e)=psk(&k.password){self.status=e;return false}}
                    let k=self.keyboard.take().unwrap();
                    match k.target{
                        KeyTarget::Wifi(n)=>self.start(Job::Connect(n,Some(k.password))),
                        KeyTarget::RaUser=>if let Some(ra)=&mut self.ra{ra.user=k.password.trim().to_string();},
                        KeyTarget::RaPassword=>if let Some(ra)=&mut self.ra{ra.password=k.password;},
                        KeyTarget::SsUser=>if let Some(l)=self.list.as_mut().and_then(|l|l.login.as_mut()){l.user=k.password.trim().to_string();},
                        KeyTarget::SsPassword=>if let Some(l)=self.list.as_mut().and_then(|l|l.login.as_mut()){l.password=k.password;},
                        KeyTarget::LabelQuery=>{let q=k.password.trim().to_string();if q.is_empty(){self.keyboard=Some(k);self.status="Type part of a game's name".into();return false}self.query=Some(Some(q));return true}
                    }
                },_=>{}
            }return false
        }
        if self.list.is_some(){return self.art_input(a)}
        if self.ra.is_some(){return self.ra_input(a)}
        match a{
            Action::GbaDown(Btn::B)=>return true,
            Action::GbaDown(Btn::Up)=>self.row=(self.row+self.networks.len()+FIXED-1)%(self.networks.len()+FIXED),
            Action::GbaDown(Btn::Down)=>self.row=(self.row+1)%(self.networks.len()+FIXED),
            Action::GbaDown(Btn::A)|Action::Insert=>match self.row{0=>self.start(Job::Power(!self.on)),1=>self.start(Job::Scan(true)),2=>self.start(Job::Disconnect),_=>self.choose(false)},
            Action::GbaDown(Btn::X)=>self.choose(true),_=>{}
        }false
    }
    fn art_input(&mut self,a:Action)->bool{
        if self.list.as_ref().is_some_and(|l|l.login.is_some()){return self.login_input(a)}
        let Some(list)=&mut self.list else{return true};
        let rows=list.games.len()+ART_FIXED;
        match a{
            Action::GbaDown(Btn::B)=>{if list.armed.take().is_none(){return true}self.status="Kept it".into();}
            Action::GbaDown(Btn::Up)=>{self.row=(self.row+rows-1)%rows;list.armed=None;}
            Action::GbaDown(Btn::Down)=>{self.row=(self.row+1)%rows;list.armed=None;}
            Action::GbaDown(Btn::A)|Action::Insert if self.row==0=>self.open_login(None),
            Action::GbaDown(Btn::A)|Action::Insert if self.row==1=>{
                match crate::cart_art::ready(&card_root()){
                    // No login: straight to where one is entered, rather than a run that can't work.
                    Err(e) if e==crate::cart_art::LOG_IN=>{self.open_login(Some("Enter your ScreenScraper username and password"));return false}
                    Err(e)=>{self.status=e;return false}
                    Ok(_)=>{}
                }
                if !online(){self.status="Connect to Wi-Fi first".into();return false}
                self.status="Cart art: looking up games on ScreenScraper...".into();
                self.art=Some(ArtJob::start(card_root()));
            }
            Action::GbaDown(Btn::A)|Action::Insert=>{
                let at=self.row-ART_FIXED;let Some((platform,stem,path,_))=list.games.get(at).cloned()else{return false};
                if list.armed!=Some(at){list.armed=Some(at);self.status=format!("Press A again to delete the label of {stem}");return false}
                list.armed=None;
                match fs::remove_file(&path){
                    Ok(())=>{
                        eprintln!("slot: cart art: deleted {}",path.display());
                        list.games.remove(at);self.row=self.row.min(list.games.len()+ART_FIXED-1);
                        self.fetched.push(Fetched{platform,stem:stem.clone(),label:None});
                        self.status=format!("Deleted the label of {stem}");
                    }
                    Err(e)=>self.status=format!("Cannot delete label: {e}"),
                }
            }
            _=>{}
        }false
    }
    /// Opens the ScreenScraper login screen, saying `why` (or what it holds).
    fn open_login(&mut self,why:Option<&str>){
        let Some(list)=&mut self.list else{return};
        list.login=Some(SsLogin::new(list.user.as_deref()));self.row=0;
        self.status=match (why,&list.user){
            (Some(w),_)=>w.to_string(),
            (None,Some(u))=>format!("Logged in as {u}"),
            (None,None) if !crate::cart_art_ss::has_developer(&card_root())=>"This Slot was built without a ScreenScraper developer id".into(),
            (None,None)=>"Your free account from screenscraper.fr fetches the art".into(),
        };
    }
    fn login_rows(list:&ArtList)->Vec<String>{
        let Some(l)=&list.login else{return Vec::new()};
        match &list.user{
            Some(u)=>vec![format!("LOGGED IN AS  {u}"),"TEST LOGIN".into(),"SIGN OUT".into()],
            None=>vec![
                format!("USERNAME     {}",if l.user.is_empty(){"-"}else{&l.user}),
                format!("PASSWORD     {}",if l.password.is_empty(){"-".to_string()}else{"*".repeat(l.password.chars().count().min(20))}),
                "TEST LOGIN AND SAVE".into(),
            ],
        }
    }
    fn login_input(&mut self,a:Action)->bool{
        let Some(list)=&mut self.list else{return true};
        let rows=Self::login_rows(list).len().max(1);
        let Some(l)=&mut list.login else{return false};
        match a{
            Action::GbaDown(Btn::B)=>{list.login=None;self.row=0;self.status=art_status(list);}
            Action::GbaDown(Btn::Up)=>self.row=(self.row+rows-1)%rows,
            Action::GbaDown(Btn::Down)=>self.row=(self.row+1)%rows,
            Action::GbaDown(Btn::A)|Action::Insert=>match (list.user.is_some(),self.row){
                (false,0)=>self.keyboard=Some(Keyboard{target:KeyTarget::SsUser,secret:false,password:l.user.clone(),key:0,page:0,typed:None}),
                (false,1)=>self.keyboard=Some(Keyboard{target:KeyTarget::SsPassword,secret:true,password:String::new(),key:0,page:0,typed:None}),
                (false,_)=>{
                    if l.user.is_empty()||l.password.is_empty(){self.status="Enter your username and password first".into();return false}
                    if !online(){self.status="Connect to Wi-Fi first".into();return false}
                    let (root,user,password)=(card_root(),l.user.clone(),l.password.clone());let(tx,rx)=mpsc::channel();
                    if !crate::quiet::spawn("slot-screenscraper",move||{let _=tx.send(crate::cart_art_ss::test_login(&root,&user,&password));}){self.status="Could not start the login".into();return false}
                    l.job=Some(rx);l.saving=true;self.status="Logging in to ScreenScraper...".into();
                }
                (true,0)=>{}
                (true,1)=>{
                    if !online(){self.status="Connect to Wi-Fi first".into();return false}
                    let root=card_root();let(tx,rx)=mpsc::channel();
                    if !crate::quiet::spawn("slot-screenscraper",move||{let _=tx.send(crate::cart_art_ss::test_saved(&root));}){self.status="Could not start the test".into();return false}
                    l.job=Some(rx);l.saving=false;self.status="Testing the login...".into();
                }
                (true,_)=>match crate::cart_art_ss::forget_account(&card_root()){
                    Ok(())=>{eprintln!("slot: screenscraper: signed out");list.user=None;*l=SsLogin::new(None);self.row=0;self.status="Signed out. Log in to fetch cart art".into();}
                    Err(e)=>self.status=format!("Cannot sign out: {e}"),
                },
            },
            _=>{}
        }false
    }
    fn poll_login(&mut self){
        let Some(list)=&mut self.list else{return};
        let Some(l)=&mut list.login else{return};
        let Some(job)=&l.job else{return};
        let done=match job.try_recv(){Ok(d)=>d,Err(mpsc::TryRecvError::Empty)=>return,Err(mpsc::TryRecvError::Disconnected)=>{l.job=None;self.status="The login test stopped".into();self.dirty=true;return}};
        l.job=None;self.dirty=true;
        self.status=match done{
            Ok(info) if l.saving=>match crate::cart_art_ss::save_account(&card_root(),&l.user,&l.password){
                Ok(())=>{eprintln!("slot: screenscraper: logged in");l.password.clear();list.user=Some(l.user.clone());self.row=1;format!("{}. Saved",info.line())}
                Err(e)=>format!("Logged in, but cannot save the login: {e}"),
            },
            Ok(info)=>info.line(),
            Err(e)=>crate::cart_art_ss::redact(&e),
        };
    }
    fn choose(&mut self,edit:bool){
        let Some(n)=self.row.checked_sub(FIXED).and_then(|i|self.networks.get(i)).cloned()else{return};
        if n.security==Security::Unsupported{self.status="Use an open or WPA/WPA2 personal network".into();return}
        if n.security==Security::Personal && (edit || n.saved.is_none()){self.status="Password is case sensitive".into();self.keyboard=Some(Keyboard{target:KeyTarget::Wifi(n),secret:true,password:String::new(),key:0,page:0,typed:None});}else{self.start(Job::Connect(n,None));}
    }
    fn poll_ra(&mut self){
        let Some(ra)=&mut self.ra else{return};
        let Some(job)=&ra.job else{return};
        let done=match job.try_recv(){Ok(d)=>d,Err(mpsc::TryRecvError::Empty)=>return,Err(mpsc::TryRecvError::Disconnected)=>{ra.job=None;self.status="RetroAchievements check stopped".into();self.dirty=true;return}};
        ra.job=None;self.dirty=true;
        self.status=match done{
            RaDone::LoggedIn(Ok((account,profile)))=>match ra::save(&card_root(),&account){
                Ok(())=>{ra.password.clear();ra.account=Some(account);self.row=1;format!("Logged in. {}",profile_line(&profile))}
                Err(e)=>format!("Logged in, but cannot save the login: {e}"),
            },
            RaDone::Checked(Ok(profile))=>profile_line(&profile),
            RaDone::LoggedIn(Err(e))|RaDone::Checked(Err(e))=>e,
        };
    }
    fn poll_cache(&mut self){
        // The cache thread keeps running if this screen closes. Progress is read from the
        // shared status line, so coming back mid-pass shows a live count, never a stale one.
        if crate::offline_ra::cache_busy(){
            if let Some(line)=crate::offline_ra::cache_status(){
                if self.status!=line{self.status=line;self.dirty=true;}
            }
            return;
        }
        if let Some(r)=self.ra.as_mut(){r.cache=None;}
        if let Some((cached,missed,failed))=crate::offline_ra::take_cache_result(){
            if let Some(r)=&mut self.ra{
                r.games=crate::offline_ra::games(&card_root());
                if let Some(p)=r.pick.as_mut(){p.refresh_cached(&card_root());}
            }
            self.status=format!("Cached {cached} sets, {missed} had none, {failed} failed");
            self.dirty=true;
        }
    }
    fn ra_rows(ra:&RaScreen)->Vec<String>{
        if ra.page==3||ra.page==4{
            return ra.pick.as_ref().map(|p|Self::picker_rows(p,ra.page)).unwrap_or_default()
        }
        match ra.page{
            1 if ra.games.is_empty()=>vec!["NO SETS CACHED YET".into()],
            1=>ra.games.iter().map(|g|{
                let got=g.achievements.iter().filter(|a|a.unlocked).count();
                format!("{got}/{}  {}",g.achievements.len(),g.title)
            }).collect(),
            2=>{
                let Some(g)=ra.games.get(ra.focus) else{return vec!["NO SET".into()]};
                if g.achievements.is_empty(){return vec!["NO ACHIEVEMENTS IN THIS SET".into()]}
                g.achievements.iter().map(|a|{
                    let mark=if a.refused{"HELD"}else if a.waiting{"WAIT"}else if a.unlocked{"GOT"}else{"LOCK"};
                    format!("{mark}  {}  {} PTS",a.title,a.points)
                }).collect()
            }
            _=>match &ra.account{
                None=>{
                    let mut rows=vec![
                        format!("USERNAME     {}",if ra.user.is_empty(){"-"}else{&ra.user}),
                        format!("PASSWORD     {}",if ra.password.is_empty(){"-".to_string()}else{"*".repeat(ra.password.chars().count().min(20))}),
                        "LOG IN".into(),
                    ];
                    if !ra.games.is_empty(){rows.push("VIEW ACHIEVEMENTS".into());}
                    rows
                }
                Some(a)=>vec![
                    format!("LOGGED IN AS  {}",a.user),
                    "TEST CONNECTION".into(),
                    "CACHE ACHIEVEMENT SETS".into(),
                    "VIEW ACHIEVEMENTS".into(),
                    "TEST ACHIEVEMENT".into(),
                    "LOG OUT".into(),
                ],
            }
        }
    }
    fn ra_input(&mut self,a:Action)->bool{
        let Some(ra)=&mut self.ra else{return true};
        let rows=Self::ra_rows(ra).len().max(1);
        match a{
            Action::GbaDown(Btn::B)=>{
                if ra.page==4{
                    let sys=ra.pick.as_ref().map(|p|p.system).unwrap_or(0);
                    ra.page=3;self.row=PICK_FIXED+sys;
                    if let Some(p)=ra.pick.as_ref(){self.status=p.status_line();}
                    return false
                }
                if ra.page==3{
                    if let Some(p)=ra.pick.as_ref(){crate::offline_ra::save_pick(&card_root(),&p.systems,&p.selected);}
                    ra.page=0;self.row=2;
                    self.status=ra.pick.as_ref().map(|p|p.status_line()).unwrap_or_else(||"Selection saved".into());
                    ra.pick=None;
                    return false
                }
                if ra.page>0{ra.page-=1;self.row=0;self.status=if ra.page==0{"Achievements saved on the card".into()}else{"Choose a game".into()};return false}
                return true
            }
            Action::GbaDown(Btn::Up)=>{self.row=(self.row+rows-1)%rows;}
            Action::GbaDown(Btn::Down)=>{self.row=(self.row+1)%rows;}
            Action::GbaDown(Btn::Left)|Action::ShelfLeft if ra.page==4=>{
                let sys=ra.pick.as_ref().map(|p|p.system).unwrap_or(0);
                ra.page=3;self.row=PICK_FIXED+sys;
                if let Some(p)=ra.pick.as_ref(){self.status=p.status_line();}
                return false
            }
            Action::GbaDown(Btn::Right)|Action::ShelfRight if ra.page==3 && self.row>=PICK_FIXED=>{
                if let Some(p)=ra.pick.as_mut(){
                    let i=self.row-PICK_FIXED;
                    if i<p.systems.len(){
                        p.system=i;ra.page=4;self.row=0;
                        let got=p.system_selected(i);
                        let n=p.systems[i].games.len();
                        self.status=format!("{}  {got}/{n} selected",p.systems[i].platform.dir_name());
                    }
                }
                return false
            }
            Action::GbaDown(Btn::A)|Action::Insert if ra.page==3=>{
                self.pick_input_systems();
                return false
            }
            Action::GbaDown(Btn::A)|Action::Insert if ra.page==4=>{
                self.pick_input_games();
                return false
            }
            Action::GbaDown(Btn::A)|Action::Insert if ra.page==1=>{
                if ra.games.is_empty(){return false}
                ra.focus=self.row.min(ra.games.len()-1);
                let title=ra.games[ra.focus].title.clone();
                ra.page=2;self.row=0;self.status=title;return false
            }
            Action::GbaDown(Btn::A)|Action::Insert if ra.page==2=>{
                if let Some(ach)=ra.games.get(ra.focus).and_then(|g|g.achievements.get(self.row)){
                    let state=if ach.refused{"Not accepted by the server"}else if ach.waiting{"Earned here, waiting to send"}else if ach.unlocked{"Earned"}else{"Not earned yet"};
                    self.status=format!("{state}. {}  {} points",ach.description,ach.points);
                }
                return false
            }
            Action::GbaDown(Btn::A)|Action::Insert=>match (&ra.account,self.row){
                (None,0)=>self.keyboard=Some(Keyboard{target:KeyTarget::RaUser,secret:false,password:ra.user.clone(),key:0,page:0,typed:None}),
                (None,1)=>self.keyboard=Some(Keyboard{target:KeyTarget::RaPassword,secret:true,password:String::new(),key:0,page:0,typed:None}),
                (None,2)=>{
                    if ra.user.is_empty() || ra.password.is_empty(){self.status="Enter your username and password first".into();return false}
                    if !online(){self.status="Connect to Wi-Fi first".into();return false}
                    let (user,password)=(ra.user.clone(),ra.password.clone());let(tx,rx)=mpsc::channel();
                    if !crate::quiet::spawn("slot-ra", move || { let _ = tx.send(RaDone::LoggedIn(ra::login(&user, &password))); }) {
                        self.status = "Could not start the login".into();
                        return false;
                    }
                    ra.job=Some(rx);self.status="Logging in...".into();
                }
                (None,_)=>{ra.page=1;self.row=0;self.status="Sets saved on the card".into();}
                (Some(_),0)=>{}
                (Some(account),1)=>{
                    if !online(){self.status="Connect to Wi-Fi first".into();return false}
                    let account=account.clone();let(tx,rx)=mpsc::channel();
                    if !crate::quiet::spawn("slot-ra", move || { let _ = tx.send(RaDone::Checked(ra::check(&account))); }) {
                        self.status = "Could not start the check".into();
                        return false;
                    }
                    ra.job=Some(rx);self.status="Testing the connection...".into();
                }
                (Some(_account),2)=>{
                    ra.pick=Some(CachePicker::load(&card_root()));
                    ra.page=3;self.row=0;
                    self.status=ra.pick.as_ref().map(|p|p.status_line()).unwrap_or_else(||"Choose which sets to cache".into());
                }
                (Some(_),3)=>{ra.page=1;self.row=0;self.status="Sets saved on the card".into();}
                (Some(_),4)=>{
                    ra.test=Some(TestUnlock{title:"Test Achievement".into(),description:"Toasts are working".into(),points:5,badge:cached_badge(&card_root())});
                    self.status="Test achievement sent".into();
                }
                (Some(_),_)=>{
                    match ra::forget(&card_root()){
                        Ok(())=>{ra.account=None;self.row=0;self.status="Logged out".into();}
                        Err(e)=>self.status=format!("Cannot log out: {e}"),
                    }
                }
            },
            _=>{}
        }false
    }
    fn picker_rows(p:&CachePicker,page:u8)->Vec<String>{
        if p.systems.is_empty(){return vec!["NO RETROACHIEVEMENTS GAMES ON THE CARD".into()]}
        if page==4{
            let Some(sys)=p.systems.get(p.system) else{return vec!["NO SYSTEM".into()]};
            let mut rows=vec!["SELECT ALL".into(),"SELECT NONE".into()];
            if sys.games.is_empty(){rows.push("NO GAMES".into());return rows}
            for g in &sys.games{
                let mark=if p.selected.contains(&g.key){"[X]"}else{"[ ]"};
                let cached=if g.cached{"  CACHED"}else{""};
                rows.push(format!("{mark}  {}{cached}",g.title));
            }
            rows
        }else{
            let n=p.selected_count();
            let size=p.size_label();
            let cache=if size.is_empty(){format!("CACHE SELECTED    {n}")}else{format!("CACHE SELECTED    {n}    {size}")};
            let mut rows=vec!["SELECT ALL".into(),"SELECT NONE".into(),cache];
            for (i,sys) in p.systems.iter().enumerate(){
                let got=p.system_selected(i);
                let total=sys.games.len();
                let mark=if total>0 && got==total{"[X]"}else if got==0{"[ ]"}else{"[+]"};
                let count=if got==total{format!("{total} game{}",if total==1{""}else{"s"})}else{format!("{got}/{total}")};
                let sizes:Vec<u64>=sys.games.iter().filter(|g|p.selected.contains(&g.key)).filter_map(|g|g.size).collect();
                let unknown=sys.games.iter().filter(|g|p.selected.contains(&g.key)&&g.size.is_none()).count();
                let size=match crate::offline_ra::approx_bytes(&sizes,unknown,p.avg){
                    Some((n,approx)) if n>0=>{
                        let s=crate::offline_ra::format_bytes(n);
                        if approx{format!("    ~{s}")}else{format!("    {s}")}
                    }
                    _=>String::new(),
                };
                rows.push(format!("{mark}  {}    {count}{size}",sys.platform.dir_name()));
            }
            rows
        }
    }
    fn pick_input_systems(&mut self){
        if self.row==2{self.start_selected_cache();return}
        let status={
            let Some(ra)=self.ra.as_mut() else{return};
            let Some(p)=ra.pick.as_mut() else{return};
            if p.systems.is_empty(){return}
            match self.row{
                0=>p.select_all(),
                1=>p.select_none(),
                _=>p.toggle_system(self.row-PICK_FIXED),
            }
            crate::offline_ra::save_pick(&card_root(),&p.systems,&p.selected);
            p.status_line()
        };
        self.status=status;
    }
    fn pick_input_games(&mut self){
        let status={
            let Some(ra)=self.ra.as_mut() else{return};
            let Some(p)=ra.pick.as_mut() else{return};
            let sys=p.system;
            match self.row{
                0=>p.select_system_all(sys),
                1=>p.select_system_none(sys),
                _=>p.toggle_game(sys,self.row-PICK_GAME_FIXED),
            }
            crate::offline_ra::save_pick(&card_root(),&p.systems,&p.selected);
            let got=p.system_selected(sys);
            let n=p.systems.get(sys).map(|s|s.games.len()).unwrap_or(0);
            let name=p.systems.get(sys).map(|s|s.platform.dir_name()).unwrap_or("");
            format!("{name}  {got}/{n} selected")
        };
        self.status=status;
    }
    fn start_selected_cache(&mut self){
        if self.ra.as_ref().and_then(|r|r.account.as_ref()).is_none(){self.status="Log in first".into();return}
        let (user,token,keys,selected)={
            let Some(ra)=self.ra.as_ref() else{return};
            let Some(account)=ra.account.as_ref() else{return};
            let Some(p)=ra.pick.as_ref() else{return};
            crate::offline_ra::save_pick(&card_root(),&p.systems,&p.selected);
            (account.user.clone(),account.token.clone(),p.uncached_keys(),p.selected_count())
        };
        if selected==0{self.status="Choose at least one game".into();return}
        if keys.is_empty(){self.status="All selected sets are already on the card".into();return}
        if !online(){self.status="Connect to Wi-Fi first".into();return}
        if crate::offline_ra::cache_busy(){
            self.status=crate::offline_ra::cache_status().unwrap_or_else(||"Caching achievement sets...".into());
            return
        }
        let root=card_root();
        // Drop the handle: the pass keeps running in the background, and poll_cache
        // reads its live status. Leaving this screen does not stop it.
        let _run=crate::offline_ra::CacheRun::start_keys(root,user,token,keys);
        self.status=crate::offline_ra::cache_status().unwrap_or_else(||"Caching achievement sets...".into());
    }
    fn render_ra(&self,ra:&RaScreen)->UndoFace{
        let mut p=Painter::new();
        let title=match ra.page{
            1=>"ACHIEVEMENT SETS",
            2=>"ACHIEVEMENTS",
            3=>"CACHE SETS",
            4=>ra.pick.as_ref().and_then(|p|p.systems.get(p.system)).map(|s|s.platform.dir_name()).unwrap_or("GAMES"),
            _=>"RETROACHIEVEMENTS",
        };
        p.text(title,32,24,576,30.0);p.text(&self.status,32,66,576,18.0);
        let rows=if ra.page==3||ra.page==4{
            ra.pick.as_ref().map(|p|Self::picker_rows(p,ra.page)).unwrap_or_default()
        }else{Self::ra_rows(ra)};
        let first=if self.row<7{0}else{self.row-6};
        for (line,row) in (first..rows.len().min(first+7)).enumerate(){
            let y=110+line*41;
            if row==self.row{p.rect(0,y,640,38,slot_ui::edge());}
            p.text(&rows[row],32,y+1,576,23.0);
        }
        let legend=if self.busy(){"B CANCEL"}
            else if ra.page==3{if self.row>=PICK_FIXED{"B BACK    A TOGGLE    RIGHT GAMES"}else{"B BACK    A SELECT    RIGHT GAMES"}}
            else if ra.page==4{"B BACK    A TOGGLE"}
            else if ra.page==0{"B BACK    A SELECT"}
            else if ra.page==1{"B BACK    A OPEN"}
            else{"B BACK    A DETAILS"};
        p.text(legend,24,430,592,20.0);
        UndoFace{rgba:p.rgba,w:640,h:480}
    }
    fn render_art(&self,list:&ArtList)->UndoFace{
        let mut p=Painter::new();
        if list.login.is_some(){
            p.text("SCREENSCRAPER",32,24,576,30.0);p.text(&self.status,32,66,576,18.0);
            for (row,label) in Self::login_rows(list).iter().enumerate(){let y=110+row*41;if row==self.row{p.rect(0,y,640,38,slot_ui::edge());}p.text(label,32,y+1,576,23.0);}
            p.text("Free account at screenscraper.fr. Saved in Config/screenscraper.txt",32,250,576,16.0);
            p.text(if self.busy(){"B CANCEL"}else{"B BACK    A SELECT"},24,430,592,20.0);
            return UndoFace{rgba:p.rgba,w:640,h:480}
        }
        p.text("CART ART",32,24,576,30.0);p.text(&self.status,32,66,576,18.0);
        let rows=list.games.len()+ART_FIXED;let first=if self.row<7{0}else{self.row-6};
        for (line,row) in (first..rows.min(first+7)).enumerate(){let y=110+line*41;if row==self.row{p.rect(0,y,640,38,slot_ui::edge());}
            let label=match row{
                0=>format!("SCREENSCRAPER     {}",list.user.as_deref().unwrap_or("NOT LOGGED IN")),
                1=>"GET CART ART".into(),
                _=>{let (platform,stem,_,famicom)=&list.games[row-ART_FIXED];format!("{}{}  {}",if list.armed==Some(row-ART_FIXED){"DELETE?  "}else{""},if *famicom{"FC"}else{platform.dir_name()},stem)}
            };
            p.text(&label,32,y+1,576,23.0);
        }
        p.text(if self.busy(){"B CANCEL"}else if self.row==0{"B BACK    A LOG IN / ACCOUNT"}else if self.row==1{"B BACK    A GET ART"}else if list.armed.is_some(){"B KEEP    A DELETE LABEL"}else{"B BACK    A DELETE LABEL"},24,430,592,20.0);
        UndoFace{rgba:p.rgba,w:640,h:480}
    }
    fn render_update(&self,u:&crate::brick_update::UpdateScreen)->UndoFace{
        let mut p=Painter::new();p.text("UPDATE",32,24,576,30.0);p.text(&u.status,32,66,576,18.0);
        let rows=u.lines();
        for (i,label) in rows.iter().enumerate(){let y=110+i*41;if i==u.row(){p.rect(0,y,640,38,slot_ui::edge());}p.text(label,32,y+1,576,23.0);}
        let (have,notes)=u.about();let mut y=110+rows.len()*41+10;
        p.text(&have,32,y,576,18.0);y+=34;
        let lines:Vec<String>=notes.iter().flat_map(|n|wrap(n,576,17.0)).collect();
        let room=(420-y)/24;let max=lines.len().saturating_sub(room);u.scroll_max.set(max);
        for (i,line) in lines.iter().skip(u.scroll()).take(room).enumerate(){p.text(line,32,y+i*24,576,17.0);}
        let legend=if max>0 && !u.legend().is_empty(){format!("{}    L1/R1 SCROLL",u.legend())}else{u.legend().to_string()};
        p.text(&legend,24,430,592,20.0);
        UndoFace{rgba:p.rgba,w:640,h:480}
    }
    pub fn render(&self)->UndoFace{
        if let Some(u)=&self.update{return self.render_update(u)}
        if self.keyboard.is_none(){
            if let Some(list)=&self.list{return self.render_art(list)}
            if let Some(ra)=&self.ra{return self.render_ra(ra)}
        }
        let mut p=Painter::new();p.text(if self.ra.is_some(){"RETROACHIEVEMENTS"}else if self.list.is_some(){"SCREENSCRAPER"}else if self.query.is_some(){"FIND A LABEL"}else{"WI-FI"},32,24,576,30.0);p.text(&self.status,32,66,576,18.0);
        if let Some(k)=&self.keyboard{
            let title=match &k.target{KeyTarget::Wifi(n)=>name(&n.ssid),KeyTarget::RaUser|KeyTarget::SsUser=>"USERNAME".into(),KeyTarget::RaPassword|KeyTarget::SsPassword=>"PASSWORD".into(),KeyTarget::LabelQuery=>"SEARCH FOR".into()};
            p.text(&title,32,102,576,24.0);p.text(&format!("{}  ({}/64)",masked(k),k.password.len()),32,142,576,21.0);
            for i in 0..40{let x=32+i%10*58;let y=196+i/10*45;if i==k.key{p.rect(x,y,54,41,slot_ui::edge());}let c=&KEYS[k.page][i..i+1];if c==" "{p.text("SPC",x,y+9,54,17.0);}else{p.text(c,x,y+3,54,26.0);}}
            p.text(&format!("A TYPE    X DELETE    Y {}",PAGES.iter().enumerate().map(|(i,n)|if i==k.page{format!("[{n}]")}else{n.to_string()}).collect::<Vec<_>>().join(" / ")),24,392,592,18.0);p.text(match k.target{KeyTarget::Wifi(_)=>"B BACK                    START CONNECT",KeyTarget::LabelQuery=>"B BACK                    START SEARCH",_=>"B BACK                    START DONE"},24,430,592,20.0);
        }else{
            let first=if self.row<7{0}else{self.row-6};
            for (line,row) in (first..(self.networks.len()+FIXED).min(first+7)).enumerate(){let y=110+line*41;if row==self.row{p.rect(0,y,640,38,slot_ui::edge());}
                let label=match row{0=>format!("WI-FI                          {}",if self.on{"ON"}else{"OFF"}),1=>"SCAN NETWORKS".into(),2=>"DISCONNECT".into(),_=>{let n=&self.networks[row-FIXED];format!("{}  {}  {} dBm",name(&n.ssid),if n.saved.is_some(){"[SAVED]"}else if n.security==Security::Open{"[OPEN]"}else{"[LOCK]"},n.signal)}};p.text(&label,32,y+1,576,23.0);
            }p.text(if self.busy(){"B CANCEL"}else{"B BACK    A SELECT    X EDIT PASSWORD"},24,430,592,20.0);
        }UndoFace{rgba:p.rgba,w:640,h:480}
    }
}
/// `text` in lines no wider than `w` at `px`, broken between words.
fn wrap(text:&str,w:usize,px:f32)->Vec<String>{
    let Some(font)=slot_ui::text::label_font()else{return vec![text.to_string()]};
    let mut lines=Vec::new();let mut line=String::new();
    for word in text.split_whitespace(){
        let next=if line.is_empty(){word.to_string()}else{format!("{line} {word}")};
        if !line.is_empty() && slot_ui::text::line_width(font,&next,px,0.0)>w as f32{lines.push(std::mem::replace(&mut line,word.to_string()));}else{line=next;}
    }
    if !line.is_empty(){lines.push(line);}lines
}
struct Painter{rgba:Vec<u8>}
impl Painter{
    fn new()->Self{let mut s=Self{rgba:vec![0;640*480*4]};s.rect(0,0,640,480,slot_ui::opening());s}
    fn rect(&mut self,x:usize,y:usize,w:usize,h:usize,c:[f32;4]){let c=c.map(|v|(v*255.0)as u8);for yy in y..(y+h).min(480){for xx in x..(x+w).min(640){self.rgba[(yy*640+xx)*4..(yy*640+xx)*4+4].copy_from_slice(&c)}}}
    fn text(&mut self,s:&str,x:usize,y:usize,w:usize,px:f32){let Some(font)=slot_ui::text::label_font()else{return};if y>=480||x>=640||w==0{return}let mut s=s.to_string();while slot_ui::text::line_width(font,&s,px,0.0)>w as f32 && s.chars().count()>1{s.pop();}
        let h=40usize;let mut face=vec![0;w.saturating_mul(h).saturating_mul(4)];let layout=slot_ui::text::Layout{lines:vec![s],px,tracking:0.0};slot_ui::text::draw_centred(&mut face,w as u32,h as u32,&layout,[238,238,242]);
        let rows=h.min(480usize.saturating_sub(y));let cols=w.min(640usize.saturating_sub(x));
        for yy in 0..rows{for xx in 0..cols{let a=(yy*w+xx)*4;let b=((y+yy)*640+x+xx)*4;if a+3>=face.len()||b+3>=self.rgba.len(){continue}let alpha=face[a+3]as u32;for ch in 0..3{self.rgba[b+ch]=((face[a+ch]as u32*alpha+self.rgba[b+ch]as u32*(255-alpha))/255)as u8;}}}
    }
}
#[cfg(test)]mod tests{
    use super::*;use std::cell::{Cell,RefCell};
    struct Mock{steps:RefCell<std::collections::VecDeque<(String,Result<String>)>>}
    impl Mock{fn new(steps:Vec<(&str,Result<&str>)>)->Self{Self{steps:RefCell::new(steps.into_iter().map(|(a,b)|(a.into(),b.map(String::from))).collect())}}fn done(&self){assert!(self.steps.borrow().is_empty());}}
    impl Supplicant for Mock{fn raw(&self,s:&str)->Result<String>{let(a,b)=self.steps.borrow_mut().pop_front().expect("unexpected command");assert_eq!(a,s);b}}
    fn network()->Network{Network{ssid:b"Test".to_vec(),signal:-40,security:Security::Personal,saved:None}}
    #[test]fn boot_repair_repairs_a_late_firmware_supplicant(){
        let t=Cell::new(0u64);let runs=RefCell::new(Vec::new());
        // The firmware's supplicant appears 5 s in; a network joins 3 s after the repair.
        boot_repair(FIRMWARE_WAIT_MS,||t.get()>=5_000,||!runs.borrow().is_empty()&&t.get()>=8_000,|up|{runs.borrow_mut().push(up);Ok(())},|ms|t.set(t.get()+ms));
        assert_eq!(*runs.borrow(),vec![true],"repaired the firmware's supplicant once");
    }
    #[test]fn boot_repair_starts_wifi_itself_when_the_firmware_never_does(){
        let t=Cell::new(0u64);let started=Cell::new(false);
        boot_repair(FIRMWARE_WAIT_MS,||started.get(),||started.get()&&t.get()>=FIRMWARE_WAIT_MS+2_000,|up|{assert!(!up);started.set(true);Ok(())},|ms|t.set(t.get()+ms));
        assert!(started.get(),"Wi-Fi was never started");
        assert!(t.get()<=FIRMWARE_WAIT_MS+JOIN_WAIT_MS,"waited past joining");
    }
    #[test]fn on_slot_os_boot_repair_starts_wifi_at_once(){
        let t=Cell::new(0u64);let started=Cell::new(false);
        boot_repair(0,||started.get(),||started.get(),|up|{assert!(!up);started.set(true);Ok(())},|ms|t.set(t.get()+ms));
        assert!(started.get());
        assert_eq!(t.get(),0,"waited for a firmware supplicant there is none of");
    }
    #[test]fn a_scan_already_running_is_not_a_refusal(){
        let m=Mock::new(vec![("SCAN",Ok("FAIL-BUSY")),("SCAN",Ok("OK")),("SCAN",Ok("FAIL")),("STATUS",Ok("wpa_state=ASSOCIATING"))]);
        for _ in 0..3{assert!(scan(&m,|_|Ok(())).is_ok());}m.done();
    }
    #[test]fn a_supplicant_not_ready_is_asked_again_before_it_counts_as_a_refusal(){
        let mut steps=vec![("SCAN",Ok("FAIL")),("STATUS",Ok("wpa_state=INTERFACE_DISABLED")),("SCAN",Ok("OK"))];
        let m=Mock::new(steps.clone());let waited=Cell::new(0);
        assert!(scan(&m,|ms|{waited.set(waited.get()+ms);Ok(())}).is_ok());m.done();
        assert_eq!(waited.get(),SCAN_RETRY_MS);
        steps.clear();for _ in 0..SCAN_TRIES{steps.push(("SCAN",Ok("FAIL")));steps.push(("STATUS",Ok("wpa_state=DISCONNECTED")));}
        let m=Mock::new(steps);assert!(scan(&m,|_|Ok(())).is_err());m.done();
    }
    #[test]fn joined_but_offline_asks_for_an_address_after_a_while_and_not_too_often(){
        let t=Instant::now();let at=|s:u64|t+Duration::from_secs(s);
        let mut w=LeaseWatch{since:None,renewed:None};
        assert!(!w.stuck(true,false,at(0)),"just joined: DHCP gets its moment");
        assert!(!w.stuck(true,false,at(14)));
        assert!(w.stuck(true,false,at(15)),"stuck: ask again");
        assert!(!w.stuck(true,false,at(30)),"asked a moment ago");
        assert!(w.stuck(true,false,at(75)),"still stuck a minute later: ask again");
        // Online, or not joined at all: nothing to fix, and the wait starts over.
        assert!(!w.stuck(true,true,at(200)));
        assert!(!w.stuck(true,false,at(201)));
        assert!(!w.stuck(false,false,at(300)));
        assert!(!w.stuck(true,false,at(301)));
        assert!(w.stuck(true,false,at(316)));
    }
    #[test]fn boot_repair_gives_up_after_its_tries(){
        let t=Cell::new(0u64);let runs=Cell::new(0);
        boot_repair(FIRMWARE_WAIT_MS,||true,||false,|_|{runs.set(runs.get()+1);Ok(())},|ms|t.set(t.get()+ms));
        assert_eq!(runs.get(),BOOT_TRIES);
    }
    #[test]fn a_failed_repair_is_tried_again_soon(){
        // As on the Brick: the first repair finds the supplicant not answering yet.
        let t=Cell::new(0u64);let runs=RefCell::new(Vec::new());let fixed=Cell::new(false);
        boot_repair(FIRMWARE_WAIT_MS,||true,||fixed.get(),|_|{runs.borrow_mut().push(t.get());if runs.borrow().len()==1{Err("no supplicant answering".into())}else{fixed.set(true);Ok(())}},|ms|t.set(t.get()+ms));
        let runs=runs.borrow();
        assert_eq!(runs.len(),2);
        assert!(runs[1]-runs[0]<=RETRY_MS,"waited {} ms to try again",runs[1]-runs[0]);
    }
    #[test]fn joined_without_an_address_is_not_online(){
        let head="Iface\tDestination\tGateway \tFlags\tRefCnt\tUse\tMetric\tMask\t\tMTU\tWindow\tIRTT\n";
        assert!(!routed(head));
        assert!(!routed(&format!("{head}wlan0\t0001A8C0\t00000000\t0001\t0\t0\t0\t00FFFFFF\t0\t0\t0\n")),"a local route only");
        assert!(routed(&format!("{head}wlan0\t00000000\t0101A8C0\t0003\t0\t0\t0\t00000000\t0\t0\t0\n")));
    }
    #[test]fn wifi_comes_back_as_it_was_left(){
        let d=tempfile::tempdir().unwrap();
        fs::create_dir_all(d.path().join("Config")).unwrap();
        fs::write(d.path().join(CHOICE_FILE),"off\n").unwrap();assert!(!wanted(d.path()));
        fs::write(d.path().join(CHOICE_FILE),"on\n").unwrap();assert!(wanted(d.path()));
        fs::remove_file(d.path().join(CHOICE_FILE)).unwrap();
        fs::write(d.path().join("Config/wpa_supplicant.conf"),"ctrl_interface=/var/run/wpa_supplicant\nnetwork={\n ssid=\"Home\"\n}\n").unwrap();
        assert!(wanted(d.path()),"a saved network and no choice yet: Wi-Fi on");
    }
    #[test]fn the_label_keyboard_starts_from_the_name_and_hands_back_what_was_typed(){
        let mut m=WifiMenu::label_query("Yoshi");
        assert!(!m.input(Action::GbaDown(Btn::X)),"deleting closed it");
        assert!(m.input(Action::GbaDown(Btn::Start)),"START did not close it");
        assert_eq!(m.take_label_query(),Some("Yosh".into()));
        assert_eq!(m.take_label_query(),None);
        let mut m=WifiMenu::label_query("Yoshi");
        assert!(m.input(Action::GbaDown(Btn::B)),"B did not close it");
        assert_eq!(m.take_label_query(),None,"B searched");
        let mut m=WifiMenu::label_query("");
        assert!(!m.input(Action::GbaDown(Btn::Start)),"an empty search closed it");
    }
    #[test]fn parse_scans(){let n=scan_rows("header\na\t2412\t-80\t[WPA2-PSK-CCMP]\tMy\\x20WiFi\nb\t2412\t-40\t[WPA2-PSK-CCMP]\tMy WiFi\nc\t2412\t-30\t[ESS]\tCafe\nd\t2412\t-20\t[WPA2-EAP-CCMP]\tWork\n","header\n7\tMy WiFi\tany\t[CURRENT]\n");assert_eq!(n.len(),3);assert_eq!(n[0].security,Security::Unsupported);assert_eq!(n[2].signal,-40);assert_eq!(n[2].saved,Some(7));}
    #[test]fn password_bytes(){assert!(psk("short").is_err());assert!(psk("1234567\n").is_err());assert_eq!(psk("Abc\"\\123").unwrap(),"\"Abc\"\\123\"");assert_eq!(psk(&"a".repeat(64)).unwrap(),"a".repeat(64));assert!(psk(&"z".repeat(64)).is_err());}
    #[test]fn helper_prefers_card_then_firmware(){
        let d=tempfile::tempdir().unwrap();let fw=d.path().join("wifi_init.sh");let root=d.path().join("Slot");
        assert_eq!(helper(&fw,&root),None);
        fs::write(&fw,"").unwrap();assert_eq!(helper(&fw,&root),Some(fw.clone()));
        fs::create_dir_all(root.join("System")).unwrap();fs::write(root.join("System/brick_wifi.sh"),"").unwrap();
        assert_eq!(helper(&fw,&root),Some(root.join("System/brick_wifi.sh")));
    }
    #[test]fn all_keys(){for s in KEYS{assert_eq!(s.len(),40);assert_eq!(s.as_bytes()[39],b' ',"space last on every page");}for b in 32..=126{assert!(KEYS.iter().any(|s|s.as_bytes().contains(&b)),"{b}");}}
    #[test]fn the_last_typed_character_shows_briefly(){
        let mut k=Keyboard{target:KeyTarget::Wifi(network()),secret:true,password:"hunter2".into(),key:0,page:0,typed:Some(Instant::now())};
        assert_eq!(masked(&k),"******2");
        k.typed=Some(Instant::now()-SHOW_TYPED);assert_eq!(masked(&k),"*******");
        k.typed=None;assert_eq!(masked(&k),"*******");
        k.password="x".repeat(40)+"y";k.typed=Some(Instant::now());
        let m=masked(&k);assert_eq!(m.chars().count(),30);assert!(m.ends_with("*y"));
    }
    #[test]fn a_username_is_typed_in_clear(){
        let k=Keyboard{target:KeyTarget::RaUser,secret:false,password:"Benni".into(),key:0,page:0,typed:None};
        assert_eq!(masked(&k),"Benni");
    }
    #[test]fn ra_screen_types_the_login_and_needs_both_fields(){
        let mut m=WifiMenu{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:Some(RaScreen{account:None,user:String::new(),password:String::new(),job:None,test:None,page:0,games:Vec::new(),focus:0,cache:None,pick:None}),query:None,update:None};
        m.input(Action::GbaDown(Btn::A));assert!(m.keyboard.as_ref().is_some_and(|k|!k.secret));
        // q w e on the letter row
        m.input(Action::GbaDown(Btn::Down));for _ in 0..3{m.input(Action::GbaDown(Btn::A));m.input(Action::GbaDown(Btn::Right));}
        m.input(Action::GbaDown(Btn::Start));assert_eq!(m.ra.as_ref().unwrap().user,"qwe");assert!(m.keyboard.is_none());
        m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.status,"Enter your username and password first");assert!(!m.busy());
        m.input(Action::GbaDown(Btn::Up));m.input(Action::GbaDown(Btn::A));assert!(m.keyboard.as_ref().is_some_and(|k|k.secret));
        if let Ok(path)=std::env::var("SLOT_RA_PREVIEW"){m.keyboard=None;m.ra.as_mut().unwrap().password="secret".into();let f=m.render();let mut e=png::Encoder::new(fs::File::create(path).unwrap(),640,480);e.set_color(png::ColorType::Rgba);e.set_depth(png::BitDepth::Eight);e.write_header().unwrap().write_image_data(&f.rgba).unwrap();}
        assert!(!m.input(Action::GbaDown(Btn::B)),"B closes the keyboard first");assert!(m.keyboard.is_none());
        assert!(m.input(Action::GbaDown(Btn::B)),"then the screen");
    }
    #[test]fn test_achievement_queues_a_pretend_unlock_with_a_cached_badge(){
        let dir=std::env::temp_dir().join(format!("slot-ra-test-{}",std::process::id()));let _=fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("System/Cache/Badges")).unwrap();fs::write(dir.join("System/Cache/Badges/b.png"),b"").unwrap();fs::write(dir.join("System/Cache/Badges/a.png"),b"").unwrap();
        assert_eq!(cached_badge(&dir),Some(dir.join("System/Cache/Badges/a.png")));
        assert_eq!(cached_badge(&dir.join("nowhere")),None);let _=fs::remove_dir_all(&dir);
        let account=ra::Account{user:"Benni".into(),token:"t".into()};
        let mut m=WifiMenu{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:Some(RaScreen{account:Some(account),user:String::new(),password:String::new(),job:None,test:None,page:0,games:Vec::new(),focus:0,cache:None,pick:None}),query:None,update:None};
        m.input(Action::GbaDown(Btn::Up));assert_eq!(m.row,5,"six rows when logged in, wrapping to LOG OUT");
        m.input(Action::GbaDown(Btn::Up));assert_eq!(m.row,4);
        assert!(m.take_test_unlock().is_none());
        m.input(Action::GbaDown(Btn::A));
        let t=m.take_test_unlock().unwrap();assert_eq!((t.title.as_str(),t.points),("Test Achievement",5));
        assert!(m.take_test_unlock().is_none(),"taken once");assert!(!m.busy());
    }
    #[test]fn a_cached_set_opens_offline(){
        let card=crate::offline_ra::GameCard{key:"abc".into(),title:"Tetris".into(),game_id:1,place:"GB  Tetris".into(),achievements:vec![crate::offline_ra::Ach{id:5,points:10,badge:"00010".into(),title:"Clear".into(),description:"Clear a line".into(),unlocked:true,waiting:true,refused:false}]};
        let mut m=WifiMenu{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:Some(RaScreen{account:None,user:String::new(),password:String::new(),job:None,test:None,page:0,games:vec![card],focus:0,cache:None,pick:None}),query:None,update:None};
        m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::Down));
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.ra.as_ref().unwrap().page,1);
        m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.ra.as_ref().unwrap().page,2);
        m.input(Action::GbaDown(Btn::A));
        assert!(m.status.contains("waiting"),"{}",m.status);
        assert!(!m.input(Action::GbaDown(Btn::B)));
        assert_eq!(m.ra.as_ref().unwrap().page,1);
    }
    #[test]fn qwerty_under_a_number_row(){
        let rows=|p:usize|KEYS[p].as_bytes().chunks(10).map(|r|String::from_utf8_lossy(r).into_owned()).collect::<Vec<_>>();
        assert_eq!(rows(0),["1234567890","qwertyuiop","asdfghjkl-","zxcvbnm_. "]);
        assert_eq!(rows(1)[1..3],["QWERTYUIOP".to_string(),"ASDFGHJKL+".to_string()]);
        for i in 10..40{let (l,u)=(KEYS[0].as_bytes()[i],KEYS[1].as_bytes()[i]);if l.is_ascii_lowercase(){assert_eq!(u,l.to_ascii_uppercase(),"shift keeps letters in place");}}
    }
    #[test]fn failed_setup_restores_previous_connection(){let m=Mock::new(vec![("LIST_NETWORKS",Ok("header\n3\tOld\tany\t[CURRENT]\n4\tOther\tany\t[DISABLED]")),("ADD_NETWORK",Ok("8")),("SET_NETWORK 8 ssid 54657374",Err("rejected".into())),("REMOVE_NETWORK 8",Ok("OK")),("SELECT_NETWORK 3",Ok("OK")),("ENABLE_NETWORK 3",Ok("OK")),("DISABLE_NETWORK 4",Ok("OK"))]);assert!(connect(&m,&network(),Some("password".into()),&AtomicBool::new(false)).is_err());m.done();}
    #[test]fn cancelled_connect_removes_unsaved_profile(){let m=Mock::new(vec![("LIST_NETWORKS",Ok("header")),("ADD_NETWORK",Ok("8")),("SET_NETWORK 8 ssid 54657374",Ok("OK")),("SET_NETWORK 8 key_mgmt WPA-PSK",Ok("OK")),("SET_NETWORK 8 psk \"password\"",Ok("OK")),("SELECT_NETWORK 8",Ok("OK")),("REMOVE_NETWORK 8",Ok("OK")),("DISCONNECT",Ok("OK"))]);assert_eq!(connect(&m,&network(),Some("password".into()),&AtomicBool::new(true)).unwrap_err(),"Cancelled");m.done();}
    #[test]fn successful_connect_saves_only_after_association(){let m=Mock::new(vec![("LIST_NETWORKS",Ok("header")),("ADD_NETWORK",Ok("8")),("SET_NETWORK 8 ssid 54657374",Ok("OK")),("SET_NETWORK 8 key_mgmt WPA-PSK",Ok("OK")),("SET_NETWORK 8 psk \"password\"",Ok("OK")),("SELECT_NETWORK 8",Ok("OK")),("STATUS",Ok("id=8\nwpa_state=COMPLETED\nip_address=192.168.1.2")),("SAVE_CONFIG",Ok("OK"))]);assert_eq!(connect(&m,&network(),Some("password".into()),&AtomicBool::new(false)).unwrap(),"Connected; network saved");m.done();}
    #[test]#[ignore="Requires a Linux runtime allowing AF_UNIX datagram sockets"]fn socket_roundtrip(){let dir=tempfile::tempdir().unwrap();let remote=dir.path().join("server");let server=UnixDatagram::bind(&remote).unwrap();let t=thread::spawn(move||{let mut b=[0;64];let(n,peer)=server.recv_from(&mut b).unwrap();assert_eq!(&b[..n],b"PING");server.send_to(b"PONG\n",peer.as_pathname().unwrap()).unwrap();});let c=Control::at(&remote).unwrap();let local=c.local.clone();assert_eq!(fs::metadata(&local).unwrap().permissions().mode()&0o777,0o600);assert_eq!(c.command("PING").unwrap(),"PONG");drop(c);assert!(!local.exists());t.join().unwrap();}
    #[test]fn art_screen_deletes_only_after_a_second_a(){
        let d=tempfile::tempdir().unwrap();let label=d.path().join("Emerald.png");fs::write(&label,b"png").unwrap();
        let games=vec![(Platform::Gba,"Emerald".to_string(),label.clone(),false),(Platform::Gb,"Tetris".to_string(),d.path().join("Tetris.png"),false)];
        let mut m=WifiMenu{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:Some(ArtList{games,armed:None,user:None,login:None}),ra:None,query:None,update:None};
        m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::A));
        assert!(label.exists(),"first A only asks");
        if let Ok(path)=std::env::var("SLOT_ART_PREVIEW"){let f=m.render();let mut e=png::Encoder::new(fs::File::create(path).unwrap(),640,480);e.set_color(png::ColorType::Rgba);e.set_depth(png::BitDepth::Eight);e.write_header().unwrap().write_image_data(&f.rgba).unwrap();}
        assert!(!m.input(Action::GbaDown(Btn::B)),"B disarms instead of closing");assert!(label.exists());
        m.input(Action::GbaDown(Btn::A));m.input(Action::GbaDown(Btn::A));
        assert!(!label.exists());assert_eq!(m.take_fetched(),vec![Fetched{platform:Platform::Gba,stem:"Emerald".into(),label:None}]);
        assert_eq!(m.list.as_ref().unwrap().games.len(),1);
        assert!(m.input(Action::GbaDown(Btn::B)),"B closes once nothing is armed");
    }
    #[test]fn render_fixture(){let s=WifiMenu{worker:None,face:None,dirty:true,on:true,status:"Password is case sensitive".into(),networks:Vec::new(),row:0,keyboard:Some(Keyboard{target:KeyTarget::Wifi(network()),secret:true,password:"Test".into(),key:0,page:0,typed:Some(Instant::now())}),refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:None,query:None,update:None};
        let mut s=s;
        // SLOT_WIFI_PREVIEW=<dir>: one picture per keyboard page.
        for page in 0..KEYS.len(){
            if let Some(k)=&mut s.keyboard{k.page=page;k.key=12;}
            let f=s.render();assert_eq!(f.rgba.len(),640*480*4);
            if let Ok(dir)=std::env::var("SLOT_WIFI_PREVIEW"){let mut e=png::Encoder::new(fs::File::create(Path::new(&dir).join(format!("keyboard-{page}.png"))).unwrap(),640,480);e.set_color(png::ColorType::Rgba);e.set_depth(png::BitDepth::Eight);e.write_header().unwrap().write_image_data(&f.rgba).unwrap();}
        }
    }
    fn art_menu(user:Option<&str>)->WifiMenu{
        let games=vec![(Platform::Gb,"Tetris (World)".to_string(),PathBuf::from("/x.png"),false),(Platform::Md,"Sonic The Hedgehog (USA, Europe)".to_string(),PathBuf::from("/y.png"),false)];
        let list=ArtList{games,armed:None,user:user.map(String::from),login:None};
        WifiMenu{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:Some(list),ra:None,query:None,update:None}
    }
    #[test]fn cart_art_opens_on_the_login_and_types_an_account_on_the_keyboard(){
        let mut m=art_menu(None);
        let rows=|m:&WifiMenu|m.list.as_ref().map(|l|WifiMenu::login_rows(l)).unwrap_or_default();
        m.open_login(None);
        assert_eq!(rows(&m)[2],"TEST LOGIN AND SAVE");
        m.input(Action::GbaDown(Btn::A));
        assert!(matches!(m.keyboard.as_ref().map(|k|&k.target),Some(KeyTarget::SsUser)));
        m.input(Action::GbaDown(Btn::A));m.input(Action::GbaDown(Btn::Start));
        assert_eq!(m.list.as_ref().unwrap().login.as_ref().unwrap().user,"1");
        m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::A));
        let k=m.keyboard.as_ref().unwrap();assert!(k.secret,"the password is typed hidden");
        m.input(Action::GbaDown(Btn::A));m.input(Action::GbaDown(Btn::Start));
        assert!(rows(&m)[1].starts_with("PASSWORD     *"));
        m.input(Action::GbaDown(Btn::B));
        assert!(m.list.as_ref().unwrap().login.is_none(),"B leaves the login for the Cart Art list");
        let signed=art_menu(Some("jeff"));
        let mut s=signed;s.open_login(None);
        assert_eq!(rows(&s),["LOGGED IN AS  jeff","TEST LOGIN","SIGN OUT"]);
    }
    /// The Cart Art and ScreenScraper screens as PNGs, for review: `SLOT_SCREENS=dir`.
    #[test]fn draw_the_cart_art_screens(){
        let Some(dir)=std::env::var_os("SLOT_SCREENS")else{return};let dir=PathBuf::from(dir);fs::create_dir_all(&dir).unwrap();
        let save=|name:&str,f:UndoFace|{let img=slot_cartart::Image::from_rgba(f.w,f.h,f.rgba).unwrap();fs::write(dir.join(name),slot_cartart::image::encode_png_rgba(&img)).unwrap();};
        let mut m=art_menu(None);m.status="Log in to ScreenScraper to fetch cart art: A on SCREENSCRAPER".into();save("cart-art-menu.png",m.render());
        m.row=1;m.status="Enter your ScreenScraper username and password".into();m.open_login(Some("Enter your ScreenScraper username and password"));
        if let Some(l)=m.list.as_mut().and_then(|l|l.login.as_mut()){l.user="jeff".into();l.password="hunter22".into();}
        save("screenscraper-login.png",m.render());
        m.input(Action::GbaDown(Btn::Down));m.input(Action::GbaDown(Btn::A));m.input(Action::GbaDown(Btn::A));save("screenscraper-password-keyboard.png",m.render());
        let mut s=art_menu(Some("jeff"));s.open_login(None);s.row=1;s.status="Logged in as jeff, level 1, 1 thread. 12 of 20000 lookups used today".into();save("screenscraper-logged-in.png",s.render());
        let mut c=art_menu(Some("jeff"));c.row=1;c.status="2 games have a label. A on one twice deletes it".into();save("cart-art-menu-logged-in.png",c.render());
    }

    fn pick_menu(systems:Vec<(Platform,Vec<(&str,bool)>)>,selected:HashSet<crate::offline_ra::CacheKey>)->WifiMenu{
        let systems=systems.into_iter().map(|(platform,games)|crate::offline_ra::CacheSystem{
            platform,
            games:games.into_iter().map(|(stem,cached)|crate::offline_ra::CacheGame{
                key:crate::offline_ra::CacheKey{platform,stem:stem.into()},
                title:stem.into(),
                cached,
                size:if cached{Some(1200)}else{None},
            }).collect(),
        }).collect::<Vec<_>>();
        let avg=Some(1200);
        let account=ra::Account{user:"Benni".into(),token:"t".into()};
        let pick=CachePicker{systems,selected,system:0,avg};
        WifiMenu{worker:None,face:None,dirty:true,on:true,status:pick.status_line(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:Some(RaScreen{account:Some(account),user:String::new(),password:String::new(),job:None,test:None,page:3,games:Vec::new(),focus:0,cache:None,pick:Some(pick)}),query:None,update:None}
    }
    #[test]fn cache_achievement_sets_opens_a_system_picker(){
        let account=ra::Account{user:"Benni".into(),token:"t".into()};
        let mut m=WifiMenu{worker:None,face:None,dirty:true,on:true,status:String::new(),networks:Vec::new(),row:0,keyboard:None,refreshed:Instant::now(),art:None,fetched:Vec::new(),list:None,ra:Some(RaScreen{account:Some(account),user:String::new(),password:String::new(),job:None,test:None,page:0,games:Vec::new(),focus:0,cache:None,pick:None}),query:None,update:None};
        m.row=2;
        m.input(Action::GbaDown(Btn::A));
        let ra=m.ra.as_ref().unwrap();
        assert_eq!(ra.page,3);
        assert!(ra.pick.is_some());
        assert!(m.status.contains("selected"),"{}",m.status);
        assert!(!m.input(Action::GbaDown(Btn::B)));
        assert_eq!(m.ra.as_ref().unwrap().page,0);
        assert!(m.ra.as_ref().unwrap().pick.is_none());
        assert_eq!(m.row,2);
    }
    #[test]fn the_cache_picker_toggles_systems_and_drills_into_games(){
        let mut selected=HashSet::new();
        selected.insert(crate::offline_ra::CacheKey{platform:Platform::Snes,stem:"Mario".into()});
        let mut m=pick_menu(vec![
            (Platform::Gb,vec![("Tetris",false),("Kirby",true)]),
            (Platform::Snes,vec![("Mario",true),("Yoshi",false)]),
        ],selected);
        let rows=WifiMenu::picker_rows(m.ra.as_ref().unwrap().pick.as_ref().unwrap(),3);
        assert_eq!(rows[0],"SELECT ALL");
        assert_eq!(rows[1],"SELECT NONE");
        assert!(rows[2].starts_with("CACHE SELECTED"),"{:?}",rows);
        assert!(rows[3].starts_with("[ ]  GB"),"{:?}",rows);
        assert!(rows[4].contains("SNES")&&rows[4].starts_with("[+]"),"{:?}",rows);
        m.row=3;m.input(Action::GbaDown(Btn::A));
        let p=m.ra.as_ref().unwrap().pick.as_ref().unwrap();
        assert_eq!(p.system_selected(0),2);
        m.row=4;m.input(Action::GbaDown(Btn::Right));
        assert_eq!(m.ra.as_ref().unwrap().page,4);
        let rows=WifiMenu::picker_rows(m.ra.as_ref().unwrap().pick.as_ref().unwrap(),4);
        assert!(rows[2].contains("[X]  Mario")&&rows[2].contains("CACHED"),"{:?}",rows);
        assert!(rows[3].contains("[ ]  Yoshi"),"{:?}",rows);
        m.row=3;m.input(Action::GbaDown(Btn::A));
        let p=m.ra.as_ref().unwrap().pick.as_ref().unwrap();
        assert!(p.selected.contains(&crate::offline_ra::CacheKey{platform:Platform::Snes,stem:"Yoshi".into()}));
        assert!(!m.input(Action::GbaDown(Btn::B)));
        assert_eq!(m.ra.as_ref().unwrap().page,3);
        assert_eq!(m.row,4);
    }
    #[test]fn cache_selected_without_a_choice_asks_first(){
        let mut m=pick_menu(vec![(Platform::Gb,vec![("Tetris",false)])],HashSet::new());
        m.row=2;m.input(Action::GbaDown(Btn::A));
        assert_eq!(m.status,"Choose at least one game");
        assert!(m.ra.as_ref().unwrap().cache.is_none());
    }
    /// The Cache sets screens as PNGs, for review: `SLOT_SCREENS=dir`.
    #[test]fn draw_the_cache_picker_screens(){
        let Some(dir)=std::env::var_os("SLOT_SCREENS")else{return};let dir=PathBuf::from(dir);fs::create_dir_all(&dir).unwrap();
        let save=|name:&str,f:UndoFace|{let img=slot_cartart::Image::from_rgba(f.w,f.h,f.rgba).unwrap();fs::write(dir.join(name),slot_cartart::image::encode_png_rgba(&img)).unwrap();};
        let mut selected=HashSet::new();
        for (p,s) in [(Platform::Snes,"Mario"),(Platform::Snes,"Yoshi"),(Platform::Gb,"Tetris"),(Platform::Nes,"Zelda"),(Platform::Md,"Sonic"),(Platform::Gba,"Emerald")]{
            selected.insert(crate::offline_ra::CacheKey{platform:p,stem:s.into()});
        }
        let mut m=pick_menu(vec![
            (Platform::Gba,vec![("Emerald",true),("Metroid Fusion",false),("Wario Land 4",false)]),
            (Platform::Gb,vec![("Tetris",true),("Kirby",false),("Zelda DX",false)]),
            (Platform::Md,vec![("Sonic",true),("Streets of Rage 2",false)]),
            (Platform::Nes,vec![("Zelda",false),("Metroid",false),("Punch-Out!!",true)]),
            (Platform::Snes,vec![("Mario",true),("Yoshi",false),("Star Fox",false),("Chrono Trigger",false)]),
        ],selected);
        m.status=m.ra.as_ref().unwrap().pick.as_ref().unwrap().status_line();
        m.row=7;
        save("cache-sets-systems.png",m.render());
        m.row=2;
        save("cache-sets-cache-selected.png",m.render());
        m.row=7;m.input(Action::GbaDown(Btn::Right));
        save("cache-sets-snes-games.png",m.render());
        let empty=pick_menu(vec![
            (Platform::Gb,vec![("Tetris",false)]),
            (Platform::Snes,vec![("Mario",false)]),
        ],HashSet::new());
        save("cache-sets-none-selected.png",empty.render());
    }


    #[test]fn a_running_cache_keeps_going_after_leaving_the_screen_and_shows_live_progress_on_return(){
        crate::offline_ra::test_set_cache_state(Some("Caching 3/10: Mario"),true,None);
        let mut m=pick_menu(vec![(Platform::Gb,vec![("Tetris",false)])],HashSet::new());
        m.poll();
        assert_eq!(m.status,"Caching 3/10: Mario");
        assert!(!m.busy(),"a background cache must not trap B as Cancel");
        // B leaves the picker for the RetroAchievements menu; the pass stays running.
        assert!(!m.input(Action::GbaDown(Btn::B)));
        assert_eq!(m.ra.as_ref().unwrap().page,0);
        assert!(crate::offline_ra::cache_busy());
        assert_eq!(crate::offline_ra::cache_status().as_deref(),Some("Caching 3/10: Mario"));
        // Coming back (or staying) still follows the live line, not a frozen copy.
        crate::offline_ra::test_set_cache_state(Some("Caching 7/10: Yoshi"),true,None);
        m.poll();
        assert_eq!(m.status,"Caching 7/10: Yoshi");
        // When it finishes, the stale "Caching N" line is gone and the counts are shown.
        crate::offline_ra::test_set_cache_state(Some("Caching 7/10: Yoshi"),false,Some((6,1,0)));
        assert!(crate::offline_ra::cache_status().is_none());
        m.poll();
        assert_eq!(m.status,"Cached 6 sets, 1 had none, 0 failed");
        crate::offline_ra::test_set_cache_state(None,false,None);
    }

}
