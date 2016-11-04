/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use dom::bindings::codegen::Bindings::EventBinding::EventBinding::EventMethods;
use dom::bindings::codegen::Bindings::VRDisplayEventBinding;
use dom::bindings::codegen::Bindings::VRDisplayEventBinding::VRDisplayEventMethods;
use dom::bindings::codegen::Bindings::VRDisplayEventBinding::VRDisplayEventReason;
use dom::bindings::error::Fallible;
use dom::bindings::inheritance::Castable;
use dom::bindings::js::Root;
use dom::bindings::reflector::reflect_dom_object;
use dom::bindings::str::DOMString;
use dom::event::Event;
use dom::globalscope::GlobalScope;
use dom::vrdisplay::VRDisplay;
use string_cache::Atom;
use vr_traits::webvr;

#[dom_struct]
pub struct VRDisplayEvent {
    event: Event,
    display: Root<VRDisplay>,
    reason: Option<VRDisplayEventReason>
}

impl VRDisplayEvent {

    fn new_inherited(display: &Root<VRDisplay>,
                     reason: &Option<VRDisplayEventReason>) 
                     -> VRDisplayEvent {
        VRDisplayEvent {
            event: Event::new_inherited(),
            display: display.clone(),
            reason: reason.clone()
        }
    }

    pub fn new(global: &GlobalScope,
               type_: Atom,
               bubbles: bool,
               cancelable: bool,
               display: &Root<VRDisplay>,
               reason: &Option<VRDisplayEventReason>)
               -> Root<VRDisplayEvent> {
        let ev = reflect_dom_object(box VRDisplayEvent::new_inherited(&display, reason),
                           global,
                           VRDisplayEventBinding::Wrap);
        {
            let event = ev.upcast::<Event>();
            event.init_event(type_, bubbles, cancelable);
        }
        ev
    }

    pub fn new_from_webvr(global: &GlobalScope,
                          display: &Root<VRDisplay>,
                          event: webvr::VRDisplayEvent) 
                          -> Root<VRDisplayEvent> {
        let (name, reason) = match event {
            webvr::VRDisplayEvent::Connect(_) => ("onvrdisplayconnect", None),
            webvr::VRDisplayEvent::Disconnect(_) => ("onvrdisplaydisconnect", None),
            webvr::VRDisplayEvent::Activate(_, reason) => ("onvrdisplayactivate", Some(reason)),
            webvr::VRDisplayEvent::Deactivate(_, reason) => ("onvrdisplaydeactivate", Some(reason)),
            webvr::VRDisplayEvent::Blur(_) => ("onvrdisplayblur", None),
            webvr::VRDisplayEvent::Focus(_) => ("onvrdisplayfocus", None),
            webvr::VRDisplayEvent::PresentChange(_) => ("onvrdisplaypresentchange", None),
            webvr::VRDisplayEvent::Change(_) => panic!("VRDisplayEvent:Change event not available in WebVR")
        };

        // map to JS enum values
        let reason = reason.map(|r| {
            match r {
                webvr::VRDisplayEventReason::Navigation => VRDisplayEventReason::Navigation,
                webvr::VRDisplayEventReason::Mounted => VRDisplayEventReason::Mounted,
                webvr::VRDisplayEventReason::Unmounted => VRDisplayEventReason::Unmounted,
            }
        });

        VRDisplayEvent::new(&global, 
                            Atom::from(DOMString::from(name)),
                            false,
                            false,
                            &display,
                            &reason)
    }

    pub fn Constructor(global: &GlobalScope,
                       type_: DOMString,
                       init: &VRDisplayEventBinding::VRDisplayEventInit)
                       -> Fallible<Root<VRDisplayEvent>> {
        Ok(VRDisplayEvent::new(global,
                            Atom::from(type_),
                            init.parent.bubbles,
                            init.parent.cancelable,
                            &init.display,
                            &init.reason))
    }
}

impl VRDisplayEventMethods for VRDisplayEvent {

    // https://w3c.github.io/webvr/#dom-vrdisplayevent-display
    fn Display(&self) -> Root<VRDisplay> {
        Root::from_ref(&*self.display)
    }

    // https://w3c.github.io/webvr/#enumdef-vrdisplayeventreason
    fn GetReason(&self) -> Option<VRDisplayEventReason> {
        self.reason
    }

    // https://dom.spec.whatwg.org/#dom-event-istrusted
    fn IsTrusted(&self) -> bool {
        self.event.IsTrusted()
    }
}
