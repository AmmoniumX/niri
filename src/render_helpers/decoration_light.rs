use std::collections::HashMap;
use std::rc::Rc;

use niri_config::decoration_shader::DecorationLight as LightConfig;
use smithay::backend::allocator::Fourcc;
use smithay::backend::renderer::element::{Element, Id, Kind};
use smithay::backend::renderer::gles::{GlesRenderer, Uniform};
use smithay::backend::renderer::{Bind as _, Offscreen as _, Renderer as _, Texture as _};
use smithay::utils::{Logical, Point, Size};

use super::blur::{Blur, BlurOptions};
use super::offscreen::OffscreenBuffer;
use super::shader_element::ShaderRenderElement;
use super::shaders::ProgramType;
use super::solid_color::{SolidColorBuffer, SolidColorRenderElement};
use crate::layout::focus_ring::FocusRingRenderElement;

/// Cached emission and blur textures for one decoration. The light never samples client content.
#[derive(Debug)]
pub struct DecorationLight {
    emission: OffscreenBuffer,
    bounds: SolidColorBuffer,
    blur: Option<Blur>,
    blur_format: Option<Fourcc>,
    composite: ShaderRenderElement,
    config: Option<(LightConfig, f64)>,
    offset: Point<f64, Logical>,
}

impl Default for DecorationLight {
    fn default() -> Self {
        Self {
            emission: OffscreenBuffer::default(),
            bounds: SolidColorBuffer::new((0., 0.), [0.; 4]),
            blur: None,
            blur_format: None,
            composite: ShaderRenderElement::empty(ProgramType::DecorationLight, Kind::Unspecified)
                .with_screen_blend(),
            config: None,
            offset: Point::default(),
        }
    }
}

impl DecorationLight {
    pub fn id(&self) -> &Id {
        self.composite.id()
    }

    pub fn clear(&mut self) {
        self.emission.clear();
        self.blur = None;
        self.blur_format = None;
        self.composite.clear_textures();
        self.config = None;
    }

    /// Uses half-resolution logical pixels regardless of monitor DPI. The shader itself
    /// retains its original AA scale and exact frame time. No fullscreen capture is needed.
    pub fn render(
        &mut self,
        renderer: &mut GlesRenderer,
        mut sources: Vec<FocusRingRenderElement>,
        config: LightConfig,
        width: f64,
        location: Point<f64, Logical>,
        alpha: f32,
    ) -> anyhow::Result<ShaderRenderElement> {
        const SCALE: f64 = 0.5;
        let geo = super::encompassing_geo(SCALE.into(), sources.iter())
            .to_f64()
            .to_logical(SCALE);
        // Include the blur taps and bilinear footprints, with a transparent guard band so
        // clamped texture sampling cannot smear lit pixels across the boundary.
        let margin = (config.spread.0 * 2. + 8.).ceil();
        let size = geo.size + Size::from((margin * 2., margin * 2.));
        self.bounds.resize(size);
        sources.push(
            SolidColorRenderElement::from_buffer(
                &self.bounds,
                geo.loc - Point::from((margin, margin)),
                1.,
                Kind::Unspecified,
            )
            .into(),
        );
        if self
            .config
            .is_some_and(|(previous, _)| previous.threshold != config.threshold)
        {
            self.emission.clear();
        }
        let (emission, _, data) = self.emission.render(renderer, SCALE.into(), &sources)?;
        let params = (config, width);
        let changed = self.config != Some(params)
            || data.damaged
            || self
                .blur
                .as_ref()
                .is_none_or(|b| b.context_id() != renderer.context_id());
        if changed {
            self.composite.clear_textures();
            if self
                .blur
                .as_ref()
                .is_none_or(|b| b.context_id() != renderer.context_id())
            {
                self.blur = Blur::new(renderer);
                self.blur_format = None;
            }
            let blur = self
                .blur
                .as_mut()
                .ok_or_else(|| anyhow::anyhow!("blur shader unavailable"))?;
            // Dual Kawase: keep offsets small and use the mip ladder for wide spill. For
            // P passes, conservative support is 3 * offset * (2^P - 1) source pixels.
            let radius = config.spread.0 * SCALE;
            let passes = ((radius / 3. + 1.).log2().ceil() as u8).clamp(1, 6);
            let offset = radius / (3. * (2_f64.powi(passes as i32) - 1.));
            let options = BlurOptions { passes, offset };
            blur.prepare_textures(
                |_, size| {
                    if let Some(format) = self.blur_format {
                        return renderer.create_buffer(format, size);
                    }
                    // Bloom amplifies very small values. Eight-bit intermediates produce
                    // visible coloured bands; prefer float, with compatible fallbacks.
                    for format in [Fourcc::Abgr16161616f, Fourcc::Abgr2101010] {
                        if let Ok(mut texture) = renderer.create_buffer(format, size) {
                            if renderer.bind(&mut texture).is_ok() {
                                self.blur_format = Some(format);
                                return Ok(texture);
                            }
                        }
                    }
                    self.blur_format = Some(Fourcc::Abgr8888);
                    renderer.create_buffer(Fourcc::Abgr8888, size)
                },
                emission.texture(),
                options,
            )?;
            let texture = blur.render(renderer, emission.texture(), options)?;
            let texture_size = texture.size();
            let logical_size = emission.logical_size();
            // OffscreenBuffer may reuse a larger allocation; use its source rectangle only.
            let tex_scale = [
                emission.src().size.w as f32 / texture_size.w as f32,
                emission.src().size.h as f32 / texture_size.h as f32,
            ];
            let gain = config.intensity.0 * (config.spread.0 / width.max(1.)).max(1.);
            self.composite.update(
                logical_size,
                None,
                1.,
                1.,
                Rc::new([
                    Uniform::new("light_gain", gain as f32),
                    Uniform::new("light_tex_scale", tex_scale),
                ]),
                HashMap::from([("light_texture".to_owned(), texture)]),
            );
            self.offset = emission.offset();
            self.config = Some(params);
        }
        self.composite.set_alpha(alpha);
        Ok(self.composite.clone().with_location(location + self.offset))
    }
}
