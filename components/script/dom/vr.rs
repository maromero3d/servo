/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::VRBinding;
use dom::bindings::codegen::Bindings::VRBinding::VRMethods;
use dom::bindings::error::Error;
use dom::bindings::js::{JS, Root};
use dom::bindings::reflector::{Reflectable, reflect_dom_object};
use dom::eventtarget::EventTarget;
use dom::promise::Promise;
use dom::globalscope::GlobalScope;
use dom::vrdisplay::VRDisplay;
use ipc_channel::ipc;
use ipc_channel::ipc::IpcSender;
use std::rc::Rc;
use vr::webvr;
use vr::WebVRMsg;

#[dom_struct]
pub struct VR {
    eventtarget: EventTarget,
    displays: DOMRefCell<Vec<JS<VRDisplay>>>
}

impl VR {
    fn new_inherited() -> VR {
        VR {
            eventtarget: EventTarget::new_inherited(),
            displays: DOMRefCell::new(Vec::new())
        }
    }

    pub fn new(global: &GlobalScope) -> Root<VR> {
        reflect_dom_object(box VR::new_inherited(),
                           global,
                           VRBinding::Wrap)
    }
}

impl VRMethods for VR {

    // https://w3c.github.io/webvr/#interface-navigator
    #[allow(unrooted_must_root)]
    fn GetVRDisplays(&self) -> Rc<Promise> {

        let promise = Promise::new(&self.global());
        if !self.VrEnabled() {
            // WebVR spec: The Promise MUST be rejected if the Document object is inside 
            // an iframe that does not have the allowvr attribute set.
            promise.reject_error(promise.global().get_cx(), Error::Security);
            return promise;
        }

        if let Some(wevbr_sender) = self.webvr_thread() {
            let (sender, receiver) = ipc::channel().unwrap();
            wevbr_sender.send(WebVRMsg::GetVRDisplays(sender)).unwrap();
            match receiver.recv().unwrap() {
                Ok(displays) => {
                    // Sync displays
                    for display in displays {
                        self.sync_display(&display);
                    }
                },
                Err(e) => {
                    promise.reject_native(promise.global().get_cx(), &e);
                    return promise;
                }
            }
        }

        // convert from JS to Root
        let displays: Vec<Root<VRDisplay>> = self.displays.borrow().iter()
                                                          .map(|d| Root::from_ref(&**d))
                                                          .collect();
        promise.resolve_native(promise.global().get_cx(), &displays);

        promise
    }

    // https://w3c.github.io/webvr/#interface-navigator
    fn VrEnabled(&self) -> bool {
        // TODO: check iframe
        true
    }
}


impl VR {

    fn webvr_thread(&self) -> Option<IpcSender<WebVRMsg>> {
        self.global().as_window().webvr_thread()
    }

    fn find_display(&self, display_id: u64) -> Option<Root<VRDisplay>> {
        self.displays.borrow()
                     .iter()
                     .find(|d| d.get_display_id() == display_id)
                     .map(|d| Root::from_ref(&**d))
    }

    fn sync_display(&self, display: &webvr::VRDisplayData) {
        if let Some(existing) = self.find_display(display.display_id) {
            existing.update_display(&display);
        } else {
            let root = VRDisplay::new(&self.global(), &display);
            self.displays.borrow_mut().push(JS::from_ref(&*root));
        }
    }
}