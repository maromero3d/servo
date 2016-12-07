/* This Source Code Form is subject to the terms of the Mozilla Public
 * License, v. 2.0. If a copy of the MPL was not distributed with this
 * file, You can obtain one at http://mozilla.org/MPL/2.0/. */

// https://www.w3.org/TR/gamepad/#gamepad-interface
interface Gamepad {
    readonly attribute DOMString id;
    readonly attribute long index;
    readonly attribute boolean connected;
    readonly attribute DOMHighResTimeStamp timestamp;
    //readonly attribute GamepadMappingType mapping;
    readonly attribute DOMString mapping;
    //readonly attribute double[] axes;
    readonly attribute Float64Array axes;
    [SameObject] readonly attribute GamepadButtonList buttons;
    readonly attribute VRPose? pose;
};
