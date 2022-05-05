use crate::{data, render, settings};
use render::{
    Buffer, Context, DoubleFramebuffer, Framebuffer, Program, TextureOptions, Uniform,
    UniformValue, VertexArrayObject, VertexBufferLayout,
};
use settings::Noise;

use crevice::std140::{AsStd140, Std140};
use glow::HasContext;
use half::f16;
use std::rc::Rc;

static NOISE_VERT_SHADER: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/shaders/noise.vert"));
static SIMPLEX_NOISE_FRAG_SHADER: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/shaders/simplex_noise.frag"));
static INJECT_NOISE_FRAG_SHADER: &'static str =
    include_str!(concat!(env!("OUT_DIR"), "/shaders/inject_noise.frag"));

#[derive(AsStd140)]
pub struct NoiseUniforms {
    scale: f32,
    offset_1: f32,
    offset_2: f32,
    multiplier: f32,
    blend_factor: f32,
}

pub struct NoiseChannel {
    noise: Noise,
    texture: Framebuffer,
    blend_begin_time: f32,
    last_blend_progress: f32,
    offset1: f32,
    offset2: f32,
    uniforms: Buffer,
}

impl NoiseChannel {
    pub fn tick(&mut self, context: &Context, elapsed_time: f32) -> () {
        self.blend_begin_time = elapsed_time;
        self.last_blend_progress = 0.0;
        self.offset1 += self.noise.offset_increment;
        // self.offset2 += self.noise.offset_increment;

        unsafe {
            context.bind_buffer(glow::UNIFORM_BUFFER, Some(self.uniforms.id));
            context.buffer_sub_data_u8_slice(
                glow::UNIFORM_BUFFER,
                1 * 4,
                &bytemuck::bytes_of(&[self.offset1, self.offset2]),
            );
            context.bind_buffer(glow::UNIFORM_BUFFER, None);
        }
    }
}

pub struct NoiseInjector {
    context: Context,
    pub channels: Vec<NoiseChannel>,
    width: u32,
    height: u32,

    generate_noise_pass: Program,
    inject_noise_pass: Program,

    noise_buffer: VertexArrayObject,
}

impl NoiseInjector {
    pub fn update_channel(&mut self, channel_number: usize, noise: &Noise) -> () {
        if let Some(channel) = self.channels.get_mut(channel_number) {
            channel.noise = noise.clone();

            let uniforms = NoiseUniforms {
                scale: noise.scale,
                offset_1: noise.offset_1,
                offset_2: noise.offset_2,
                multiplier: noise.multiplier,
                blend_factor: 0.0,
            };

            unsafe {
                self.context
                    .bind_buffer(glow::UNIFORM_BUFFER, Some(channel.uniforms.id));
                self.context.buffer_sub_data_u8_slice(
                    glow::UNIFORM_BUFFER,
                    0,
                    uniforms.as_std140().as_bytes(),
                );
                self.context.bind_buffer(glow::UNIFORM_BUFFER, None);
            }
        }
    }

    pub fn new(context: &Context, width: u32, height: u32) -> Result<Self, render::Problem> {
        // Geometry
        let plane_vertices = Buffer::from_f32(
            &context,
            &data::PLANE_VERTICES,
            glow::ARRAY_BUFFER,
            glow::STATIC_DRAW,
        )?;
        let plane_indices = Buffer::from_u16(
            &context,
            &data::PLANE_INDICES,
            glow::ELEMENT_ARRAY_BUFFER,
            glow::STATIC_DRAW,
        )?;

        let simplex_noise_program =
            Program::new(&context, (NOISE_VERT_SHADER, SIMPLEX_NOISE_FRAG_SHADER))?;
        let inject_noise_program =
            Program::new(&context, (NOISE_VERT_SHADER, INJECT_NOISE_FRAG_SHADER))?;

        let noise_buffer = VertexArrayObject::new(
            &context,
            &simplex_noise_program,
            &[(
                &plane_vertices,
                VertexBufferLayout {
                    name: "position",
                    size: 3,
                    type_: glow::FLOAT,
                    ..Default::default()
                },
            )],
            Some(&plane_indices),
        )?;

        simplex_noise_program.set_uniform_block("NoiseUniforms", 0);

        inject_noise_program.set_uniforms(&[
            &Uniform {
                name: "velocityTexture",
                value: UniformValue::Texture2D(0),
            },
            &Uniform {
                name: "noiseTexture",
                value: UniformValue::Texture2D(1),
            },
        ]);

        Ok(Self {
            context: Rc::clone(context),
            channels: Vec::new(),
            width,
            height,

            generate_noise_pass: simplex_noise_program,
            inject_noise_pass: inject_noise_program,

            noise_buffer,
        })
    }

