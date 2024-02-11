use anyhow::Result;
use crossbeam_channel::{Receiver, Sender};
use nih_plug::debug::*;
use nih_plug::prelude::*;
use nih_plug_vizia::ViziaState;
use parking_lot::RwLock;
use rosc::{OscMessage, OscPacket, OscType};
use std::net::UdpSocket;
use std::sync::atomic::AtomicBool;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;

mod editor;
mod subviews;

pub struct OsClaPhoton {
    params: Arc<OsClaPhotonParams>,
    osc_thread: Option<JoinHandle<()>>,
    sender: Arc<Sender<OscChannelMessageType>>,
    receiver: Option<Receiver<OscChannelMessageType>>,
    editor_state: Arc<ViziaState>,

    beamNr_dirty: Arc<AtomicBool>,
    rotSpeed_dirty: Arc<AtomicBool>,
    beamSz_dirty: Arc<AtomicBool>,
    zoom_dirty: Arc<AtomicBool>,
    zoomSp_dirty: Arc<AtomicBool>,
    offset_dirty: Arc<AtomicBool>,

    //ToDo:
    //tilts params
    //nested and array params info:
    // https://github.com/robbert-vdh/nih-plug/blob/master/plugins/examples/gain/src/lib.rs
    // and around line 270 here: https://github.com/robbert-vdh/nih-plug/blob/master/src/params.rs
    //control params

    //----Perform:
    //Beam Nr
    //Rot Speed
    //Beam Size
    //Zoom
    //Zoom Speed ?
    //Offset
    //----Tilts:
    //Tilt
    //Dimmer
    //R
    //G
    //B
    //=====46 params
}

impl Default for OsClaPhoton {
    fn default() -> Self {

        let beamNr_dirty = Arc::new(AtomicBool::new(false));
        let rotSpeed_dirty = Arc::new(AtomicBool::new(false));
        let beamSz_dirty = Arc::new(AtomicBool::new(false));
        let zoom_dirty = Arc::new(AtomicBool::new(false));
        let zoomSp_dirty = Arc::new(AtomicBool::new(false));
        let offset_dirty = Arc::new(AtomicBool::new(false));

        let channel = OscChannel::default();
        Self {
            params: Arc::new(OsClaPhotonParams::new(
                beamNr_dirty.clone(),
                rotSpeed_dirty.clone(),
                beamSz_dirty.clone(),
                zoom_dirty.clone(),
                zoomSp_dirty.clone(),
                offset_dirty.clone(),
            )),
            osc_thread: None,
            sender: Arc::new(channel.sender),
            receiver: Some(channel.receiver),
            editor_state: editor::default_state(),
            beamNr_dirty,
            rotSpeed_dirty,
            beamSz_dirty,
            zoom_dirty,
            zoomSp_dirty,
            offset_dirty,
        }
    }
}

impl Drop for OsClaPhoton {
    fn drop(&mut self) {
        self.kill_background_thread();
    }
}

struct OscChannel {
    sender: Sender<OscChannelMessageType>,
    receiver: Receiver<OscChannelMessageType>,
}

impl Default for OscChannel {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::bounded(65_536);
        Self { sender, receiver }
    }
}

struct OscParamType {
    name: String,
    value: f32,
}

struct OscConnectionType {
    ip: String,
    port: u16,
}

struct OscAddressBaseType {
    address: String,
}

enum OscChannelMessageType {
    Exit,
    ConnectionChange(OscConnectionType),
    AddressBaseChange(OscAddressBaseType),
    Param(OscParamType),
}

#[derive(Params)]
pub struct OsClaPhotonParams {
    //Persisted Settings
    #[persist = "osc_server_address"]
    osc_server_address: RwLock<String>,
    #[persist = "osc_server_port"]
    osc_server_port: RwLock<u16>,
    #[persist = "osc_address_base"]
    osc_address_base: RwLock<String>,

    //Setting Flags
    // #[id = "flag_send_midi"]
    // flag_send_midi: BoolParam,
    // #[id = "flag_send_audio"]
    // flag_send_audio: BoolParam,
    // #[id = "osc_sample_rate"]
    // osc_sample_rate: IntParam,

    //Exposed Params
    #[id = "beamNr"]
    beamNr: FloatParam,
    #[id = "rotSpeed"]
    rotSpeed: FloatParam,
    #[id = "beamSz"]
    beamSz: FloatParam,
    #[id = "zoom"]
    zoom: FloatParam,
    #[id = "zoomSp"]
    zoomSp: FloatParam,
    #[id = "offset"]
    offset: FloatParam,
}

