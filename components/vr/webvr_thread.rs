use webvr::*;
use ipc_channel::ipc;
use ipc_channel::ipc::{IpcReceiver, IpcSender};
use util::thread::spawn_named;

pub type WebVRResult<T> = Result<T, String>;

#[derive(Deserialize, Serialize)]
pub enum WebVRMsg {
    GetVRDisplays(IpcSender<WebVRResult<Vec<VRDisplayData>>>),
    GetFrameData(u64, f64, f64, IpcSender<WebVRResult<VRFrameData>>),
    ResetPose(u64, Option<IpcSender<WebVRResult<()>>>),
    Exit,
}

pub struct WebVRThread {
    receiver: IpcReceiver<WebVRMsg>,
    service: VRServiceManager
}

impl WebVRThread {
    fn new (receiver: IpcReceiver<WebVRMsg>) -> WebVRThread {
        let mut service = VRServiceManager::new();
        service.register_mock();
        WebVRThread {
            receiver: receiver,
            service: service
        }
    }

    pub fn spawn() -> IpcSender<WebVRMsg> {
        let (sender, receiver) = ipc::channel().unwrap();
        spawn_named("WebVRThread".into(), move || {
            WebVRThread::new(receiver).start();
        });
        sender
    }

    fn start(&mut self) {
        while let Ok(msg) = self.receiver.recv() {
            match msg {
                WebVRMsg::GetVRDisplays(sender) => {
                    self.handle_get_displays(sender)
                },
                WebVRMsg::GetFrameData(device_id, near, far, sender) => {
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
}

