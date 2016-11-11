use vr_traits::{WebVRMsg, WebVRResult};
use vr_traits::webvr::*;
use ipc_channel::ipc;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use util::thread::spawn_named;
use std::collections::{HashMap, HashSet};
use msg::constellation_msg::PipelineId;
use script_traits::{ConstellationMsg, WebVREventMsg};
use std::ops::DerefMut;
use std::sync::mpsc::Sender;
use std::{thread, time};

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

    fn start(&mut self) {

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
            displays.push(device.borrow().get_display_data());
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
                sender.send(Ok(device.borrow().get_frame_data(near, far))).unwrap()
            },
            Err(msg) => sender.send(Err(msg.into())).unwrap()
        }
    }

    fn handle_reset_pose(&mut self,
                         pipeline: PipelineId,
                         device_id: u64,
                         sender: Option<IpcSender<WebVRResult<()>>>) {
        match self.access_check(pipeline, device_id) {
            Ok(device) => {
                device.borrow_mut().reset_pose();
                if let Some(sender) = sender {
                    sender.send(Ok(())).unwrap()
                }
            },
            Err(msg) => {
                if let Some(sender) = sender {
                    sender.send(Err(msg.into())).unwrap()
                }
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
                         sender: IpcSender<WebVRResult<VRDeviceCompositor>>) {
        match self.access_check(pipeline, device_id).map(|d| d.clone()) {
            Ok(device) => {
                self.presenting.insert(device_id, pipeline);
                let data = device.borrow().get_display_data();
                let compositor = VRDeviceCompositor::new(device.borrow_mut().deref_mut());
                sender.send(Ok(compositor)).unwrap();
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
                         sender: IpcSender<WebVRResult<()>>) {
        match self.access_check(pipeline, device_id).map(|d| d.clone()) {
            Ok(device) => {
                self.presenting.remove(&device_id);
                sender.send(Ok(())).unwrap();
                let data = device.borrow().get_display_data();
                self.notify_event(VRDisplayEvent::PresentChange(data, false));
            },
            Err(msg) => {
                sender.send(Err(msg.into())).unwrap();
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
                        break;
                    }
                    if receiver.recv().unwrap() == false {
                        // WebVR Thread asked to unschedule this thread
                        break; 
                    }
                    thread::sleep(time::Duration::from_millis(500));
                }
            });
        }
    }

}

