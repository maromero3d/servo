/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use core::nonzero::NonZero;
use core::ops::Deref;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::GamepadBinding;
use dom::bindings::codegen::Bindings::GamepadBinding::GamepadMethods;
use dom::bindings::js::{JS, MutNullableJS, Root};
use dom::bindings::num::Finite;
use dom::bindings::reflector::{DomObject, Reflector, reflect_dom_object};
use dom::bindings::str::DOMString;
use dom::globalscope::GlobalScope;
use dom::gamepadbuttonlist::GamepadButtonList;
use dom_struct::dom_struct;
use dom::vrpose::VRPose;
use js::jsapi::{Heap, JSContext, JSObject};
use js::typedarray::{Float64Array, CreateWith};
use std::cell::Cell;
use webvr_traits::WebVRGamepadState;

#[dom_struct]
pub struct Gamepad {
    reflector_: Reflector,
    id: DOMRefCell<String>,
    index: Cell<u32>,
    connected: Cell<bool>,
    timestamp: Cell<f64>,
    mapping_type: DOMRefCell<String>,
    axes: Heap<*mut JSObject>,
    buttons: JS<GamepadButtonList>,
    pose: MutNullableJS<VRPose>,
    display_id: Cell<u64>
}

impl Gamepad {
    #[allow(unrooted_must_root)]
    pub fn new_from_vr(global: &GlobalScope,
                       name: String,
                       display_id: u64,
                       index: u32,
                       state: &WebVRGamepadState) -> Root<Gamepad> {
        let buttons = GamepadButtonList::new_from_vr(&global, &state.buttons);
        let pose = VRPose::new(&global, &state.pose);

        let gamepad = Gamepad {
            reflector_: Reflector::new(),
            id: DOMRefCell::new(name),
            index: Cell::new(index),
            connected: Cell::new(state.connected),
            timestamp: Cell::new(state.timestamp),
            mapping_type: DOMRefCell::new("".into()),
            axes: Heap::default(),
            buttons: JS::from_ref(&*buttons),
            pose: MutNullableJS::new(Some(pose.deref())),
            display_id: Cell::new(display_id)
        };
      
        let root = reflect_dom_object(box gamepad,
                           global,
                           GamepadBinding::Wrap);
        root.init_axes(&state.axes);
        root
    }
}

impl GamepadMethods for Gamepad {
    // https://www.w3.org/TR/gamepad/#dom-gamepad-id
    fn Id(&self) -> DOMString {
        DOMString::from(self.id.borrow().clone())
    }

    // https://www.w3.org/TR/gamepad/#dom-gamepad-index
    fn Index(&self) -> i32 {
        self.index.get() as i32
    }

    // https://www.w3.org/TR/gamepad/#dom-gamepad-connected
    fn Connected(&self) -> bool {
        self.connected.get()
    }

    // https://www.w3.org/TR/gamepad/#dom-gamepad-timestamp
    fn Timestamp(&self) -> Finite<f64> {
        Finite::wrap(self.timestamp.get())
    }

    // https://www.w3.org/TR/gamepad/#dom-gamepad-mapping
    fn Mapping(&self) -> DOMString {
        DOMString::from(self.mapping_type.borrow().clone())
    }

    #[allow(unsafe_code)]
    // https://www.w3.org/TR/gamepad/#dom-gamepad-axes
    unsafe fn Axes(&self, _cx: *mut JSContext) -> NonZero<*mut JSObject> {
        NonZero::new(self.axes.get())
    }

    // https://www.w3.org/TR/gamepad/#dom-gamepad-buttons
    fn Buttons(&self) -> Root<GamepadButtonList> {
        Root::from_ref(&*self.buttons)
    }

    // https://www.w3.org/TR/gamepad/
    fn GetPose(&self) -> Option<Root<VRPose>> {
        self.pose.get().map(|p| Root::from_ref(&*p))
    }

    // https://w3c.github.io/webvr/spec/1.1/#gamepad-getvrdisplays-attribute
    fn DisplayId(&self) -> u32 {
        self.display_id.get() as u32
    }
}

impl Gamepad {
    #[allow(unsafe_code)]
    pub fn update_from_vr(&self, state: &WebVRGamepadState) {
        self.connected.set(state.connected);
        self.timestamp.set(state.timestamp);
        unsafe {
            let cx = self.global().get_cx();
            typedarray!(in(cx) let axes: Float64Array = self.axes.get());
            if let Ok(mut array) = axes {
                array.update(&state.axes);
            }
        }
        self.buttons.sync_vr(&state.buttons);
        self.pose.get().unwrap().update(&state.pose);
    }

    #[allow(unsafe_code)]
    fn init_axes(&self, axes: &Vec<f64>) {
        unsafe {
            let _ = Float64Array::create(self.global().get_cx(),
                                         CreateWith::Slice(&axes),
                                         self.axes.handle_mut());
        }
    }

    pub fn gamepad_id(&self) -> u64 {
        self.display_id.get()
    }
}
