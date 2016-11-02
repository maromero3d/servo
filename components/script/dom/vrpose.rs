/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use core::nonzero::NonZero;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::VRPoseBinding;
use dom::bindings::codegen::Bindings::VRPoseBinding::VRPoseMethods;
use dom::bindings::conversions::{slice_to_array_buffer_view, update_array_buffer_view};
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::globalscope::GlobalScope;
use js::jsapi::{Heap, JSContext, JSObject};
use vr::webvr;

#[dom_struct]
pub struct VRPose {
    reflector_: Reflector,
    position: DOMRefCell<Option<Heap<*mut JSObject>>>,
    orientation: DOMRefCell<Option<Heap<*mut JSObject>>>,
    linear_vel: DOMRefCell<Option<Heap<*mut JSObject>>>,
    angular_vel: DOMRefCell<Option<Heap<*mut JSObject>>>,
    linear_acc: DOMRefCell<Option<Heap<*mut JSObject>>>,
    angular_acc: DOMRefCell<Option<Heap<*mut JSObject>>>
}

#[allow(unsafe_code)]
fn update_typed_array(cx: *mut JSContext, 
                      src: Option<&[f32]>, 
                      dst: &DOMRefCell<Option<Heap<*mut JSObject>>>) {

    match src {
        Some(data) => {
            if let Some(ref heap) = *dst.borrow() {
                unsafe { 
                    update_array_buffer_view(heap.get(), data)
                }
                return;
            } 
            let mut heap = Heap::default();
            heap.set(slice_to_array_buffer_view(cx, data));
            *dst.borrow_mut() = Some(heap);
            
        },
        None => *dst.borrow_mut() = None
    }
}

impl VRPose {

    #[allow(unrooted_must_root)]
    fn new_inherited(global: &GlobalScope, pose: &webvr::VRPose) -> VRPose {
        let result = VRPose {
            reflector_: Reflector::new(),
            position: DOMRefCell::new(None),
            orientation: DOMRefCell::new(None),
            linear_vel: DOMRefCell::new(None),
            angular_vel: DOMRefCell::new(None),
            linear_acc: DOMRefCell::new(None),
            angular_acc: DOMRefCell::new(None)
        };
        result.update(&global, &pose);
        result
    }

    pub fn new(global: &GlobalScope, pose: &webvr::VRPose) -> Root<VRPose> {
        reflect_dom_object(box VRPose::new_inherited(global, &pose),
                           global,
                           VRPoseBinding::Wrap)
    }

    pub fn update(&self, global: &GlobalScope, pose: &webvr::VRPose) {
        let cx = global.get_cx();
        update_typed_array(cx, pose.position.as_ref().map(|v| &v[..]), &self.position);
        update_typed_array(cx, pose.orientation.as_ref().map(|v| &v[..]), &self.orientation);
        update_typed_array(cx, pose.linear_velocity.as_ref().map(|v| &v[..]), &self.linear_vel);
        update_typed_array(cx, pose.angular_velocity.as_ref().map(|v| &v[..]), &self.angular_vel);
        update_typed_array(cx, pose.linear_acceleration.as_ref().map(|v| &v[..]), &self.linear_acc);
        update_typed_array(cx, pose.angular_acceleration.as_ref().map(|v| &v[..]), &self.angular_acc);
    }
}

impl VRPoseMethods for VRPose {
    #[allow(unsafe_code)]
    fn GetPosition(&self, _cx: *mut JSContext) -> Option<NonZero<*mut JSObject>> {
        self.position.borrow().as_ref().map(|v| {
            unsafe { NonZero::new(v.get()) }
        })
    }

    #[allow(unsafe_code)]
    fn GetLinearVelocity(&self, _cx: *mut JSContext) -> Option<NonZero<*mut JSObject>> {
        self.linear_vel.borrow().as_ref().map(|v| {
            unsafe { NonZero::new(v.get()) }
        })
    }

    #[allow(unsafe_code)]
    fn GetLinearAcceleration(&self, _cx: *mut JSContext) -> Option<NonZero<*mut JSObject>> {
        self.linear_acc.borrow().as_ref().map(|v| {
            unsafe { NonZero::new(v.get()) }
        })
    }

    #[allow(unsafe_code)]
    fn GetOrientation(&self, _cx: *mut JSContext) -> Option<NonZero<*mut JSObject>> {
        self.orientation.borrow().as_ref().map(|v| {
            unsafe { NonZero::new(v.get()) }
        })
    }

    #[allow(unsafe_code)]
    fn GetAngularVelocity(&self, _cx: *mut JSContext) -> Option<NonZero<*mut JSObject>> {
        self.angular_vel.borrow().as_ref().map(|v| {
            unsafe { NonZero::new(v.get()) }
        })
    }

    #[allow(unsafe_code)]
    fn GetAngularAcceleration(&self, _cx: *mut JSContext) -> Option<NonZero<*mut JSObject>> {
        self.angular_acc.borrow().as_ref().map(|v| {
            unsafe { NonZero::new(v.get()) }
        })
    }
}
