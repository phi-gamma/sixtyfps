/* LICENSE BEGIN
    This file is part of the SixtyFPS Project -- https://sixtyfps.io
    Copyright (c) 2021 Olivier Goffart <olivier.goffart@sixtyfps.io>
    Copyright (c) 2021 Simon Hausmann <simon.hausmann@sixtyfps.io>

    SPDX-License-Identifier: GPL-3.0-only
    This file is also available under commercial licensing terms.
    Please contact info@sixtyfps.io for more information.
LICENSE END */

use sixtyfps::SharedString;
use std::rc::Rc;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

sixtyfps::sixtyfps! {
    import { Slider, HorizontalBox, VerticalBox, GroupBox, ComboBox } from "sixtyfps_widgets.60";

    export MainWindow := Window {
        title: "SixtyFPS Image Filter Integration Example";
        preferred-width: 800px;
        preferred-height: 600px;

        property original-image <=> original.source;
        property filters <=> filter-combo.model;
        callback filter-image(int) -> image;

        HorizontalBox {
            VerticalBox {
                Text {
                    font-size: 20px;
                    text: "Original Image";
                    horizontal-alignment: center;
                }
                original := Image { }
            }
            VerticalBox {
                alignment: center;
                filter-combo := ComboBox {
                    current-value: "Blur";
                    current-index: 0;
                    vertical-stretch: 0;
                }
            }
            VerticalBox {
                Text {
                    font-size: 20px;
                    text: "Filtered Image";
                    horizontal-alignment: center;
                }
                Image {
                    source: filter-image(filter-combo.current-index);
                }
            }
        }
    }
}

struct Filter {
    name: SharedString,
    apply_function: fn(&image::RgbaImage) -> image::RgbaImage,
}

struct Filters(Vec<Filter>);

impl sixtyfps::Model for Filters {
    type Data = SharedString;

    fn row_count(&self) -> usize {
        self.0.len()
    }

    fn row_data(&self, row: usize) -> Self::Data {
        self.0[row].name.clone()
    }

    fn attach_peer(&self, _peer: sixtyfps::ModelPeer) {
        // The model is read_only: nothing to do
    }
}

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    // This provides better error messages in debug mode.
    // It's disabled in release mode so it doesn't bloat up the file size.
    #[cfg(all(debug_assertions, target_arch = "wasm32"))]
    console_error_panic_hook::set_once();

    let main_window = MainWindow::new();

    #[cfg(target_arch = "wasm32")]
    let source_image =
        { image::load_from_memory(include_bytes!("cat_preview_round.png")).unwrap().into_rgba8() };
    #[cfg(not(target_arch = "wasm32"))]
    let source_image = {
        let mut cat_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        cat_path.push("cat_preview_round.png");
        image::open(&cat_path).expect("Error loading cat image").into_rgba8()
    };

    main_window.set_original_image(sixtyfps::Image::from_rgba8(
        sixtyfps::SharedPixelBuffer::clone_from_slice(
            source_image.as_raw(),
            source_image.width() as _,
            source_image.height() as _,
        ),
    ));

    let filters = Filters(vec![
        Filter {
            name: "Blur".into(),
            apply_function: |image: &image::RgbaImage| image::imageops::blur(image, 4.),
        },
        Filter {
            name: "Brighten".into(),
            apply_function: |image: &image::RgbaImage| {
                image::imageops::colorops::brighten(image, 30)
            },
        },
        Filter {
            name: "Darken".into(),
            apply_function: |image: &image::RgbaImage| {
                image::imageops::colorops::brighten(image, -30)
            },
        },
        Filter {
            name: "Increase Contrast".into(),
            apply_function: |image: &image::RgbaImage| {
                image::imageops::colorops::contrast(image, 30.)
            },
        },
        Filter {
            name: "Decrease Contrast".into(),
            apply_function: |image: &image::RgbaImage| {
                image::imageops::colorops::contrast(image, -30.)
            },
        },
        Filter {
            name: "Invert".into(),
            apply_function: |image: &image::RgbaImage| {
                let mut inverted = image.clone();
                image::imageops::colorops::invert(&mut inverted);
                inverted
            },
        },
    ]);
    let filters = Rc::new(filters);

    main_window.set_filters(sixtyfps::ModelHandle::new(filters.clone()));

    main_window.on_filter_image(move |filter_index| {
        let filter_fn = filters.0[filter_index as usize].apply_function;
        let filtered_image = filter_fn(&source_image);
        sixtyfps::Image::from_rgba8(sixtyfps::SharedPixelBuffer::clone_from_slice(
            filtered_image.as_raw(),
            filtered_image.width() as _,
            filtered_image.height() as _,
        ))
    });

    main_window.run();
}
