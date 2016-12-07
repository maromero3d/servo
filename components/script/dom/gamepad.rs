/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use core::nonzero::NonZero;
use core::ops::Deref;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::GamepadBinding;
use dom::bindings::codegen::Bindings::GamepadBinding::GamepadMethods;
use dom::bindings::conversions::{slice_to_array_buffer_view, update_array_buffer_view};
use dom::bindings::js::{JS, MutNullableHeap, Root};
use dom::bindings::num::Finite;
use dom::bindings::reflector::{Reflectable, Reflector, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::globalscope::GlobalScope;
use dom::gamepadbuttonlist::GamepadButtonList;
use dom::vrpose::VRPose;
use js::jsapi::{Heap, JSContext, JSObject};
use std::cell::Cell;
use vr_traits::webvr;

#[dom_struct]
pub struct Gamepad {
    reflector_: Reflector,
    id: DOMRefCell<String>,
    index: Cell<u64>,
    connected: Cell<bool>,
    timestamp: Cell<f64>,
    mapping_type: DOMRefCell<String>,
    axes: DOMRefCell<Heap<*mut JSObject>>,
    buttons: JS<GamepadButtonList>,
    pose: MutNullableHeap<JS<VRPose>>
}

impl Gamepad {
    #[allow(unrooted_must_root)]
    pub fn new_from_vr(global: &GlobalScope,
                       index: u64,
                       state: &webvr::VRGamepadState) -> Root<Gamepad> {
        let buttons = GamepadButtonList::new_from_vr(&global, &state.buttons);
        let pose = VRPose::new(&global, &state.pose);

        let gamepad = Gamepad {
            reflector_: Reflector::new(),
            id: DOMRefCell::new("t4st".into()),
            index: Cell::new(index),
            connected: Cell::new(state.connected),
            timestamp: Cell::new(state.timestamp),
            mapping_type: DOMRefCell::new("t4st".into()),
            axes: DOMRefCell::new(Heap::default()),
            buttons: JS::from_ref(&*buttons),
            pose: MutNullableHeap::new(Some(pose.deref()))
        };
      
        let root = reflect_dom_object(box gamepad,
                           global,
                           GamepadBinding::Wrap);
        root.init_axes(&state.axes);
        root
    }
}

impl GamepadMethods for Gamepad {
    // https://www.w3.org/TR/gamepad/
    fn Id(&self) -> DOMString {
        DOMString::from(self.id.borrow().clone())
    }

    // https://www.w3.org/TR/gamepad/
    fn Index(&self) -> i32 {
        self.index.get() as i32
    }

    // https://www.w3.org/TR/gamepad/
    fn Connected(&self) -> bool {
        self.connected.get()
    }

    // https://www.w3.org/TR/gamepad/
    fn Timestamp(&self) -> Finite<f64> {
        Finite::wrap(self.timestamp.get())
    }

    // https://www.w3.org/TR/gamepad/
    fn Mapping(&self) -> DOMString {
        DOMString::from(self.mapping_type.borrow().clone())
    }

    #[allow(unsafe_code)]
    // https://www.w3.org/TR/gamepad/
    unsafe fn Axes(&self, _cx: *mut JSContext) -> NonZero<*mut JSObject> {
        NonZero::new(self.axes.borrow().get())
    }

    // https://www.w3.org/TR/gamepad/
    fn Buttons(&self) -> Root<GamepadButtonList> {
        Root::from_ref(&*self.buttons)
    }

    // https://www.w3.org/TR/gamepad/
    fn GetPose(&self) -> Option<Root<VRPose>> {
        self.pose.get().map(|p| Root::from_ref(&*p))
    }
}

impl Gamepad {
    #[allow(unsafe_code)]
    pub fn update_from_vr(&self, state: &webvr::VRGamepadState) {
        self.connected.set(state.connected);
        self.timestamp.set(state.timestamp);
        unsafe {
            update_array_buffer_view(self.axes.borrow_mut().get(), &state.axes);
        }
        self.buttons.sync_vr(&state.buttons);
        self.pose.get().unwrap().update(&state.pose);
    }

    fn init_axes(&self, axes: &Vec<f64>) {
        self.axes.borrow_mut().set(slice_to_array_buffer_view(self.global().get_cx(), &axes));
    }

    pub fn gamepad_id(&self) -> u64 {
        self.index.get()
    }
}

