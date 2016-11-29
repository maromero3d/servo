use vr_traits::{WebVRMsg, WebVRResult};
use vr_traits::webvr::*;
use ipc_channel::ipc;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use util::thread::spawn_named;
use std::collections::{HashMap, HashSet};
use msg::constellation_msg::PipelineId;
use script_traits::{ConstellationMsg, WebVREventMsg};
use std::ptr;
use std::sync::mpsc::Sender;
use std::{thread, time};
use webrender_traits;

pub struct WebVRThread {
    receiver: IpcReceiver<WebVRMsg>,
    sender: IpcSender<WebVRMsg>,
    service: VRServiceManager,
    contexts: HashSet<PipelineId>,
    constellation_chan: Sender<ConstellationMsg>,
    polling_events: bool,
    presenting: HashMap<u64, PipelineId>
}

impl WebVRThread {
    fn new (receiver: IpcReceiver<WebVRMsg>,
            sender: IpcSender<WebVRMsg>,
            constellation_chan: Sender<ConstellationMsg>)
            -> WebVRThread {
        let mut service = VRServiceManager::new();
        service.register_defaults();
        WebVRThread {
            receiver: receiver,
            sender: sender,
            service: service,
            contexts: HashSet::new(),
            constellation_chan: constellation_chan,
            polling_events: false,
            presenting: HashMap::new()
        }
    }

    pub fn spawn(constellation_chan: Sender<ConstellationMsg>) -> IpcSender<WebVRMsg> {
        let (sender, receiver) = ipc::channel().unwrap();
        let sender_clone = sender.clone();
        spawn_named("WebVRThread".into(), move || {
            WebVRThread::new(receiver, sender_clone, constellation_chan).start();
        });
        sender
    }

    #[allow(unsafe_code)]
    fn start(&mut self) {
        unsafe {
            // Set the shared raw pointer for the VRCompositors.
            // See WebVRCompositorCreator for more details.
            VR_MANAGER = &mut self.service;
        }
        while let Ok(msg) = self.receiver.recv() {
            match msg {
                WebVRMsg::RegisterContext(context) => {
                    self.handle_register_context(context);
                    self.schedule_poll_events();
                },
                WebVRMsg::UnregisterContext(context) => {
                    self.handle_unregister_context(context);
                },
                WebVRMsg::PollEvents(sender) => {
                    self.poll_events(sender);
                },
                WebVRMsg::GetVRDisplays(sender) => {
                    self.handle_get_displays(sender);
                    self.schedule_poll_events();
                },
                WebVRMsg::GetFrameData(pipeline_id, device_id, near, far, sender) => {
                    self.handle_framedata(pipeline_id, device_id, near, far, sender);
                },
                WebVRMsg::ResetPose(pipeline_id, device_id, sender) => {
                    self.handle_reset_pose(pipeline_id, device_id, sender);
                },
                WebVRMsg::RequestPresent(pipeline_id, device_id, sender) => {
                    self.handle_request_present(pipeline_id, device_id, sender);
                },
                WebVRMsg::ExitPresent(pipeline_id, device_id, sender) => {
                    self.handle_exit_present(pipeline_id, device_id, sender);
                },
                WebVRMsg::Exit => {
                    break
                },
            }
        }
        unsafe {
            // VRServiceManager instance will Drop, reset the shared pointer
            VR_MANAGER = ptr::null_mut();
        }
    }

    fn handle_register_context(&mut self, ctx: PipelineId) {
        self.contexts.insert(ctx);
    }

    fn handle_unregister_context(&mut self, ctx: PipelineId) {
        self.contexts.remove(&ctx);
    }

    fn handle_get_displays(&mut self, sender: IpcSender<WebVRResult<Vec<VRDisplayData>>>) {
        let devices = self.service.get_devices();
        let mut displays = Vec::new();
        for device in devices {
            displays.push(device.borrow().display_data());
        }
        sender.send(Ok(displays)).unwrap();
    }

    fn handle_framedata(&mut self, 
                        pipeline: PipelineId,
                        device_id: u64,
                        near: f64,
                        far: f64,
                        sender: IpcSender<WebVRResult<VRFrameData>>) {
      match self.access_check(pipeline, device_id) {
            Ok(device) => {
                sender.send(Ok(device.borrow().inmediate_frame_data(near, far))).unwrap()
            },
            Err(msg) => sender.send(Err(msg.into())).unwrap()
        }
    }

    fn handle_reset_pose(&mut self,
                         pipeline: PipelineId,
                         device_id: u64,
                         sender: IpcSender<WebVRResult<VRDisplayData>>) {
        match self.access_check(pipeline, device_id) {
            Ok(device) => {
                device.borrow_mut().reset_pose();
                sender.send(Ok(device.borrow().display_data())).unwrap();
            },
            Err(msg) => {
                sender.send(Err(msg.into())).unwrap()
            }
        }
    }