impl OsClaPhotonParams {
    #[allow(clippy::derivable_impls)]
    fn new(
        beamNr_dirty: Arc<AtomicBool>,
        rotSpeed_dirty: Arc<AtomicBool>,
        beamSz_dirty: Arc<AtomicBool>,
        zoom_dirty: Arc<AtomicBool>,
        zoomSp_dirty: Arc<AtomicBool>,
        offset_dirty: Arc<AtomicBool>,

    ) -> Self {
        Self {
            osc_server_address: RwLock::new("255.255.255.255".to_string()),
            osc_server_port: RwLock::new(12345),
            osc_address_base: RwLock::new("osclaphoton".to_string()),
            // flag_send_midi: BoolParam::new("flag_send_midi", true)
            //     .hide()
            //     .non_automatable(),
            // flag_send_audio: BoolParam::new("flag_send_audio", false)
            //     .hide()
            //     .non_automatable(),
            // //TODO: handle value change updating resampler ratio
            // osc_sample_rate: IntParam::new(
            //     "osc_sample_rate",
            //     100,
            //     IntRange::Linear { min: 0, max: 1000 },
            // )
            // .hide()
            // .non_automatable(),

            beamNr: FloatParam::new("Beams Number", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.01)
                .with_callback(Arc::new(move |_x| beamNr_dirty.store(true, Ordering::Release))),
            rotSpeed: FloatParam::new("Rotation Speed", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.0001)
                .with_callback(Arc::new(move |_x| rotSpeed_dirty.store(true, Ordering::Release))),
            beamSz: FloatParam::new("Beams Size", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.0001)
                .with_callback(Arc::new(move |_x| beamSz_dirty.store(true, Ordering::Release))),
            zoom: FloatParam::new("Zoom", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.0001)
                .with_callback(Arc::new(move |_x| zoom_dirty.store(true, Ordering::Release))),
            zoomSp: FloatParam::new("Zoom Speed", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.0001)
                .with_callback(Arc::new(move |_x| zoomSp_dirty.store(true, Ordering::Release))),
            offset: FloatParam::new("Offset", 0.0, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_step_size(0.0001)
                .with_callback(Arc::new(move |_x| offset_dirty.store(true, Ordering::Release))),
            
        }
    }
}

impl Plugin for OsClaPhoton {
    const NAME: &'static str = "OSCLAPHOTON";
    const VENDOR: &'static str = "VanTa @Hawtin Labs";
    const URL: &'static str = "https://github.com/Hawtin-Labs/OSCLAPHOTON";
    const EMAIL: &'static str = "";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;
    const HARD_REALTIME_ONLY: bool = true;

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),

        aux_input_ports: &[],
        aux_output_ports: &[],
        names: PortNames::const_default(),
    }];

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        nih_trace!("Params Called");
        self.params.clone() as Arc<dyn Params>
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        nih_trace!("Editor Called");
        editor::create(
            self.params.clone(),
            self.sender.clone(),
            self.editor_state.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        nih_trace!("Initialize Called");

        if buffer_config.process_mode != ProcessMode::Realtime {
            nih_log!("Plugin is not in realtime mode, bailing!");
            return false;
        }

        //Setup OSC background thread
        //Dont remake the background thread if its already running
        if self.osc_thread.is_none() {
            let socket = match UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => socket,
                Err(e) => {
                    nih_error!("Failed to bind socket {:?}", e);
                    return false;
                }
            };
            let ip_port = format!(
                "{}:{}",
                *self.params.osc_server_address.read(),
                *self.params.osc_server_port.read()
            );
            nih_trace!("Connecting: {}", ip_port);
            
            socket.set_broadcast(true);

            let connect_result = socket.connect(&ip_port);
            if connect_result.is_err() {
                nih_error!(
                    "Failed to connect socket to {} {:?}",
                    ip_port,
                    connect_result.unwrap_err()
                );
                return false;
            }

            nih_trace!("Connected!");
            nih_trace!("Connected to: {}", ip_port);

            let address_base = self.params.osc_address_base.read().to_string();
            nih_trace!("OSC Address Base: {}", address_base);

            if let Some(receiver) = std::mem::replace(&mut self.receiver, None) {
                let client_thread =
                    thread::spawn(move || osc_client_worker(socket, address_base, receiver));

                self.osc_thread = Some(client_thread);
            } else {
                nih_error!("Failed get thread channel receiver");
                return false;
            }
        } else {
            //Threads already alive just update params
            let connection_send_result =
                self.sender
                    .send(OscChannelMessageType::ConnectionChange(OscConnectionType {
                        ip: self.params.osc_server_address.read().to_string(),
                        port: *self.params.osc_server_port.read(),
                    }));
            if connection_send_result.is_err() {
                nih_error!(
                    "Failed to send ConnectionChange update {:?}",
                    connection_send_result.unwrap_err()
                );
            }
            let address_base = self.params.osc_address_base.read().to_string();
            nih_trace!("OSC Address Base: {}", address_base);
            let address_send_result = self.sender.send(OscChannelMessageType::AddressBaseChange(
                OscAddressBaseType {
                    address: address_base,
                },
            ));
            if address_send_result.is_err() {
                nih_error!(
                    "Failed to send AddressBaseChange update {:?}",
                    address_send_result.unwrap_err()
                );
            }
        }
        true
    }

    fn deactivate(&mut self) {
        nih_trace!("Deactivate Called");
        self.kill_background_thread();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        //Process Dirty Params
        let param_result = self.process_params();
        if param_result.is_err() {
            nih_error!("Failed to send params {:?}", param_result.unwrap_err());
        }
        ProcessStatus::Normal
    }
}

