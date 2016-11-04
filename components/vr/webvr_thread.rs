use vr_traits::{WebVRMsg, WebVRResult};
use vr_traits::webvr::*;

use ipc_channel::ipc;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use util::thread::spawn_named;
use std::collections::HashSet;
use msg::constellation_msg::PipelineId;
use script_traits::{ConstellationMsg, WebVREventMsg};
use std::sync::mpsc::Sender;

pub struct WebVRThread {
    receiver: IpcReceiver<WebVRMsg>,
    service: VRServiceManager,
    contexts: HashSet<PipelineId>,
    constellation_chan: Sender<ConstellationMsg>
}

impl WebVRThread {
    fn new (receiver: IpcReceiver<WebVRMsg>, constellation_chan: Sender<ConstellationMsg>) -> WebVRThread {
        let mut service = VRServiceManager::new();
        service.register_defaults();
        WebVRThread {
            receiver: receiver,
            service: service,
            contexts: HashSet::new(),
            constellation_chan: constellation_chan
        }
    }

    pub fn spawn(constellation_chan: Sender<ConstellationMsg>) -> IpcSender<WebVRMsg> {
        let (sender, receiver) = ipc::channel().unwrap();
        spawn_named("WebVRThread".into(), move || {
            WebVRThread::new(receiver, constellation_chan).start();
        });
        sender
    }

    fn start(&mut self) {

        while let Ok(msg) = self.receiver.recv() {
            match msg {
                WebVRMsg::RegisterContext(context) => {
                    self.handle_register_context(context);
                },
                WebVRMsg::UnregisterContext(context) => {
                    self.handle_unregister_context(context);
                },
                WebVRMsg::GetVRDisplays(sender) => {
                    self.handle_get_displays(sender)
                },
                WebVRMsg::GetFrameData(device_id, near, far, sender) => {
                    // TODO: create a timer to poll events
                    //self.poll_events();

                    self.handle_framedata(device_id, near, far, sender)
                },
                WebVRMsg::ResetPose(device_id, sender) => {
                    self.handle_reset_pose(device_id, sender)
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

    fn handle_framedata(&mut self, device_id: u64, near: f64, far: f64, sender: IpcSender<WebVRResult<VRFrameData>>) {
        let device = self.service.get_device(device_id);
        match device {
            Some(device) => {
                sender.send(Ok(device.borrow().get_frame_data(near, far))).unwrap()
            },
            None => sender.send(Err("Device not found".into())).unwrap()
        }
    }

    fn handle_reset_pose(&mut self, device_id: u64, sender: Option<IpcSender<WebVRResult<()>>>) {
        let device = self.service.get_device(device_id);
        match device {
            Some(device) => {
                device.borrow_mut().reset_pose();
                if let Some(sender) = sender {
                    sender.send(Ok(())).unwrap()
                }
            },
            None => {
                if let Some(sender) = sender {
                    sender.send(Err("Device not found".into())).unwrap()
                }
            }
        }
    }

    fn poll_events(&mut self) {
        let events = self.service.poll_events();
        if events.len() > 0 {
            let pipeline_ids: Vec<PipelineId> = self.contexts.iter().map(|c| *c).collect();
            for event in events {
                let event = WebVREventMsg::DisplayEvent(event);
                self.constellation_chan.send(ConstellationMsg::WebVREvent(pipeline_ids.clone(), event)).unwrap();
            }
        }
    }

}