    fn access_check(&self, pipeline: PipelineId, device_id: u64) -> Result<&VRDevicePtr, &'static str> {
        if *self.presenting.get(&device_id).unwrap_or(&pipeline) != pipeline {
            return Err("Device owned by another context");
        }
        self.service.get_device(device_id).ok_or("Device not found")
    }

    fn handle_request_present(&mut self,
                         pipeline: PipelineId,
                         device_id: u64,
                         sender: IpcSender<WebVRResult<()>>) {
        match self.access_check(pipeline, device_id).map(|d| d.clone()) {
            Ok(device) => {
                self.presenting.insert(device_id, pipeline);
                let data = device.borrow().display_data();
                sender.send(Ok(())).unwrap();
                self.notify_event(VRDisplayEvent::PresentChange(data, true));
            },
            Err(msg) => {
                sender.send(Err(msg.into())).unwrap();
            }
        }
    }

    fn handle_exit_present(&mut self,
                         pipeline: PipelineId,
                         device_id: u64,
                         sender: Option<IpcSender<WebVRResult<()>>>) {
        match self.access_check(pipeline, device_id).map(|d| d.clone()) {
            Ok(device) => {
                self.presenting.remove(&device_id);
                if let Some(sender) = sender {
                    sender.send(Ok(())).unwrap();
                }
                let data = device.borrow().display_data();
                self.notify_event(VRDisplayEvent::PresentChange(data, false));
            },
            Err(msg) => {
                if let Some(sender) = sender {
                    sender.send(Err(msg.into())).unwrap();
                }
            }
        }
    }

    fn poll_events(&mut self, sender: IpcSender<bool>) {
        let events = self.service.poll_events();
        if events.len() > 0 {
            let pipeline_ids: Vec<PipelineId> = self.contexts.iter().map(|c| *c).collect();
            for event in events {
                let event = WebVREventMsg::DisplayEvent(event);
                self.constellation_chan.send(ConstellationMsg::WebVREvent(pipeline_ids.clone(), event)).unwrap();
            }
        }

        // Stop polling events if the callers are not using VR
        self.polling_events = self.contexts.len() > 0;
        sender.send(self.polling_events).unwrap();
    }

    fn notify_event(&self, event: VRDisplayEvent) {
        let pipeline_ids: Vec<PipelineId> = self.contexts.iter().map(|c| *c).collect();
        let event = WebVREventMsg::DisplayEvent(event);
        self.constellation_chan.send(ConstellationMsg::WebVREvent(pipeline_ids.clone(), event)).unwrap();
    }

    fn schedule_poll_events(&mut self) {
        if self.service.is_initialized() && !self.polling_events {
            self.polling_events = true;
            let webvr_thread = self.sender.clone();
            let (sender, receiver) = ipc::channel().unwrap();
            spawn_named("WebVRPollEvents".into(), move || {
                loop {
                    if webvr_thread.send(WebVRMsg::PollEvents(sender.clone())).is_err() {
                        // WebVR Thread closed
                        break;
                    }
                    if !receiver.recv().unwrap_or(false) {
                        // WebVR Thread asked to unschedule this thread
                        break;
                    }
                    thread::sleep(time::Duration::from_millis(500));
                }
            });
        }
    }
}

// In the compositor we use shared pointers instead of Arc<Mutex> for latency reasons.
// This also avoids "JS DDoS" attacks: A Second JSContext doing a lot of calls
// while the main one is presenting and demands both high framerate and low latency.
static mut VR_MANAGER: *mut VRServiceManager = 0 as *mut _;

pub struct WebVRCompositorHandler {
    compositors: HashMap<webrender_traits::VRCompositorId, *mut VRDevice>
}

#[allow(unsafe_code)]
unsafe impl Send for WebVRCompositorHandler {}

impl WebVRCompositorHandler {
    pub fn new() -> Box<WebVRCompositorHandler> {
        Box::new(WebVRCompositorHandler{
            compositors: HashMap::new()
        })
    }
}

impl webrender_traits::VRCompositorHandler for WebVRCompositorHandler {

    #[allow(unsafe_code)]
    fn handle(&mut self, cmd: webrender_traits::VRCompositorCommand, texture_id: Option<u32>) {
        match cmd {
            webrender_traits::VRCompositorCommand::Create(compositor_id) => {
                self.create_compositor(compositor_id);
            }
            webrender_traits::VRCompositorCommand::SyncPoses(compositor_id, near, far, sender) => {
                if let Some(compositor) = self.compositors.get(&compositor_id) {
                    let pose = unsafe {
                        (**compositor).sync_poses();
                        (**compositor).synced_frame_data(near, far).to_bytes()
                    };
                    let _result = sender.send(Ok(pose));
                } else {
                    let _result = sender.send(Err(()));
                }
            }
            webrender_traits::VRCompositorCommand::SubmitFrame(compositor_id, left_bounds, right_bounds) => {
                if let Some(compositor) = self.compositors.get(&compositor_id) {
                    if let Some(texture_id) = texture_id {
                        let layer = VRLayer {
                            texture_id: texture_id,
                            left_bounds: left_bounds,
                            right_bounds: right_bounds
                        };
                        unsafe {
                            (**compositor).submit_frame(&layer);
                        }
                    }
                }
            }
            webrender_traits::VRCompositorCommand::Release(compositor_id) => {
                self.compositors.remove(&compositor_id);
            }
        }
    }
}

impl WebVRCompositorHandler {
    #[allow(unsafe_code)]
    fn create_compositor(&mut self, device_id: webrender_traits::VRCompositorId) {

        if unsafe { VR_MANAGER.is_null() } {
            error!("VRServiceManager not available when creating a new VRCompositor");
            return;
        }

        let device = unsafe {
            (*VR_MANAGER).get_device(device_id)
        };

        match device {
            Some(ref device) => {
                self.compositors.insert(device_id, device.as_ptr());
            },
            None => {
                error!("VRDevice not found when creating a new VRCompositor");
            }
        };
    }
}