impl OsClaPhoton {
    fn process_params(&self) -> Result<()> {

        self.send_dirty_param(&self.beamNr_dirty, &self.params.beamNr)?;
        self.send_dirty_param(&self.rotSpeed_dirty, &self.params.rotSpeed)?;
        self.send_dirty_param(&self.beamSz_dirty, &self.params.beamSz)?;
        self.send_dirty_param(&self.zoom_dirty, &self.params.zoom)?;
        self.send_dirty_param(&self.zoomSp_dirty, &self.params.zoomSp)?;
        self.send_dirty_param(&self.offset_dirty, &self.params.offset)?;


        Ok(())
    }

    fn send_dirty_param(&self, param_dirty: &Arc<AtomicBool>, param: &FloatParam) -> Result<()> {
        if param_dirty
            .compare_exchange(true, false, Ordering::Acquire, Ordering::Relaxed)
            .is_ok()
        {
            nih_trace!("Param Dirty: {} {}", param.name(), param.value());
            self.sender
                .send(OscChannelMessageType::Param(OscParamType {
                    name: param.name().to_string(), //TODO: allocation
                    value: param.value(),
                }))?;
        }
        Ok(())
    }

    fn kill_background_thread(&mut self) {
        let exit_result = self.sender.send(OscChannelMessageType::Exit);
        if exit_result.is_err() {
            nih_error!(
                "Failed to send shutdown to background thread {:?}",
                exit_result.unwrap_err()
            );
        }
        self.osc_thread = None;
    }
}

// /<osc_address_base>/param/<param_name>

fn osc_client_worker(
    socket: UdpSocket,
    param_address_base: String,
    recv: Receiver<OscChannelMessageType>,
) -> () {
    nih_trace!("Background thread spawned!");
    nih_trace!("Background thread OSC Address Base: {}", param_address_base);
    let mut address_base = format_osc_address_base(&param_address_base);
    let mut connected = true; //We assume the socket we get is good
    while let Some(channel_message) = recv.recv().ok() {
        let osc_message = match channel_message {
            OscChannelMessageType::Exit => break,
            OscChannelMessageType::ConnectionChange(message) => {
                let ip_port = format!("{}:{}", message.ip, message.port);
                nih_trace!("Connection Change: {}", ip_port);
                let socket_result = socket.connect(&ip_port);
                match socket_result {
                    Ok(_) => connected = true,
                    Err(e) => {
                        connected = false;
                        nih_error!("Failed to connect to {} {:?}", ip_port, e);
                    }
                }
                continue;
            }
            OscChannelMessageType::AddressBaseChange(message) => {
                address_base = format_osc_address_base(&message.address);
                nih_trace!("AddressBase Change: {}", address_base);
                continue;
            }
            OscChannelMessageType::Param(message) => OscMessage {
                addr: format!("{}/param/{}", address_base, message.name),
                args: vec![OscType::Float(message.value)],
            },
        };
        if connected {
            let packet = OscPacket::Message(osc_message);
            let buf = match rosc::encoder::encode(&packet) {
                Ok(buf) => buf,
                Err(e) => {
                    nih_error!("Failed to encode osc message {:?}", e);
                    continue;
                }
            };
            let len = match socket.send(&buf[..]) {
                Ok(buf) => buf,
                Err(e) => {
                    nih_error!("Failed to send osc message {:?}", e);
                    continue;
                }
            };
            if len != buf.len() {
                nih_trace!("UDP packet not fully sent");
            }
            nih_trace!("Sent {:?} packet", packet);
        }
    }
}

fn format_osc_address_base(raw_base: &str) -> String {
    if raw_base.is_empty() {
        return "".to_string();
    } else {
        return format!("/{}", raw_base); //Prefix with slash
    }
}

impl ClapPlugin for OsClaPhoton {
    const CLAP_ID: &'static str = "xyz.vanta.osclaphoton";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Outputs OSC Photon control from the DAW");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::NoteEffect,
        ClapFeature::Utility,
        ClapFeature::Analyzer,
    ];

    const CLAP_MANUAL_URL: Option<&'static str> = None;

    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_POLY_MODULATION_CONFIG: Option<PolyModulationConfig> = None;
}

// impl Vst3Plugin for OsClap {
//     const VST3_CLASS_ID: [u8; 16] = *b"grbt-daw-outputs";
//     const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Instrument, Vst3SubCategory::Tools];
// }

nih_export_clap!(OsClaPhoton);
//nih_export_vst3!(OsClap);
