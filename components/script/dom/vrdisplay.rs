/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use canvas_traits::{CanvasCommonMsg, CanvasMsg, byte_swap};
use core::nonzero::NonZero;
use core::ops::Deref;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::VRDisplayBinding;
use dom::bindings::codegen::Bindings::VRDisplayBinding::VRDisplayMethods;
use dom::bindings::codegen::Bindings::VRDisplayBinding::VREye;
use dom::bindings::codegen::Bindings::VRLayerBinding::VRLayer;
use dom::bindings::codegen::Bindings::WindowBinding::FrameRequestCallback;
use dom::bindings::codegen::UnionTypes::ImageDataOrHTMLImageElementOrHTMLCanvasElementOrHTMLVideoElement;
use dom::bindings::conversions::{ArrayBufferViewContents, ConversionResult, FromJSValConvertible, ToJSValConvertible};
use dom::bindings::conversions::{array_buffer_to_vec, array_buffer_view_data, array_buffer_view_data_checked};
use dom::bindings::conversions::{array_buffer_view_to_vec, array_buffer_view_to_vec_checked};
use dom::bindings::error::{Error, Fallible};
use dom::bindings::inheritance::Castable;
use dom::bindings::js::{JS, LayoutJS, MutNullableHeap, MutHeap, Root};
use dom::bindings::num::Finite;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::bindings::trace::JSTraceable;
use dom::event::{Event, EventBubbles, EventCancelable};
use dom::eventtarget::EventTarget;
use dom::globalscope::GlobalScope;
use dom::htmlcanvaselement::HTMLCanvasElement;
use dom::promise::Promise;
use dom::node::{Node, NodeDamage, window_from_node};
use dom::vrdisplaycapabilities::VRDisplayCapabilities;
use dom::vrstageparameters::VRStageParameters;
use dom::vreyeparameters::VREyeParameters;
use dom::vrframedata::VRFrameData;
use dom::vrpose::VRPose;
use heapsize::HeapSizeOf;
use ipc_channel::ipc::{self, IpcSender};
use js::conversions::ConversionBehavior;
use js::jsapi::{JSContext, JSObject, JS_GetArrayBufferViewType, Type};
use js::jsval::{BooleanValue, DoubleValue, Int32Value, JSVal, NullValue, UndefinedValue};
use net_traits::image::base::PixelFormat;
use net_traits::image_cache_thread::ImageResponse;
use script_traits::ScriptMsg as ConstellationMsg;
use std::cell::Cell;
use std::rc::Rc;
use vr::webvr;

#[dom_struct]
pub struct VRDisplay {
    eventtarget: EventTarget,
    display: DOMRefCell<WebVRDisplayData>,
    depth_near: Cell<f64>,
    depth_far: Cell<f64>,
    connected: Cell<bool>,
    left_eye_params: MutHeap<JS<VREyeParameters>>,
    right_eye_params: MutHeap<JS<VREyeParameters>>,
    capabilities: MutHeap<JS<VRDisplayCapabilities>>,
    stage_params: MutNullableHeap<JS<VRStageParameters>>,
    frame_data: DOMRefCell<WebVRFrameData> 
}

// Wrappers to include WebVR structs in a DOM struct
#[derive(Clone)]
pub struct WebVRDisplayData(webvr::VRDisplayData);
no_jsmanaged_fields!(WebVRDisplayData);
impl HeapSizeOf for WebVRDisplayData {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}

#[derive(Clone, Default)]
pub struct WebVRFrameData(webvr::VRFrameData);
no_jsmanaged_fields!(WebVRFrameData);
impl HeapSizeOf for WebVRFrameData {
    fn heap_size_of_children(&self) -> usize {
        0
    }
}


impl VRDisplay {

    fn new_inherited(display:&webvr::VRDisplayData, global: &GlobalScope) -> VRDisplay {

        let stage = match display.stage_parameters {
            Some(ref params) => Some(VRStageParameters::new(&params, &global)),
            None => None
        };

        VRDisplay {
            eventtarget: EventTarget::new_inherited(),
            display: DOMRefCell::new(WebVRDisplayData(display.clone())),
            depth_near: Cell::new(0.01),
            depth_far: Cell::new(10000.0),
            connected: Cell::new(false),
            left_eye_params: MutHeap::new(&*VREyeParameters::new(&display.left_eye_parameters, &global)),
            right_eye_params: MutHeap::new(&*VREyeParameters::new(&display.right_eye_parameters, &global)),
            capabilities: MutHeap::new(&*VRDisplayCapabilities::new(&display.capabilities, &global)),
            stage_params: MutNullableHeap::new(stage.as_ref()),
            frame_data: DOMRefCell::new(Default::default())
        }
    }

    pub fn new(display:&webvr::VRDisplayData, global: &GlobalScope) -> Root<VRDisplay> {
        reflect_dom_object(box VRDisplay::new_inherited(display, global),
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
        self.connected.get()
    }

    fn IsPresenting(&self) -> bool {
        unimplemented!()
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
        DOMString::from(self.display.borrow().0.display_name)
    }

    fn GetFrameData(&self, frameData: &VRFrameData) -> bool {
        frameData.update(&self.frame_data.borrow().0);
        true
    }

    fn GetPose(&self) -> Root<VRPose> {
        VRPose::new(&self.global(), &self.frame_data.borrow().0.pose)
    }

    fn ResetPose(&self) -> () {
        unimplemented!()
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

    fn RequestAnimationFrame(&self, callback: Rc<FrameRequestCallback>) -> i32 {
        unimplemented!()
    }

    fn CancelAnimationFrame(&self, handle: i32) -> () {
        unimplemented!()
    }

    fn RequestPresent(&self, layers: Vec<VRLayer>) -> Rc<Promise> {
        unimplemented!()
    }

    fn ExitPresent(&self) -> Rc<Promise> {
        unimplemented!()
    }

    fn SubmitFrame(&self) -> () {
        unimplemented!()
    }
}
