use std::sync::Arc;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;

use crate::{editor::OsClaPhotonEditorEvent, editor::OscSettings, OsClaPhotonParams};

pub struct ParamView;

impl ParamView {
    pub fn new<P>(cx: &mut Context, params: P) -> Handle<Self>
    where
        P: Lens<Target = Arc<OsClaPhotonParams>> + Copy,
    {
        //TODO handle param names
        Self.build(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "Beams Nr").class("label");
                ParamSlider::new(cx, params, |params| &params.beamNr)
                    .class("widget");
            })
            .class("row");
            HStack::new(cx, |cx| {
                Label::new(cx, "Rotation Speed").class("label");
                ParamSlider::new(cx, params, |params| &params.rotSpeed)
                    .class("widget");
            })
            .class("row");
            HStack::new(cx, |cx| {
                Label::new(cx, "Beams Size").class("label");
                ParamSlider::new(cx, params, |params| &params.beamSz)
                    .class("widget");
            })
            .class("row");
            HStack::new(cx, |cx| {
                Label::new(cx, "Zoom").class("label");
                ParamSlider::new(cx, params, |params| &params.zoom)
                    .class("widget");
            })
            .class("row");
            HStack::new(cx, |cx| {
                Label::new(cx, "Zoom Speed").class("label");
                ParamSlider::new(cx, params, |params| &params.zoomSp)
                    .class("widget");
            })
            .class("row");
            HStack::new(cx, |cx| {
                Label::new(cx, "Offset").class("label");
                ParamSlider::new(cx, params, |params| &params.offset)
                    .class("widget");
            })
            .class("row");
            // HStack::new(cx, |cx| {
            //     Label::new(cx, "param7").class("label");
            //     ParamSlider::new(cx, params, |params| &params.param7)
            //         .class("widget");
            // })
            // .class("row");
            // HStack::new(cx, |cx| {
            //     Label::new(cx, "param8").class("label");
            //     ParamSlider::new(cx, params, |params| &params.param8)
            //         .class("widget");
            // })
            // .class("row");
        })
    }
}

impl View for ParamView {
    fn element(&self) -> Option<&'static str> {
        Some("generic-ui")
    }
}


pub struct SettingsView;

impl SettingsView {
    pub fn new<S,P,L>(cx: &mut Context, settings: S, params: P, log: L) -> Handle<Self>
    where
        S: Lens<Target = OscSettings> + Copy,
        P: Lens<Target = Arc<OsClaPhotonParams>> + Copy,
        L: Lens<Target = Vec<String>>,
    {
        Self.build(cx, |cx| {
            HStack::new(cx, |cx| {
                Label::new(cx, "TD OSC IP").class("label");
                Textbox::new(cx, settings.map(|settings| settings.osc_server_address.clone()))
                    .on_edit(move |cx, text| {
                        //TODO: validate
                        cx.emit(OsClaPhotonEditorEvent::SetOscServerAddress(text));
                    })
                    .on_submit(|cx,  _, _| {
                        cx.emit(OsClaPhotonEditorEvent::ConnectionChange);
                    })
                    .width(Pixels(135.0)); // 200 = 135 + 60 + 5
                Textbox::new(cx, settings.map(|settings| settings.osc_server_port))
                    .on_edit(move |cx, text| {
                        if let Ok(val) = text.parse::<u16>() {
                            cx.emit(OsClaPhotonEditorEvent::SetOscServerPort(val));
                            cx.toggle_class("invalid", false);
                        } else {
                            cx.toggle_class("invalid", true);
                        }
                    })
                    .on_submit(|cx,  _, _| {
                        cx.emit(OsClaPhotonEditorEvent::ConnectionChange);
                    })
                    .width(Pixels(60.0));
            })
            .class("row");
            // .col_between(Pixels(5.0));
            HStack::new(cx, |cx| {
                Label::new(cx, "OSC Address Base").class("label");
                Textbox::new(cx, settings.map(|settings| settings.osc_address_base.clone()))
                    .on_edit(move |cx, text| {
                        //TODO: validate
                        cx.emit(OsClaPhotonEditorEvent::SetOscAddressBase(text));
                    })
                    .on_submit(|cx,  _, _| {
                        cx.emit(OsClaPhotonEditorEvent::AddressBaseChange);
                    })
                    .width(Pixels(200.0));
            })
            .class("row");
            // HStack::new(cx, |cx| {
            //     Label::new(cx, "Send MIDI").class("label");
            //     ParamSlider::new(cx, params, |params| &params.flag_send_midi)
            //         .class("widget");
            // })
            // .class("row");
            // HStack::new(cx, |cx| {
            //     Label::new(cx, "Send Audio").class("label");
            //     ParamSlider::new(cx, params, |params| &params.flag_send_audio)
            //     .class("widget");
            // })
            // .class("row");
            VirtualList::new(cx, log, 20.0, |cx, _index, item| {
                return Label::new(cx, item).left(Pixels(0.0)).class("label");
            })
            .height(Pixels(180.0))
            .class("row");
        })
    }
}

impl View for SettingsView {
    fn element(&self) -> Option<&'static str> {
        Some("generic-ui")
    }
}
