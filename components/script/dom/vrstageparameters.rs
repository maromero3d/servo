/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

use core::nonzero::NonZero;
use dom::bindings::cell::DOMRefCell;
use dom::bindings::codegen::Bindings::VRStageParametersBinding;
use dom::bindings::codegen::Bindings::VRStageParametersBinding::VRStageParametersMethods;
use dom::bindings::conversions::slice_to_array_buffer_view;
use dom::bindings::js::Root;
use dom::bindings::reflector::{Reflector, reflect_dom_object};
use dom::bindings::num::Finite;
use dom::globalscope::GlobalScope;
use js::jsapi::{Heap, JSContext, JSObject};
use vr::webvr;

#[dom_struct]
pub struct VRStageParameters {
    reflector_: Reflector,
    #[ignore_heap_size_of = "Defined in rust-webvr"]
    parameters: DOMRefCell<WebVRStageParameters>,
    transform: Heap<*mut JSObject>,
}

// Wrappers required to include WebVR structs in a DOM struct
#[derive(Clone)]
pub struct WebVRStageParameters(webvr::VRStageParameters);
no_jsmanaged_fields!(WebVRStageParameters);

impl VRStageParameters {

    #[allow(unrooted_must_root)]
    fn new_inherited(parameters: &webvr::VRStageParameters, global: &GlobalScope) -> VRStageParameters {
        let mut stage = VRStageParameters {
            reflector_: Reflector::new(),
            parameters: DOMRefCell::new(WebVRStageParameters(parameters.clone())),
            transform: Heap::default()
        };
        stage.transform.set(slice_to_array_buffer_view(global.get_cx(), &parameters.sitting_to_standing_transform));

        stage
    }

    pub fn new(parameters: &webvr::VRStageParameters, global: &GlobalScope) -> Root<VRStageParameters> {
        reflect_dom_object(box VRStageParameters::new_inherited(&parameters, global),
                           global,
                           VRStageParametersBinding::Wrap)
    }
}

impl VRStageParametersMethods for VRStageParameters {

    // https://w3c.github.io/webvr/#dom-vrstageparameters-sittingtostandingtransform
    #[allow(unsafe_code)]
    fn SittingToStandingTransform(&self, _cx: *mut JSContext) -> NonZero<*mut JSObject> {
        unsafe { NonZero::new(self.transform.get()) }
    }

    // https://w3c.github.io/webvr/#dom-vrstageparameters-sizex
    fn SizeX(&self) -> Finite<f32> {
        Finite::wrap(self.parameters.borrow().0.size_x)
    }

    // https://w3c.github.io/webvr/#dom-vrstageparameters-sizez
    fn SizeZ(&self) -> Finite<f32> {
        Finite::wrap(self.parameters.borrow().0.size_y)
    }
}