    pub fn add_noise(&mut self, noise: Noise) -> Result<(), render::Problem> {
        let uniforms = NoiseUniforms {
            scale: noise.scale,
            offset_1: noise.offset_1,
            offset_2: noise.offset_2,
            multiplier: noise.multiplier,
            blend_factor: 0.0,
        };

        let uniforms = Buffer::from_bytes(
            &self.context,
            &uniforms.as_std140().as_bytes(),
            glow::ARRAY_BUFFER,
            glow::STATIC_DRAW,
        )?;

        let texture = Framebuffer::new(
            &self.context,
            self.width,
            self.height,
            TextureOptions {
                mag_filter: glow::LINEAR,
                min_filter: glow::LINEAR,
                format: glow::RG16F,
                ..Default::default()
            },
        )?
        .with_data(None::<&[f16]>)?;

        self.channels.push(NoiseChannel {
            noise: noise.clone(),
            texture,
            blend_begin_time: -noise.delay,
            last_blend_progress: 0.0,
            offset1: noise.offset_1,
            offset2: noise.offset_2,
            uniforms,
        });

        Ok(())
    }

    pub fn generate_all(&mut self, elapsed_time: f32) -> () {
        for channel in self.channels.iter_mut() {
            let time_since_last_update = elapsed_time - channel.blend_begin_time;

            if time_since_last_update >= channel.noise.delay {
                self.generate_noise_pass.use_program();

                unsafe {
                    self.context.bind_vertex_array(Some(self.noise_buffer.id));

                    self.context.bind_buffer_base(
                        glow::UNIFORM_BUFFER,
                        0,
                        Some(channel.uniforms.id),
                    );

                    channel.texture.draw_to(&self.context, || {
                        self.context.clear_color(0.0, 0.0, 0.0, 0.0);
                        self.context.clear(glow::COLOR_BUFFER_BIT);
                        self.context
                            .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
                    });

                    // self.context.disable(glow::BLEND);
                }

                channel.tick(&self.context, elapsed_time);
            }
        }
    }

    pub fn generate_by_channel_number(&mut self, channel_number: usize, elapsed_time: f32) {
        if let Some(channel) = self.channels.get_mut(channel_number) {
            self.generate_noise_pass.use_program();

            unsafe {
                self.context.bind_vertex_array(Some(self.noise_buffer.id));

                self.context
                    .bind_buffer_base(glow::UNIFORM_BUFFER, 0, Some(channel.uniforms.id));

                channel.texture.draw_to(&self.context, || {
                    self.context
                        .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
                });
            }

            channel.tick(&self.context, elapsed_time);
        }
    }

    pub fn blend_noise_into(
        &mut self,
        target_textures: &DoubleFramebuffer,
        elapsed_time: f32,
        timestep: f32,
    ) -> () {
        for channel in self.channels.iter_mut() {
            let blend_progress: f32 = ((elapsed_time - channel.blend_begin_time)
                / channel.noise.blend_duration)
                .clamp(0.0, 1.0);

            if blend_progress >= 1.0 - 0.0001 {
                continue;
            }

            target_textures.draw_to(&self.context, |target_texture| {
                self.inject_noise_pass.use_program();

                unsafe {
                    self.context.disable(glow::BLEND);
                    self.context.bind_vertex_array(Some(self.noise_buffer.id));

                    self.inject_noise_pass.set_uniform(&Uniform {
                        name: "deltaTime",
                        value: UniformValue::Float(timestep),
                    });

                    self.context.active_texture(glow::TEXTURE0);
                    self.context
                        .bind_texture(glow::TEXTURE_2D, Some(target_texture.texture));

                    self.context.active_texture(glow::TEXTURE1);
                    self.context
                        .bind_texture(glow::TEXTURE_2D, Some(channel.texture.texture));

                    self.context
                        .draw_elements(glow::TRIANGLES, 6, glow::UNSIGNED_SHORT, 0);
                }
            });

            channel.last_blend_progress = blend_progress;
        }
    }

    pub fn get_delta_blend_progress(&mut self, elapsed_time: f32) -> f32 {
        let channel = &mut self.channels[0];

        if channel.last_blend_progress >= 1.0 {
            return 0.0;
        }

        let blend_progress: f32 =
            (elapsed_time - channel.blend_begin_time) / channel.noise.blend_duration;

        let delta_blend_progress = (blend_progress - channel.last_blend_progress).clamp(0.0, 1.0);

        channel.last_blend_progress = blend_progress;

        delta_blend_progress
    }

    #[allow(dead_code)]
    pub fn get_noise(&self) -> &Framebuffer {
        let channel = &self.channels[0];
        &channel.texture
    }
}
