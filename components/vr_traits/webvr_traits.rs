use webvr::*;

use ipc_channel::ipc::IpcSender;
use msg::constellation_msg::PipelineId;

pub type WebVRResult<T> = Result<T, String>;

#[derive(Deserialize, Serialize)]
pub enum WebVRMsg {
    RegisterContext(PipelineId),
    UnregisterContext(PipelineId),
    GetVRDisplays(IpcSender<WebVRResult<Vec<VRDisplayData>>>),
    GetFrameData(u64, f64, f64, IpcSender<WebVRResult<VRFrameData>>),
    ResetPose(u64, Option<IpcSender<WebVRResult<()>>>),
    Exit,
}
