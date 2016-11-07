/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use core::ops::Deref;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::VRDisplayBinding;
use dom::bindings::codegen::Bindings::VRDisplayBinding::VRDisplayMethods;
use dom::bindings::codegen::Bindings::VRDisplayBinding::VREye;
use dom::bindings::codegen::Bindings::VRLayerBinding::VRLayer;
use dom::bindings::codegen::Bindings::WindowBinding::FrameRequestCallback;
use dom::bindings::inheritance::Castable;
use dom::bindings::js::{JS, MutNullableHeap, MutHeap, Root};
use dom::bindings::num::Finite;
use dom::bindings::reflector::{Reflectable, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::event::Event;
use dom::eventtarget::EventTarget;
use dom::globalscope::GlobalScope;
use dom::promise::Promise;
use dom::vrdisplaycapabilities::VRDisplayCapabilities;
use dom::vrdisplayevent::VRDisplayEvent;
use dom::vrstageparameters::VRStageParameters;
use dom::vreyeparameters::VREyeParameters;
use dom::vrframedata::VRFrameData;
use dom::vrpose::VRPose;
use ipc_channel::ipc;
use ipc_channel::ipc::IpcSender;
use std::cell::Cell;
use std::rc::Rc;
use vr_traits::webvr;
use vr_traits::WebVRMsg;

#[dom_struct]
pub struct VRDisplay {
    eventtarget: EventTarget,
    #[ignore_heap_size_of = "Defined in rust-webvr"]
    display: DOMRefCell<WebVRDisplayData>,
    depth_near: Cell<f64>,
    depth_far: Cell<f64>,
    presenting: Cell<bool>,
    left_eye_params: MutHeap<JS<VREyeParameters>>,
    right_eye_params: MutHeap<JS<VREyeParameters>>,
    capabilities: MutHeap<JS<VRDisplayCapabilities>>,
    stage_params: MutNullableHeap<JS<VRStageParameters>>,
    #[ignore_heap_size_of = "Defined in rust-webvr"]
    frame_data: DOMRefCell<WebVRFrameData> 
}

// Wrappers to include WebVR structs in a DOM struct
#[derive(Clone)]
pub struct WebVRDisplayData(webvr::VRDisplayData);
no_jsmanaged_fields!(WebVRDisplayData);

#[derive(Clone, Default)]
pub struct WebVRFrameData(webvr::VRFrameData);
no_jsmanaged_fields!(WebVRFrameData);

impl VRDisplay {

    fn new_inherited(global: &GlobalScope, display:&webvr::VRDisplayData) -> VRDisplay {

        let stage = match display.stage_parameters {
            Some(ref params) => Some(VRStageParameters::new(&params, &global)),
            None => None
        };

        VRDisplay {
            eventtarget: EventTarget::new_inherited(),
            display: DOMRefCell::new(WebVRDisplayData(display.clone())),
            depth_near: Cell::new(0.01),
            depth_far: Cell::new(10000.0),
            presenting: Cell::new(false),
            left_eye_params: MutHeap::new(&*VREyeParameters::new(&display.left_eye_parameters, &global)),
            right_eye_params: MutHeap::new(&*VREyeParameters::new(&display.right_eye_parameters, &global)),
            capabilities: MutHeap::new(&*VRDisplayCapabilities::new(&display.capabilities, &global)),
            stage_params: MutNullableHeap::new(stage.as_ref().map(|v| v.deref())),
            frame_data: DOMRefCell::new(Default::default())
        }
    }

    pub fn new(global: &GlobalScope, display:&webvr::VRDisplayData) -> Root<VRDisplay> {
        reflect_dom_object(box VRDisplay::new_inherited(&global, &display),
                           global,
                           VRDisplayBinding::Wrap)
    }
}

impl Drop for VRDisplay {
    fn drop(&mut self) {

    }
}

impl VRDisplayMethods for VRDisplay {

    fn IsConnected(&self) -> bool {
        self.display.borrow().0.connected
    }

    fn IsPresenting(&self) -> bool {
        self.presenting.get()
    }

    fn Capabilities(&self) -> Root<VRDisplayCapabilities> {
        Root::from_ref(&*self.capabilities.get())
    }

    fn GetStageParameters(&self) -> Option<Root<VRStageParameters>> {
        self.stage_params.get().map(|s| Root::from_ref(&*s))
    }

    fn GetEyeParameters(&self, eye: VREye) -> Root<VREyeParameters> {
        match eye {
            VREye::Left => Root::from_ref(&*self.left_eye_params.get()),
            VREye::Right => Root::from_ref(&*self.right_eye_params.get())
        }
    }

    fn DisplayId(&self) -> u32 {
        self.display.borrow().0.display_id as u32
    }

    fn DisplayName(&self) -> DOMString {
        DOMString::from(self.display.borrow().0.display_name.clone())
    }

    fn GetFrameData(&self, frameData: &VRFrameData) -> bool {
        //TODO: sync with compositor
        if let Some(wevbr_sender) = self.webvr_thread() {
            let (sender, receiver) = ipc::channel().unwrap();
            wevbr_sender.send(WebVRMsg::GetFrameData(self.global().pipeline_id(),
                                                     self.get_display_id(),
                                                     self.depth_near.get(),
                                                     self.depth_far.get(),
                                                     sender)).unwrap();
            match receiver.recv().unwrap() {
                Ok(data) => {
                    self.frame_data.borrow_mut().0 = data;
                },
                Err(e) => {
                    error!("WebVR::GetFrameData: {:?}", e);
                }
            }
        }

        frameData.update(&self.frame_data.borrow().0);
        true
    }

    fn GetPose(&self) -> Root<VRPose> {
        VRPose::new(&self.global(), &self.frame_data.borrow().0.pose)
    }

    fn ResetPose(&self) -> () {
        if let Some(wevbr_sender) = self.webvr_thread() {
            wevbr_sender.send(WebVRMsg::ResetPose(self.global().pipeline_id(),
                                                  self.get_display_id(), 
                                                  None)).unwrap();
        }
    }

    fn DepthNear(&self) -> Finite<f64> {
        Finite::wrap(self.depth_near.get())
    }

    fn SetDepthNear(&self, value: Finite<f64>) -> () {
        self.depth_near.set(*value.deref());
    }

    fn DepthFar(&self) -> Finite<f64> {
        Finite::wrap(self.depth_far.get())
    }

    fn SetDepthFar(&self, value: Finite<f64>) -> () {
        self.depth_far.set(*value.deref());
    }

    fn RequestAnimationFrame(&self, _callback: Rc<FrameRequestCallback>) -> i32 {
        unimplemented!()
    }

    fn CancelAnimationFrame(&self, _handle: i32) -> () {
        unimplemented!()
    }

    #[allow(unrooted_must_root)]
    fn RequestPresent(&self, _layers: Vec<VRLayer>) -> Rc<Promise> {
        unimplemented!()
    }

    #[allow(unrooted_must_root)]
    fn ExitPresent(&self) -> Rc<Promise> {
        unimplemented!()
    }

    fn SubmitFrame(&self) -> () {
        unimplemented!()
    }
}

impl VRDisplay {

    fn webvr_thread(&self) -> Option<IpcSender<WebVRMsg>> {
        self.global().as_window().webvr_thread()
    }

    pub fn get_display_id(&self) -> u64 {
        self.display.borrow().0.display_id
    }

    pub fn update_display(&self, display: &webvr::VRDisplayData) {
        self.display.borrow_mut().0 = display.clone()
    }

    pub fn handle_webvr_event(&self, event: &webvr::VRDisplayEvent) {
        match *event {
            webvr::VRDisplayEvent::Connect(ref display) => {
                self.update_display(&display);
            },
            webvr::VRDisplayEvent::Disconnect(_id) => {
                self.display.borrow_mut().0.connected = false;
            },
            webvr::VRDisplayEvent::Activate(ref display, _) |
            webvr::VRDisplayEvent::Deactivate(ref display, _) |
            webvr::VRDisplayEvent::Blur(ref display) |
            webvr::VRDisplayEvent::Focus(ref display) => {
                self.update_display(&display);
                self.notify_event(&event);
            },
            webvr::VRDisplayEvent::PresentChange(ref display, presenting) => {
                self.update_display(&display);
                self.presenting.set(presenting);
                self.notify_event(&event);
            },
            webvr::VRDisplayEvent::Change(ref display) => {
                // Change event doesn't exist in WebVR spec.
                // So we update diplsay data but don't notify to JS.
                self.update_display(&display);
            }
        };
    }

    fn notify_event(&self, event: &webvr::VRDisplayEvent) {
        let root = Root::from_ref(&*self);
        let event = VRDisplayEvent::new_from_webvr(&self.global(), &root, &event);
        event.upcast::<Event>().fire(self.upcast());
    }
}